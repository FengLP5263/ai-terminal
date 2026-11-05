mod notification;
pub use notification::{CodexProgramStatus, ProgramApp, ProgramEvent};
pub mod settings;

use crate::types::Size;
use alacritty_terminal::event::{Event, EventListener, Notify, OnResize, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, Msg, Notifier};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{
    Selection, SelectionRange, SelectionType as AlacrittySelectionType,
};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::search::{Match, RegexIter, RegexSearch};
use alacritty_terminal::term::{
    self, cell::Cell, color::Colors, test::TermSize, viewport_to_point, Term, TermMode,
};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::CursorShape;
use egui::Modifiers;
use settings::BackendSettings;
use std::borrow::Cow;
use std::cmp::min;
use std::io::Result;
use std::ops::{Index, RangeInclusive};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};

pub type TerminalMode = TermMode;
pub type PtyEvent = Event;
pub const CODEX_ATTENTION_EVENT: &str = "ai-terminal-codex-attention-v1";
pub const CODEX_PROGRAM_EVENT: &str = "ai-terminal-codex-program-v1";
pub type SelectionType = AlacrittySelectionType;

#[derive(Debug, Clone)]
pub enum BackendCommand {
    Write(Vec<u8>),
    Scroll(i32),
    MouseWheel(i32, Modifiers, Point),
    Resize(Size, Size, f32),
    SelectStart(SelectionType, f32, f32),
    SelectUpdate(f32, f32),
    ProcessLink(LinkAction, Point),
    MouseReport(MouseButton, Modifiers, Point, bool),
}

#[derive(Debug, Clone)]
pub enum MouseMode {
    Sgr,
    Normal(bool),
}

impl From<TermMode> for MouseMode {
    fn from(term_mode: TermMode) -> Self {
        if term_mode.contains(TermMode::SGR_MOUSE) {
            MouseMode::Sgr
        } else if term_mode.contains(TermMode::UTF8_MOUSE) {
            MouseMode::Normal(true)
        } else {
            MouseMode::Normal(false)
        }
    }
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    LeftButton = 0,
    MiddleButton = 1,
    RightButton = 2,
    LeftMove = 32,
    MiddleMove = 33,
    RightMove = 34,
    NoneMove = 35,
    ScrollUp = 64,
    ScrollDown = 65,
    Other = 99,
}

#[derive(Debug, Clone)]
pub enum LinkAction {
    Clear,
    Hover,
    Open,
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub cell_width: f32,
    pub cell_height: f32,
    pub pixels_per_point: f32,
    num_cols: u16,
    num_lines: u16,
    layout_size: Size,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            cell_width: 1.0,
            cell_height: 1.0,
            pixels_per_point: 1.0,
            num_cols: 80,
            num_lines: 50,
            layout_size: Size::default(),
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.num_lines as usize
    }

    fn columns(&self) -> usize {
        self.num_cols as usize
    }

    fn last_column(&self) -> Column {
        Column(self.num_cols as usize - 1)
    }

    fn bottommost_line(&self) -> Line {
        Line(self.num_lines as i32 - 1)
    }
}

impl From<TerminalSize> for WindowSize {
    fn from(size: TerminalSize) -> Self {
        Self {
            num_lines: size.num_lines,
            num_cols: size.num_cols,
            cell_width: (size.cell_width * size.pixels_per_point).round().max(1.0) as u16,
            cell_height: (size.cell_height * size.pixels_per_point).round().max(1.0) as u16,
        }
    }
}

pub struct TerminalBackend {
    id: u64,
    pty_id: u32,
    url_regex: RegexSearch,
    term: Arc<FairMutex<Term<EventProxy>>>,
    size: TerminalSize,
    notifier: Notifier,
    last_content: RenderableContent,
    visible: Arc<AtomicBool>,
    codex_notifications: Arc<AtomicBool>,
    codex_notice_epoch: Arc<AtomicU64>,
    codex_notice_pending: Arc<AtomicU8>,
    codex_program: Arc<std::sync::Mutex<notification::ProgramQueue>>,
    shutdown: mpsc::Sender<Event>,
}

impl TerminalBackend {
    pub fn new(
        id: u64,
        app_context: egui::Context,
        pty_event_proxy_sender: Sender<(u64, PtyEvent)>,
        settings: BackendSettings,
    ) -> Result<Self> {
        let pty_config = tty::Options {
            shell: Some(tty::Shell::new(settings.shell, settings.args)),
            working_directory: settings.working_directory,
            env: settings.env,
            #[cfg(windows)]
            escape_args: true,
            ..tty::Options::default()
        };
        let config = term::Config {
            scrolling_history: 5000,
            ..Default::default()
        };
        let terminal_size = TerminalSize::default();
        let pty = tty::new(&pty_config, terminal_size.into(), id)?;
        #[cfg(not(windows))]
        let pty_id = pty.child().id();
        #[cfg(windows)]
        let pty_id = pty
            .child_watcher()
            .pid()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Failed to get child process ID",
            ))?
            .into();
        let (event_sender, event_receiver) = mpsc::channel();
        let shutdown = event_sender.clone();
        let wake_pending = Arc::new(AtomicBool::new(false));
        let event_proxy = EventProxy {
            sender: event_sender,
            wake_pending: wake_pending.clone(),
        };
        let term = Term::new(config, &terminal_size, event_proxy.clone());
        let initial_content = RenderableContent {
            cells: vec![],
            display_offset: 0,
            cursor_point: term.grid().cursor.point,
            selectable_range: None,
            terminal_mode: *term.mode(),
            terminal_size,
            cursor_shape: term.renderable_content().cursor.shape,
            colors: *term.renderable_content().colors,
            hovered_hyperlink: None,
        };
        let term = Arc::new(FairMutex::new(term));
        let codex_notifications = Arc::new(AtomicBool::new(false));
        let codex_notice_epoch = Arc::new(AtomicU64::new(0));
        let codex_notice_pending = Arc::new(AtomicU8::new(0));
        let codex_program = Arc::new(std::sync::Mutex::new(notification::ProgramQueue::default()));
        let pty = notification::ObservedPty::new(
            pty,
            id,
            notification::Control {
                enabled: codex_notifications.clone(),
                epoch: codex_notice_epoch.clone(),
                pending: codex_notice_pending.clone(),
                program: codex_program.clone(),
            },
            pty_event_proxy_sender.clone(),
            app_context.clone(),
        );
        let pty_event_loop = EventLoop::new(term.clone(), event_proxy, pty, false, false)?;
        let notifier = Notifier(pty_event_loop.channel());
        let pty_notifier = Notifier(pty_event_loop.channel());
        let url_regex = RegexSearch::new(r#"(ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\u{0000}-\u{001F}\u{007F}-\u{009F}<>"\s{-}\^⟨⟩`]+"#).unwrap();
        let _pty_event_loop_thread = pty_event_loop.spawn();
        let visible = Arc::new(AtomicBool::new(true));
        let event_visible = visible.clone();
        let _pty_event_subscription = std::thread::Builder::new()
            .name(format!("pty_events_{id}"))
            .spawn(move || {
                while let Ok(event) = event_receiver.recv() {
                    match event {
                        Event::Wakeup => {
                            wake_pending.store(false, Ordering::Release);
                            // Output is parsed even for hidden sessions. Only paint the active one.
                            if event_visible.load(Ordering::Relaxed) {
                                app_context
                                    .request_repaint_after(std::time::Duration::from_millis(16));
                            }
                        }
                        Event::PtyWrite(bytes) => pty_notifier.notify(bytes.into_bytes()),
                        Event::ColorRequest(index, formatter) => {
                            use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
                            let key = match index {
                                256 => Color::Named(NamedColor::Foreground),
                                257 => Color::Named(NamedColor::Background),
                                _ => Color::Indexed(index.min(255) as u8),
                            };
                            let color = crate::TerminalTheme::default().get_color(key);
                            pty_notifier.notify(
                                formatter(Rgb {
                                    r: color.r(),
                                    g: color.g(),
                                    b: color.b(),
                                })
                                .into_bytes(),
                            );
                        }
                        Event::Title(title) if title.starts_with("ai-terminal-cwd;") => {
                            let _ = pty_event_proxy_sender.send((id, Event::Title(title)));
                            app_context.request_repaint_of(egui::ViewportId::ROOT);
                        }
                        Event::Exit => {
                            let _ = pty_event_proxy_sender.send((id, Event::Exit));
                            app_context.request_repaint_of(egui::ViewportId::ROOT);
                            break;
                        }
                        Event::ChildExit(_) => {
                            if pty_event_proxy_sender.send((id, event)).is_err() {
                                break;
                            }
                            app_context.request_repaint_of(egui::ViewportId::ROOT);
                        }
                        _ => {}
                    }
                }
            })?;

        Ok(Self {
            id,
            pty_id,
            url_regex,
            term: term.clone(),
            size: terminal_size,
            notifier,
            last_content: initial_content,
            visible,
            codex_notifications,
            codex_notice_epoch,
            codex_notice_pending,
            codex_program,
            shutdown,
        })
    }

    /// Observe Codex's scoped OSC 9 attention notifications; never record their contents.
    pub fn set_codex_notifications(&self, enabled: bool) {
        self.codex_notifications.store(false, Ordering::Release);
        let epoch = self.codex_notice_epoch.fetch_add(1, Ordering::AcqRel) + 1;
        self.codex_notice_pending.store(0, Ordering::Release);
        self.codex_program.lock().unwrap().reset(epoch);
        self.codex_notifications.store(enabled, Ordering::Release);
    }

    /// Drain a bounded, ordered lifecycle queue for this invocation only.
    pub fn take_program_events(&self, notice: &str) -> Vec<ProgramEvent> {
        let epoch = notice
            .strip_prefix(CODEX_PROGRAM_EVENT)
            .and_then(|s| s.strip_prefix(';'))
            .and_then(|s| s.parse::<u64>().ok());
        if !self.codex_notifications.load(Ordering::Acquire)
            || epoch != Some(self.codex_notice_epoch.load(Ordering::Acquire))
        {
            return Vec::new();
        }
        self.codex_program.lock().unwrap().drain()
    }

    /// Compatibility accessor for callers observing Codex only.
    pub fn take_codex_program_statuses(&self, notice: &str) -> Vec<CodexProgramStatus> {
        self.take_program_events(notice)
            .into_iter()
            .filter(|e| e.app != Some(ProgramApp::Kimi))
            .map(|e| e.state)
            .collect()
    }

    /// Accept each notice once, only for the currently running Codex invocation.
    /// Returns whether the notice is a plan/question rather than an approval.
    pub fn take_codex_attention(&self, notice: &str) -> Option<bool> {
        let (epoch, kind) = notice
            .strip_prefix(CODEX_ATTENTION_EVENT)?
            .strip_prefix(';')?
            .split_once(';')?;
        let epoch = epoch.parse::<u64>().ok()?;
        let (bit, question) = match kind {
            "approval" => (1, false),
            "question" => (2, true),
            _ => return None,
        };
        if !self.codex_notifications.load(Ordering::Acquire)
            || self.codex_notice_epoch.load(Ordering::Acquire) != epoch
        {
            return None;
        }
        (self.codex_notice_pending.fetch_and(!bit, Ordering::AcqRel) & bit != 0).then_some(question)
    }

    pub fn process_command(&mut self, cmd: BackendCommand) {
        // Queue user input before waiting for the emulator grid, which can be
        // busy parsing output from the child process.
        if let BackendCommand::Write(input) = cmd {
            self.write(input);
            self.term.lock().scroll_display(Scroll::Bottom);
            return;
        }
        // A view is laid out on every frame, but the grid rarely changes size.
        if let BackendCommand::Resize(layout, font, scale) = &cmd {
            if *layout == self.size.layout_size
                && font.width == self.size.cell_width
                && font.height == self.size.cell_height
                && *scale == self.size.pixels_per_point
            {
                return;
            }
        }
        let term = self.term.clone();
        let mut term = term.lock();
        match cmd {
            BackendCommand::Write(_) => unreachable!("input is handled before the grid lock"),
            BackendCommand::Scroll(delta) => {
                self.scroll(&mut term, delta);
            }
            BackendCommand::MouseWheel(delta, modifiers, point) => {
                if modifiers.ctrl || modifiers.command {
                    return;
                }
                if !modifiers.shift && term.mode().intersects(TermMode::MOUSE_MODE) {
                    let button = if delta > 0 {
                        MouseButton::ScrollUp
                    } else {
                        MouseButton::ScrollDown
                    };
                    for _ in 0..delta.unsigned_abs().min(32) {
                        self.process_mouse_report(
                            *term.mode(),
                            button.clone(),
                            modifiers,
                            point,
                            true,
                        );
                    }
                } else {
                    self.scroll(&mut term, delta);
                }
            }
            BackendCommand::Resize(layout_size, font_size, pixels_per_point) => {
                self.resize(&mut term, layout_size, font_size, pixels_per_point);
            }
            BackendCommand::SelectStart(selection_type, x, y) => {
                self.start_selection(&mut term, selection_type, x, y);
            }
            BackendCommand::SelectUpdate(x, y) => {
                self.update_selection(&mut term, x, y);
            }
            BackendCommand::ProcessLink(link_action, point) => {
                self.process_link_action(&term, link_action, point);
            }
            BackendCommand::MouseReport(button, modifiers, point, pressed) => {
                self.process_mouse_report(*term.mode(), button, modifiers, point, pressed);
            }
        };
    }

    pub fn selection_point(
        x: f32,
        y: f32,
        terminal_size: &TerminalSize,
        display_offset: usize,
    ) -> Point {
        let col = (x.max(0.0) / terminal_size.cell_width).floor() as usize;
        let col = min(Column(col), Column(terminal_size.num_cols as usize - 1));

        let line = (y.max(0.0) / terminal_size.cell_height).floor() as usize;
        let line = min(line, terminal_size.num_lines as usize - 1);

        viewport_to_point(display_offset, Point::new(line, col))
    }

    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Relaxed);
    }
    pub fn plain_text(&mut self) -> String {
        let content = self.sync();
        let mut text = String::new();
        let mut line = None;
        for indexed in &content.cells {
            if line.is_some_and(|last| last != indexed.point.line) {
                text.push('\n');
            }
            line = Some(indexed.point.line);
            if !indexed
                .cell
                .flags
                .contains(term::cell::Flags::WIDE_CHAR_SPACER)
            {
                text.push(indexed.cell.c);
            }
        }
        text
    }
    pub fn selectable_content(&self) -> String {
        let terminal = self.term.lock();
        terminal
            .selection
            .as_ref()
            .and_then(|s| s.to_range(&terminal))
            .map(|range| terminal.bounds_to_string(range.start, range.end))
            .unwrap_or_default()
    }
    pub fn sync(&mut self) -> &RenderableContent {
        let term = self.term.clone();
        let terminal = term.lock();
        let content = terminal.renderable_content();
        self.last_content.cells.clear();
        self.last_content
            .cells
            .extend(content.display_iter.map(|cell| RenderCell {
                point: cell.point,
                cell: cell.cell.clone(),
            }));
        self.last_content.display_offset = content.display_offset;
        self.last_content.cursor_point = content.cursor.point;
        self.last_content.cursor_shape = content.cursor.shape;
        self.last_content.selectable_range = content.selection;
        self.last_content.colors = *content.colors;
        self.last_content.terminal_mode = content.mode;
        self.last_content.terminal_size = self.size;
        self.last_content()
    }

    pub fn last_content(&self) -> &RenderableContent {
        &self.last_content
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn pty_id(&self) -> u32 {
        self.pty_id
    }

    fn process_link_action(
        &mut self,
        terminal: &Term<EventProxy>,
        link_action: LinkAction,
        point: Point,
    ) {
        match link_action {
            LinkAction::Hover => {
                self.last_content.hovered_hyperlink =
                    self.regex_match_at(terminal, point, &mut self.url_regex.clone());
            }
            LinkAction::Clear => {
                self.last_content.hovered_hyperlink = None;
            }
            LinkAction::Open => {
                self.open_link();
            }
        };
    }

    fn open_link(&self) {
        if let Some(range) = &self.last_content.hovered_hyperlink {
            let start = range.start();
            let end = range.end();

            let terminal = self.term.lock();
            let mut url = String::from(terminal.grid().index(*start).c);
            for indexed in terminal.grid().iter_from(*start) {
                url.push(indexed.c);
                if indexed.point == *end {
                    break;
                }
            }

            let _ = open::that(url);
        }
    }

    fn process_mouse_report(
        &self,
        terminal_mode: TermMode,
        button: MouseButton,
        modifiers: Modifiers,
        point: Point,
        pressed: bool,
    ) {
        let mut mods = 0;
        if modifiers.contains(Modifiers::SHIFT) {
            mods += 4;
        }
        if modifiers.contains(Modifiers::ALT) {
            mods += 8;
        }
        if modifiers.contains(Modifiers::COMMAND) {
            mods += 16;
        }

        match MouseMode::from(terminal_mode) {
            MouseMode::Sgr => self.sgr_mouse_report(point, button as u8 + mods, pressed),
            MouseMode::Normal(is_utf8) => {
                if pressed {
                    self.normal_mouse_report(point, button as u8 + mods, is_utf8)
                } else {
                    self.normal_mouse_report(point, 3 + mods, is_utf8)
                }
            }
        }
    }

    fn sgr_mouse_report(&self, point: Point, button: u8, pressed: bool) {
        let c = if pressed { 'M' } else { 'm' };

        let msg = format!(
            "\x1b[<{};{};{}{}",
            button,
            point.column + 1,
            point.line + 1,
            c
        );

        self.notifier.notify(msg.as_bytes().to_vec());
    }

    fn normal_mouse_report(&self, point: Point, button: u8, is_utf8: bool) {
        let Point { line, column } = point;
        let max_point = if is_utf8 { 2015 } else { 223 };

        if line >= max_point || column >= max_point {
            return;
        }

        let mut msg = vec![b'\x1b', b'[', b'M', 32 + button];

        let mouse_pos_encode = |pos: usize| -> Vec<u8> {
            let pos = 32 + 1 + pos;
            let first = 0xC0 + pos / 64;
            let second = 0x80 + (pos & 63);
            vec![first as u8, second as u8]
        };

        if is_utf8 && column >= Column(95) {
            msg.append(&mut mouse_pos_encode(column.0));
        } else {
            msg.push(32 + 1 + column.0 as u8);
        }

        if is_utf8 && line >= 95 {
            msg.append(&mut mouse_pos_encode(line.0 as usize));
        } else {
            msg.push(32 + 1 + line.0 as u8);
        }

        self.notifier.notify(msg);
    }

    fn start_selection(
        &mut self,
        terminal: &mut Term<EventProxy>,
        selection_type: SelectionType,
        x: f32,
        y: f32,
    ) {
        let location = Self::selection_point(x, y, &self.size, terminal.grid().display_offset());
        terminal.selection = Some(Selection::new(
            selection_type,
            location,
            self.selection_side(x),
        ));
    }

    fn update_selection(&mut self, terminal: &mut Term<EventProxy>, x: f32, y: f32) {
        let display_offset = terminal.grid().display_offset();
        if let Some(ref mut selection) = terminal.selection {
            let location = Self::selection_point(x, y, &self.size, display_offset);
            selection.update(location, self.selection_side(x));
        }
    }

    fn selection_side(&self, x: f32) -> Side {
        let cell_x = x.max(0.0) % self.size.cell_width;
        let half_cell_width = self.size.cell_width / 2.0;

        if cell_x > half_cell_width {
            Side::Right
        } else {
            Side::Left
        }
    }

    fn resize(
        &mut self,
        terminal: &mut Term<EventProxy>,
        layout_size: Size,
        font_size: Size,
        pixels_per_point: f32,
    ) {
        if layout_size == self.size.layout_size
            && font_size.width == self.size.cell_width
            && font_size.height == self.size.cell_height
            && pixels_per_point == self.size.pixels_per_point
        {
            return;
        }

        let lines = (layout_size.height / font_size.height.max(1.0)) as u16;
        let cols = (layout_size.width / font_size.width.max(1.0)) as u16;
        if lines > 0 && cols > 0 {
            let grid_changed = cols != self.size.num_cols || lines != self.size.num_lines;
            let cell_changed = font_size.width != self.size.cell_width
                || font_size.height != self.size.cell_height
                || pixels_per_point != self.size.pixels_per_point;
            self.size = TerminalSize {
                layout_size,
                cell_height: font_size.height,
                pixels_per_point,
                cell_width: font_size.width,
                num_lines: lines,
                num_cols: cols,
            };

            if grid_changed || cell_changed {
                self.notifier.on_resize(self.size.into());
            }
            if grid_changed {
                terminal.resize(TermSize::new(cols as usize, lines as usize));
            }
        }
    }

    fn write<I: Into<Cow<'static, [u8]>>>(&self, input: I) {
        self.notifier.notify(input);
    }

    fn scroll(&mut self, terminal: &mut Term<EventProxy>, delta_value: i32) {
        if delta_value != 0 {
            terminal.scroll_display(Scroll::Delta(delta_value));
        }
    }

    /// Based on alacritty/src/display/hint.rs > regex_match_at
    /// Retrieve the match, if the specified point is inside the content matching the regex.
    fn regex_match_at(
        &self,
        terminal: &Term<EventProxy>,
        point: Point,
        regex: &mut RegexSearch,
    ) -> Option<Match> {
        let x = visible_regex_match_iter(terminal, regex).find(|rm| rm.contains(&point));
        x
    }
}

/// Copied from alacritty/src/display/hint.rs:
/// Iterate over all visible regex matches.
fn visible_regex_match_iter<'a>(
    term: &'a Term<EventProxy>,
    regex: &'a mut RegexSearch,
) -> impl Iterator<Item = Match> + 'a {
    let viewport_start = Line(-(term.grid().display_offset() as i32));
    let viewport_end = viewport_start + term.bottommost_line();
    let mut start = term.line_search_left(Point::new(viewport_start, Column(0)));
    let mut end = term.line_search_right(Point::new(viewport_end, Column(0)));
    start.line = start.line.max(viewport_start - 100);
    end.line = end.line.min(viewport_end + 100);

    RegexIter::new(start, end, Direction::Right, term, regex)
        .skip_while(move |rm| rm.end().line < viewport_start)
        .take_while(move |rm| rm.start().line <= viewport_end)
}

pub struct RenderCell {
    pub point: Point,
    pub cell: Cell,
}

pub struct RenderableContent {
    pub cells: Vec<RenderCell>,
    pub display_offset: usize,
    pub cursor_point: Point,
    pub hovered_hyperlink: Option<RangeInclusive<Point>>,
    pub selectable_range: Option<SelectionRange>,
    pub cursor_shape: CursorShape,
    pub colors: Colors,
    pub terminal_mode: TermMode,
    pub terminal_size: TerminalSize,
}

impl Default for RenderableContent {
    fn default() -> Self {
        Self {
            cells: vec![],
            display_offset: 0,
            cursor_point: Point::new(Line(0), Column(0)),
            hovered_hyperlink: None,
            selectable_range: None,
            cursor_shape: CursorShape::Hidden,
            colors: Colors::default(),
            terminal_mode: TermMode::empty(),
            terminal_size: TerminalSize::default(),
        }
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        let _ = self.notifier.0.send(Msg::Shutdown);
        let _ = self.shutdown.send(Event::Exit);
    }
}

#[derive(Clone)]
pub struct EventProxy {
    sender: mpsc::Sender<Event>,
    wake_pending: Arc<AtomicBool>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        if matches!(event, Event::Wakeup) && self.wake_pending.swap(true, Ordering::AcqRel) {
            return;
        }
        if self.sender.send(event).is_err() {
            self.wake_pending.store(false, Ordering::Release);
        }
    }
}

#[cfg(all(test, windows))]
mod confirmation_snapshot_tests {
    use super::*;
    use crate::TerminalView;
    use std::time::Duration;

    #[test]
    fn confirmation_snapshot_does_not_wait_for_a_busy_terminal_grid() {
        let ctx = egui::Context::default();
        let (tx, _rx) = mpsc::channel();
        let mut backend = TerminalBackend::new(
            91_273,
            ctx.clone(),
            tx,
            BackendSettings {
                shell: "powershell.exe".into(),
                args: vec![
                    "-NoLogo".into(),
                    "-NoProfile".into(),
                    "-Command".into(),
                    "Start-Sleep -Seconds 3".into(),
                ],
                ..Default::default()
            },
        )
        .unwrap();
        backend.sync();
        let term = backend.term.clone();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let holder = std::thread::spawn(move || {
            let _guard = term.lock();
            locked_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
        });
        locked_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let render = std::thread::spawn(move || {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 500.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |ui| {
                let view = TerminalView::new(ui, &mut backend).set_snapshot_only(true);
                ui.add(view);
            });
            let _ = done_tx.send(());
        });
        let completed = done_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        let _ = release_tx.send(());
        holder.join().unwrap();
        render.join().unwrap();
        assert!(
            completed,
            "exit confirmation waited for the locked terminal grid"
        );
    }
}
