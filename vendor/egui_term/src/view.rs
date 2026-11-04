use alacritty_terminal::index::Point as TerminalGridPoint;
use alacritty_terminal::term::cell;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use egui::epaint::RectShape;
use egui::Modifiers;
use egui::MouseWheelUnit;
use egui::Shape;
use egui::Widget;
use egui::{CornerRadius, Key};
use egui::{Id, PointerButton};
use egui::{Painter, Pos2, Rect, Response, Stroke, Vec2};
use std::sync::{Arc, OnceLock};
use unicode_segmentation::UnicodeSegmentation;

use crate::backend::BackendCommand;
use crate::backend::TerminalBackend;
use crate::backend::{LinkAction, MouseButton, SelectionType};
use crate::bindings::Binding;
use crate::bindings::{BindingAction, BindingsLayout, InputKind};
use crate::font::TerminalFont;
use crate::theme::TerminalTheme;
use crate::types::Size;

const EGUI_TERM_WIDGET_ID_PREFIX: &str = "egui_term::instance::";

#[derive(Debug, Clone)]
enum InputAction {
    BackendCall(BackendCommand),
    WriteToClipboard(String),
    Ignore,
}

#[derive(Clone, Default)]
pub struct TerminalViewState {
    preedit: String,
    input_enabled: bool,
    is_dragged: bool,
    local_selection_drag: bool,
    pending_primary: Option<Pos2>,
    scroll_pixels: f32,
    current_mouse_position_on_grid: TerminalGridPoint,
}

pub struct TerminalView<'a> {
    widget_id: Id,
    has_focus: bool,
    focus_requested: bool,
    native_image_paste: bool,
    drag_selection: bool,
    snapshot_only: bool,
    size: Vec2,
    backend: &'a mut TerminalBackend,
    font: TerminalFont,
    theme: TerminalTheme,
    bindings_layout: Arc<BindingsLayout>,
}

impl Widget for TerminalView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (_, rect) = ui.allocate_space(self.size);
        let layout = ui.interact(rect, self.widget_id, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let widget_id = self.widget_id;
        let mut state = ui.memory(|m| {
            m.data
                .get_temp::<TerminalViewState>(widget_id)
                .unwrap_or_default()
        });

        let view = self.focus(&layout, &mut state);
        if view.snapshot_only {
            view.show(&mut state, &layout, &painter);
        } else {
            view.resize(&layout)
                .process_input(&layout, &mut state)
                .show(&mut state, &layout, &painter);
        }

        ui.memory_mut(|m| m.data.insert_temp(widget_id, state));
        layout
    }
}

impl<'a> TerminalView<'a> {
    pub fn new(ui: &mut egui::Ui, backend: &'a mut TerminalBackend) -> Self {
        let widget_id =
            ui.make_persistent_id(format!("{}{}", EGUI_TERM_WIDGET_ID_PREFIX, backend.id()));

        Self {
            widget_id,
            has_focus: false,
            focus_requested: false,
            native_image_paste: false,
            drag_selection: false,
            snapshot_only: false,
            size: ui.available_size(),
            backend,
            font: TerminalFont::default(),
            theme: TerminalTheme::default(),
            bindings_layout: {
                static DEFAULT: OnceLock<Arc<BindingsLayout>> = OnceLock::new();
                DEFAULT
                    .get_or_init(|| Arc::new(BindingsLayout::new()))
                    .clone()
            },
        }
    }

    #[inline]
    pub fn set_theme(mut self, theme: TerminalTheme) -> Self {
        self.theme = theme;
        self
    }

    #[inline]
    pub fn set_font(mut self, font: TerminalFont) -> Self {
        self.font = font;
        self
    }

    #[inline]
    pub fn set_focus(mut self, has_focus: bool) -> Self {
        self.has_focus = has_focus;
        self
    }

    /// Let Windows coding CLIs read clipboard images after a native Ctrl+V key.
    /// The host keeps text paste for other terminals and UI editors.
    #[inline]
    pub fn set_native_image_paste(mut self, enabled: bool) -> Self {
        self.native_image_paste = enabled;
        self
    }

    /// Prefer local text selection for plain left drags in coding CLIs.
    /// A click without a drag still reaches the application on release.
    pub fn set_drag_selection(mut self, enabled: bool) -> Self {
        self.drag_selection = enabled;
        self
    }

    /// Paint the most recent snapshot without waiting for terminal output.
    #[inline]
    pub fn set_snapshot_only(mut self, enabled: bool) -> Self {
        self.snapshot_only = enabled;
        self
    }

    #[inline]
    pub fn request_focus(mut self, requested: bool) -> Self {
        self.focus_requested = requested;
        self
    }

    #[inline]
    pub fn set_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    #[inline]
    pub fn add_bindings(mut self, bindings: Vec<(Binding<InputKind>, BindingAction)>) -> Self {
        Arc::make_mut(&mut self.bindings_layout).add_bindings(bindings);
        self
    }

    fn focus(self, layout: &Response, state: &mut TerminalViewState) -> Self {
        if self.has_focus {
            // request_focus also interrupts native IME. Never repeat it every repaint.
            if !layout.has_focus()
                && (self.focus_requested
                    || !state.input_enabled
                    || layout.clicked()
                    || layout.ctx.memory(|memory| memory.focused().is_none()))
            {
                layout.request_focus();
            }
            if layout.has_focus() {
                layout.ctx.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        layout.id,
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                        },
                    )
                });
            }
        } else {
            layout.surrender_focus();
            state.preedit.clear();
        }
        // IME cancellation can arrive after the native window loses focus,
        // when process_input intentionally skips key events. Clear its local
        // composition on focus transitions so stale preedit cannot swallow
        // every key after the user returns to this terminal.
        if !layout.ctx.input(|input| input.focused)
            || !layout.has_focus()
            || self.focus_requested
            || layout.gained_focus()
        {
            state.preedit.clear();
            state.is_dragged = false;
            state.local_selection_drag = false;
            state.pending_primary = None;
        }
        state.input_enabled = self.has_focus;
        self
    }

    fn resize(self, layout: &Response) -> Self {
        self.backend.process_command(BackendCommand::Resize(
            Size::from(layout.rect.size()),
            self.font.font_measure(&layout.ctx),
            layout.ctx.pixels_per_point(),
        ));

        self
    }

    fn process_input(self, layout: &Response, state: &mut TerminalViewState) -> Self {
        if !layout.has_focus() || !self.has_focus || !layout.ctx.input(|i| i.focused) {
            return self;
        }

        let modifiers = layout.ctx.input(|i| i.modifiers);
        let events = layout.ctx.input(|i| i.events.clone());
        if events.is_empty() {
            return self;
        }
        // Input routing needs current terminal mode and coordinates. Repaints
        // without input take just the single snapshot in `show`.
        self.backend.sync();
        let mut pending_input = Vec::new();
        let ime_composing = !state.preedit.is_empty()
            || events
                .iter()
                .any(|event| matches!(event,
                    egui::Event::Ime(egui::ImeEvent::Preedit { text, .. } | egui::ImeEvent::Commit(text)) if !text.is_empty()));
        for event in events {
            // Mouse, clipboard, and modified-key actions can consult terminal
            // state while being interpreted. Send earlier text first.
            let flush_before = match &event {
                egui::Event::Key { modifiers, .. } => !modifiers.is_none(),
                egui::Event::Copy
                | egui::Event::Paste(_)
                | egui::Event::MouseWheel { .. }
                | egui::Event::PointerButton { .. }
                | egui::Event::PointerMoved(_) => true,
                _ => false,
            };
            if flush_before {
                flush_pending_input(self.backend, &mut pending_input);
            }
            let mut input_actions = vec![];

            match event {
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    state.preedit.clear();
                    input_actions.push(InputAction::BackendCall(BackendCommand::Write(
                        text.into_bytes(),
                    )));
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    state.preedit = text;
                }
                #[allow(deprecated)]
                egui::Event::Ime(egui::ImeEvent::Disabled) => state.preedit.clear(),
                egui::Event::Key { .. } | egui::Event::Text(_) if ime_composing => {}
                egui::Event::Text(_)
                | egui::Event::Key { .. }
                | egui::Event::Copy
                | egui::Event::Paste(_) => input_actions.push(process_keyboard_event(
                    event,
                    self.backend,
                    &self.bindings_layout,
                    modifiers,
                    self.native_image_paste,
                )),
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers: wheel_modifiers,
                    ..
                } if layout.contains_pointer() => input_actions.push(process_mouse_wheel(
                    state,
                    self.font.font_type().size,
                    unit,
                    delta,
                    layout,
                    self.backend,
                    wheel_modifiers,
                )),
                egui::Event::PointerButton {
                    button,
                    pressed,
                    modifiers,
                    pos,
                    ..
                } if layout.rect.contains(pos) || state.is_dragged => {
                    input_actions = process_button_click(
                        state,
                        layout,
                        self.backend,
                        &self.bindings_layout,
                        button,
                        pos,
                        &modifiers,
                        pressed,
                        self.drag_selection,
                    )
                }
                egui::Event::PointerMoved(pos) if layout.rect.contains(pos) || state.is_dragged => {
                    input_actions = process_mouse_move(state, layout, self.backend, pos, &modifiers)
                }
                _ => {}
            };

            for action in input_actions {
                match action {
                    InputAction::BackendCall(BackendCommand::Write(bytes)) => {
                        const MAX_INPUT_BATCH: usize = 64 * 1024;
                        if pending_input.len().saturating_add(bytes.len()) > MAX_INPUT_BATCH {
                            flush_pending_input(self.backend, &mut pending_input);
                        }
                        if bytes.len() > MAX_INPUT_BATCH {
                            self.backend.process_command(BackendCommand::Write(bytes));
                        } else {
                            pending_input.extend(bytes);
                        }
                    }
                    InputAction::BackendCall(cmd) => {
                        flush_pending_input(self.backend, &mut pending_input);
                        self.backend.process_command(cmd);
                    }
                    InputAction::WriteToClipboard(data) => {
                        flush_pending_input(self.backend, &mut pending_input);
                        layout.ctx.copy_text(data);
                    }
                    InputAction::Ignore => {}
                }
            }
        }
        flush_pending_input(self.backend, &mut pending_input);

        self
    }

    fn show(self, state: &mut TerminalViewState, layout: &Response, painter: &Painter) {
        let (_, baseline) = self.font.metrics(&layout.ctx);
        let scale = layout.ctx.pixels_per_point();
        let snap = |value: f32| (value * scale).round() / scale;
        let content = if self.snapshot_only {
            self.backend.last_content()
        } else {
            self.backend.sync()
        };
        let origin = Pos2::new(snap(layout.rect.min.x), snap(layout.rect.min.y));
        let cell_height = content.terminal_size.cell_height;
        let cell_width = content.terminal_size.cell_width;
        let color = |value: Color| {
            let index = match value {
                Color::Named(name) => Some(name as usize),
                Color::Indexed(index) => Some(index as usize),
                Color::Spec(_) => None,
            };
            index
                .and_then(|i| content.colors[i])
                .map(|rgb| egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b))
                .unwrap_or_else(|| self.theme.get_color(value))
        };
        let global_bg = color(Color::Named(NamedColor::Background));
        let cursor_color = content.colors[NamedColor::Cursor]
            .map(|rgb| egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b))
            .unwrap_or_else(|| color(Color::Named(NamedColor::Foreground)));
        let mut backgrounds = vec![Shape::Rect(RectShape::filled(
            layout.rect,
            CornerRadius::ZERO,
            global_bg,
        ))];
        let mut foregrounds = Vec::new();
        let mut cursors = Vec::new();
        let window_focused = layout.ctx.input(|i| i.focused);
        let active = layout.has_focus() && window_focused;
        let cursor_shape = if active || content.cursor_shape == CursorShape::Hidden {
            content.cursor_shape
        } else {
            CursorShape::HollowBlock
        };
        let mut skip_glyph_until = None;
        for (cell_index, indexed) in content.cells.iter().enumerate() {
            let flags = indexed.cell.flags;
            let wide = flags.contains(cell::Flags::WIDE_CHAR);
            let spacer = flags
                .intersects(cell::Flags::WIDE_CHAR_SPACER | cell::Flags::LEADING_WIDE_CHAR_SPACER);
            let x = origin.x + cell_width * indexed.point.column.0 as f32;
            let y = origin.y
                + cell_height * (indexed.point.line.0 + content.display_offset as i32) as f32;
            let width = if wide { cell_width * 2.0 } else { cell_width };
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, cell_height));
            if !rect.intersects(layout.rect) {
                continue;
            }
            let selected = content
                .selectable_range
                .is_some_and(|r| r.contains(indexed.point));
            let hovered = content.hovered_hyperlink.as_ref().is_some_and(|range| {
                range.contains(&indexed.point)
                    && range.contains(&state.current_mouse_position_on_grid)
            });
            let mut fg = color(indexed.cell.fg);
            let mut bg = color(indexed.cell.bg);
            if flags.contains(cell::Flags::DIM) {
                fg = fg.linear_multiply(0.7);
            }
            if flags.contains(cell::Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if selected {
                std::mem::swap(&mut fg, &mut bg);
            }
            // Paint every background before any glyph: adjacent cells must not erase text.
            if global_bg != bg {
                backgrounds.push(Shape::Rect(RectShape::filled(rect, CornerRadius::ZERO, bg)));
            }
            if spacer {
                continue;
            }
            if content.cursor_point == indexed.point {
                match cursor_shape {
                    CursorShape::Block => {
                        backgrounds.push(Shape::Rect(RectShape::filled(
                            rect,
                            CornerRadius::ZERO,
                            cursor_color,
                        )));
                        fg = global_bg;
                        bg = cursor_color;
                    }
                    CursorShape::Beam => cursors.push(Shape::Rect(RectShape::filled(
                        Rect::from_min_size(rect.min, Vec2::new(1.5, cell_height)),
                        CornerRadius::ZERO,
                        cursor_color,
                    ))),
                    CursorShape::Underline => cursors.push(Shape::Rect(RectShape::filled(
                        Rect::from_min_size(Pos2::new(x, rect.max.y - 1.5), Vec2::new(width, 1.5)),
                        CornerRadius::ZERO,
                        cursor_color,
                    ))),
                    CursorShape::HollowBlock => cursors.push(Shape::Rect(RectShape::stroke(
                        rect.shrink(0.5),
                        CornerRadius::ZERO,
                        Stroke::new(1.0, cursor_color),
                        egui::StrokeKind::Inside,
                    ))),
                    CursorShape::Hidden => {}
                }
            }
            if !flags.contains(cell::Flags::HIDDEN)
                && indexed.cell.c != '\t'
                && !skip_glyph_until.is_some_and(|end| cell_index <= end)
            {
                let (text, end, columns) = glyph_cluster(&content.cells, cell_index);
                if end > cell_index {
                    skip_glyph_until = Some(end);
                }
                let glyph_rect = Rect::from_min_size(
                    rect.min,
                    egui::vec2(columns as f32 * cell_width, cell_height),
                );
                if text != " " {
                    let builtin = indexed
                        .cell
                        .zerowidth()
                        .is_none_or(|chars| chars.is_empty())
                        .then(|| grid_glyph(indexed.cell.c, rect, fg))
                        .flatten();
                    if let Some(shapes) = builtin {
                        foregrounds.extend(shapes);
                    } else {
                        #[cfg(windows)]
                        let native = self.font.native.as_ref().and_then(|font| {
                            crate::native_text::shape(
                                &layout.ctx,
                                font,
                                self.font.font_type().size,
                                &text,
                                glyph_rect,
                                baseline,
                                flags.contains(cell::Flags::BOLD),
                                flags.contains(cell::Flags::ITALIC),
                                fg,
                            )
                        });
                        #[cfg(not(windows))]
                        let native: Option<Shape> = None;
                        if let Some(native) = native {
                            foregrounds.push(native);
                        } else {
                            let font = self.font.styled_font_at_scale(
                                flags.contains(cell::Flags::BOLD),
                                flags.contains(cell::Flags::ITALIC),
                                painter.ctx().pixels_per_point(),
                            );
                            let galley =
                                painter.layout_no_wrap(text, font, egui::Color32::PLACEHOLDER);
                            let glyph_baseline = galley
                                .rows
                                .first()
                                .and_then(|row| {
                                    row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y)
                                })
                                .unwrap_or(0.0);
                            let inset = if wide {
                                ((width - galley.size().x) / 2.0).max(0.0)
                            } else {
                                0.0
                            };
                            let pos =
                                Pos2::new(snap(x + inset), snap(y + baseline - glyph_baseline));
                            foregrounds.push(Shape::Text(
                                egui::epaint::TextShape::new(pos, galley, fg)
                                    .with_override_text_color(fg),
                            ));
                        }
                    }
                }
            }

            if hovered || flags.intersects(cell::Flags::ALL_UNDERLINES) {
                let line = rect.max.y - 1.5;
                foregrounds.push(Shape::line_segment(
                    [Pos2::new(x, line), Pos2::new(x + width, line)],
                    Stroke::new(1.0, fg),
                ));
            }
            if flags.contains(cell::Flags::STRIKEOUT) {
                foregrounds.push(Shape::line_segment(
                    [
                        Pos2::new(x, rect.center().y),
                        Pos2::new(x + width, rect.center().y),
                    ],
                    Stroke::new(1.0, fg),
                ));
            }
        }
        painter.extend(backgrounds);
        painter.extend(foregrounds);
        painter.extend(cursors);
        if layout.has_focus() && self.has_focus {
            let cursor_pos = Pos2::new(
                origin.x + content.cursor_point.column.0 as f32 * cell_width,
                origin.y
                    + (content.cursor_point.line.0 + content.display_offset as i32) as f32
                        * cell_height,
            );
            layout.ctx.output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: layout.rect,
                    cursor_rect: Rect::from_min_size(cursor_pos, Vec2::new(1.0, cell_height))
                        .intersect(layout.rect),
                    should_interrupt_composition: false,
                });
            });
            if !state.preedit.is_empty() {
                let galley = painter.layout_no_wrap(
                    state.preedit.clone(),
                    self.font
                        .styled_font_at_scale(false, false, painter.ctx().pixels_per_point()),
                    egui::Color32::WHITE,
                );
                let rect = Rect::from_min_size(
                    cursor_pos,
                    galley.size().max(Vec2::new(cell_width, cell_height)),
                );
                painter.rect_filled(rect, CornerRadius::ZERO, global_bg);
                painter.galley(cursor_pos, galley, egui::Color32::WHITE);
                painter.line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1.0, cursor_color),
                );
            }
        }
    }
}

fn flush_pending_input(backend: &mut TerminalBackend, pending: &mut Vec<u8>) {
    if !pending.is_empty() {
        backend.process_command(BackendCommand::Write(std::mem::take(pending)));
    }
}

/// Preserve emulator coordinates while joining emoji modifiers, flags and ZWJ sequences for shaping.
fn glyph_cluster(cells: &[crate::backend::RenderCell], index: usize) -> (String, usize, usize) {
    let first = &cells[index];
    let mut text = first.cell.c.to_string();
    if let Some(zero) = first.cell.zerowidth() {
        text.extend(zero);
    }
    let mut end = index;
    let first_width = if first.cell.flags.contains(cell::Flags::WIDE_CHAR) {
        2
    } else {
        1
    };
    let mut columns = first_width;
    if !first.cell.c.is_ascii() {
        loop {
            let mut next = end + 1;
            while next < cells.len()
                && cells[next]
                    .cell
                    .flags
                    .contains(cell::Flags::WIDE_CHAR_SPACER)
            {
                next += 1;
            }
            let Some(candidate) = cells.get(next) else {
                break;
            };
            if candidate.point.line != first.point.line
                || candidate.point.column.0 != first.point.column.0 + columns
                || candidate.cell.c == ' '
                || candidate.cell.fg != first.cell.fg
                || candidate.cell.bg != first.cell.bg
                || candidate.cell.flags != first.cell.flags
            {
                break;
            }
            let mut joined = text.clone();
            joined.push(candidate.cell.c);
            if let Some(zero) = candidate.cell.zerowidth() {
                joined.extend(zero);
            }
            if joined.graphemes(true).count() != 1 {
                break;
            }
            text = joined;
            end = next;
            columns += if candidate.cell.flags.contains(cell::Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            if text.len() > 128 {
                break;
            }
        }
    }
    let presentation = emoji_presentation(&text);
    let blank_after = cells.get(index + 1).is_some_and(|c| {
        c.point.line == first.point.line
            && c.point.column.0 == first.point.column.0 + 1
            && c.cell.c == ' '
    });
    // A trailing badge in a narrow symbol run must use the same width as its
    // neighbours. Otherwise only that last glyph expands into the padding and
    // gets a different raster scale (for example Claude's adjacent play icons).
    let previous_badge = presentation
        && first_width == 1
        && blank_after
        && index
            .checked_sub(1)
            .and_then(|i| cells.get(i))
            .is_some_and(|c| {
                if c.point.line != first.point.line
                    || c.point.column.0 + 1 != first.point.column.0
                    || c.cell
                        .flags
                        .intersects(cell::Flags::WIDE_CHAR | cell::Flags::WIDE_CHAR_SPACER)
                {
                    return false;
                }
                let mut previous = c.cell.c.to_string();
                if let Some(zero) = c.cell.zerowidth() {
                    previous.extend(zero);
                }
                emoji_presentation(&previous)
            });
    let visual = if presentation && first_width == 1 && blank_after && !previous_badge {
        2
    } else {
        columns.min(2)
    };
    (text, end, visual)
}

/// Text-default emoji (e.g. Claude's pause/play badges) may occupy a narrow
/// emulator cell followed by padding. Shape them as an emoji without consuming
/// or moving the neighbouring terminal cells. VS15 explicitly requests text.
pub(crate) fn emoji_presentation(text: &str) -> bool {
    !text.contains('\u{fe0e}')
        && (text.contains('\u{fe0f}')
            || text.chars().any(|c| {
                matches!(c as u32,
                0x23e9..=0x23fa | 0x25b6 | 0x25c0 |
                0x2600..=0x2604 | 0x2614..=0x2615 | 0x2648..=0x2653 |
                0x26a0..=0x26a1 | 0x26bd..=0x26be | 0x26c4..=0x26c5 |
                0x26f2..=0x26f3 | 0x26fa | 0x26fd | 0x2705 | 0x2728 |
                0x274c | 0x274e | 0x2753..=0x2755 | 0x2757 | 0x2764 |
                0x2795..=0x2797 | 0x27b0 | 0x27bf | 0x1f000..=0x1faff)
            }))
}

fn process_keyboard_event(
    event: egui::Event,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    modifiers: Modifiers,
    native_image_paste: bool,
) -> InputAction {
    match event {
        egui::Event::Text(text) => process_text_event(&text, modifiers),
        egui::Event::Paste(text) => {
            let bracketed = backend
                .last_content()
                .terminal_mode
                .contains(TermMode::BRACKETED_PASTE);
            let image = native_image_paste && crate::paste::has_image();
            InputAction::BackendCall(BackendCommand::Write(crate::paste::bytes(
                &text, bracketed, image,
            )))
        }
        egui::Event::Copy => {
            #[cfg(not(any(target_os = "ios", target_os = "macos")))]
            {
                let content = backend.selectable_content();
                if modifiers.contains(Modifiers::COMMAND | Modifiers::SHIFT) || !content.is_empty()
                {
                    InputAction::WriteToClipboard(content)
                } else {
                    InputAction::BackendCall(BackendCommand::Write(vec![0x03]))
                }
            }
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            {
                let content = backend.selectable_content();
                InputAction::WriteToClipboard(content)
            }
        }
        egui::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } => process_keyboard_key(backend, bindings_layout, key, modifiers, pressed),
        _ => InputAction::Ignore,
    }
}

fn process_text_event(text: &str, modifiers: Modifiers) -> InputAction {
    // Text events carry literal text, not key names. Ctrl/Command shortcuts are
    // handled by Key/Copy/Paste; Ctrl+Alt can be AltGr producing normal text.
    if (modifiers.ctrl || modifiers.command) && !modifiers.alt {
        InputAction::Ignore
    } else if modifiers.alt && !modifiers.ctrl {
        InputAction::BackendCall(BackendCommand::Write(format!("\x1b{text}").into_bytes()))
    } else {
        InputAction::BackendCall(BackendCommand::Write(text.as_bytes().to_vec()))
    }
}

fn process_keyboard_key(
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    key: Key,
    modifiers: Modifiers,
    pressed: bool,
) -> InputAction {
    if !pressed {
        return InputAction::Ignore;
    }

    let terminal_mode = backend.last_content().terminal_mode;
    let binding_action =
        bindings_layout.get_action(InputKind::KeyCode(key), modifiers, terminal_mode);

    match binding_action {
        BindingAction::Char(c) => {
            let mut buf = [0, 0, 0, 0];
            let str = c.encode_utf8(&mut buf);
            InputAction::BackendCall(BackendCommand::Write(str.as_bytes().to_vec()))
        }
        BindingAction::Esc(seq) => {
            InputAction::BackendCall(BackendCommand::Write(seq.as_bytes().to_vec()))
        }
        _ => InputAction::Ignore,
    }
}

fn process_mouse_wheel(
    state: &mut TerminalViewState,
    font_size: f32,
    unit: MouseWheelUnit,
    delta: Vec2,
    layout: &Response,
    backend: &TerminalBackend,
    modifiers: Modifiers,
) -> InputAction {
    if modifiers.ctrl || modifiers.command {
        return InputAction::Ignore;
    }
    let lines = match unit {
        MouseWheelUnit::Line => delta.y.signum() * delta.y.abs().ceil(),
        MouseWheelUnit::Point => {
            state.scroll_pixels += delta.y;
            let lines = (state.scroll_pixels / font_size).trunc();
            state.scroll_pixels %= font_size;
            lines
        }
        MouseWheelUnit::Page => 0.0,
    } as i32;
    if lines == 0 {
        return InputAction::Ignore;
    }
    // Mouse protocols use screen coordinates, even when local scrollback is visible.
    let position = layout
        .ctx
        .pointer_hover_pos()
        .unwrap_or(layout.rect.center());
    let point = TerminalBackend::selection_point(
        position.x - layout.rect.min.x,
        position.y - layout.rect.min.y,
        &backend.last_content().terminal_size,
        0,
    );
    InputAction::BackendCall(BackendCommand::MouseWheel(lines, modifiers, point))
}

// Route the complete mouse event together with terminal selection policy.
#[allow(clippy::too_many_arguments)]
fn process_button_click(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    button: PointerButton,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
    drag_selection: bool,
) -> Vec<InputAction> {
    if button != PointerButton::Primary {
        return vec![InputAction::Ignore];
    }
    let terminal_mode = backend.last_content().terminal_mode;
    let application_mouse = terminal_mode.intersects(TermMode::MOUSE_MODE) && !modifiers.shift;
    if pressed {
        state.is_dragged = true;
        state.local_selection_drag = !application_mouse;
        state.pending_primary = if application_mouse && drag_selection && modifiers.is_none() {
            Some(position)
        } else {
            None
        };
        if !application_mouse || state.pending_primary.is_some() {
            return vec![process_left_button_pressed(state, layout, position)];
        }
    } else {
        state.is_dragged = false;
        if let Some(start) = state.pending_primary.take() {
            return vec![
                application_left_click(backend, layout, start, *modifiers, true),
                application_left_click(backend, layout, position, *modifiers, false),
            ];
        }
        if state.local_selection_drag {
            state.local_selection_drag = false;
            return vec![process_left_button_released(
                state,
                layout,
                backend,
                bindings_layout,
                position,
                modifiers,
            )];
        }
    }
    if application_mouse {
        vec![application_left_click(
            backend, layout, position, *modifiers, pressed,
        )]
    } else {
        vec![process_left_button_released(
            state,
            layout,
            backend,
            bindings_layout,
            position,
            modifiers,
        )]
    }
}

fn application_left_click(
    backend: &TerminalBackend,
    layout: &Response,
    position: Pos2,
    modifiers: Modifiers,
    pressed: bool,
) -> InputAction {
    let point = TerminalBackend::selection_point(
        position.x - layout.rect.min.x,
        position.y - layout.rect.min.y,
        &backend.last_content().terminal_size,
        0,
    );
    InputAction::BackendCall(BackendCommand::MouseReport(
        MouseButton::LeftButton,
        modifiers,
        point,
        pressed,
    ))
}

fn process_left_button_pressed(
    state: &mut TerminalViewState,
    layout: &Response,
    position: Pos2,
) -> InputAction {
    state.is_dragged = true;
    InputAction::BackendCall(build_start_select_command(layout, position))
}

fn process_left_button_released(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    position: Pos2,
    modifiers: &Modifiers,
) -> InputAction {
    state.is_dragged = false;
    if layout.double_clicked() || layout.triple_clicked() {
        InputAction::BackendCall(build_start_select_command(layout, position))
    } else {
        let terminal_content = backend.last_content();
        let binding_action = bindings_layout.get_action(
            InputKind::Mouse(PointerButton::Primary),
            *modifiers,
            terminal_content.terminal_mode,
        );

        if binding_action == BindingAction::LinkOpen {
            InputAction::BackendCall(BackendCommand::ProcessLink(
                LinkAction::Open,
                state.current_mouse_position_on_grid,
            ))
        } else {
            InputAction::Ignore
        }
    }
}

fn build_start_select_command(layout: &Response, cursor_position: Pos2) -> BackendCommand {
    let selection_type = if layout.double_clicked() {
        SelectionType::Semantic
    } else if layout.triple_clicked() {
        SelectionType::Lines
    } else {
        SelectionType::Simple
    };

    BackendCommand::SelectStart(
        selection_type,
        cursor_position.x - layout.rect.min.x,
        cursor_position.y - layout.rect.min.y,
    )
}

fn process_mouse_move(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    position: Pos2,
    modifiers: &Modifiers,
) -> Vec<InputAction> {
    let terminal_content = backend.last_content();
    let cursor_x = position.x - layout.rect.min.x;
    let cursor_y = position.y - layout.rect.min.y;
    state.current_mouse_position_on_grid = TerminalBackend::selection_point(
        cursor_x,
        cursor_y,
        &terminal_content.terminal_size,
        terminal_content.display_offset,
    );

    let mut actions = vec![];
    if let Some(start) = state.pending_primary {
        if position.distance(start) < 3.0 {
            return actions;
        }
        state.pending_primary = None;
        state.local_selection_drag = true;
        actions.push(InputAction::BackendCall(build_start_select_command(
            layout, start,
        )));
    }
    // Handle command or selection update based on terminal mode and modifiers
    if state.is_dragged {
        let terminal_mode = terminal_content.terminal_mode;
        let cmd = if state.local_selection_drag {
            InputAction::BackendCall(BackendCommand::SelectUpdate(cursor_x, cursor_y))
        } else if terminal_mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
            && modifiers.is_none()
        {
            InputAction::BackendCall(BackendCommand::MouseReport(
                MouseButton::LeftMove,
                *modifiers,
                TerminalBackend::selection_point(
                    cursor_x,
                    cursor_y,
                    &terminal_content.terminal_size,
                    0,
                ),
                true,
            ))
        } else {
            InputAction::Ignore
        };

        actions.push(cmd);
    }

    // Handle link hover if applicable
    if modifiers.command_only() {
        actions.push(InputAction::BackendCall(BackendCommand::ProcessLink(
            LinkAction::Hover,
            state.current_mouse_position_on_grid,
        )));
    }

    actions
}

// Terminal box rules and block art must meet the cell edges. Font fallback and
// glyph advances otherwise leave seams in TUI frames and Claude's block mascot.
fn grid_glyph(ch: char, rect: Rect, color: egui::Color32) -> Option<Vec<Shape>> {
    let mut shapes = Vec::new();
    let connections = match ch {
        '─' => Some((true, true, false, false)),
        '│' => Some((false, false, true, true)),
        '┌' | '╭' => Some((false, true, false, true)),
        '┐' | '╮' => Some((true, false, false, true)),
        '└' | '╰' => Some((false, true, true, false)),
        '┘' | '╯' => Some((true, false, true, false)),
        '├' => Some((false, true, true, true)),
        '┤' => Some((true, false, true, true)),
        '┬' => Some((true, true, false, true)),
        '┴' => Some((true, true, true, false)),
        '┼' => Some((true, true, true, true)),
        _ => None,
    };
    if let Some((left, right, up, down)) = connections {
        let middle = rect.center();
        for (enabled, end) in [
            (left, rect.left_center()),
            (right, rect.right_center()),
            (up, rect.center_top()),
            (down, rect.center_bottom()),
        ] {
            if enabled {
                shapes.push(Shape::line_segment([middle, end], Stroke::new(1.0, color)));
            }
        }
        return Some(shapes);
    }
    let fraction = |x0: f32, y0: f32, x1: f32, y1: f32| {
        Shape::Rect(RectShape::filled(
            Rect::from_min_max(
                egui::pos2(
                    rect.left() + x0 * rect.width(),
                    rect.top() + y0 * rect.height(),
                ),
                egui::pos2(
                    rect.left() + x1 * rect.width(),
                    rect.top() + y1 * rect.height(),
                ),
            ),
            CornerRadius::ZERO,
            color,
        ))
    };
    let code = ch as u32;
    match ch {
        '▀' => shapes.push(fraction(0.0, 0.0, 1.0, 0.5)),
        '▁'..='█' => shapes.push(fraction(0.0, 1.0 - (code - 0x2580) as f32 / 8.0, 1.0, 1.0)),
        '▉'..='▏' => shapes.push(fraction(0.0, 0.0, (0x2590 - code) as f32 / 8.0, 1.0)),
        '▐' => shapes.push(fraction(0.5, 0.0, 1.0, 1.0)),
        '▔' => shapes.push(fraction(0.0, 0.0, 1.0, 0.125)),
        '▕' => shapes.push(fraction(0.875, 0.0, 1.0, 1.0)),
        '▖'..='▟' => {
            let mask = match ch {
                '▖' => 4,
                '▗' => 8,
                '▘' => 1,
                '▙' => 13,
                '▚' => 9,
                '▛' => 7,
                '▜' => 11,
                '▝' => 2,
                '▞' => 6,
                '▟' => 14,
                _ => 0,
            };
            for (bit, x, y) in [(1, 0.0, 0.0), (2, 0.5, 0.0), (4, 0.0, 0.5), (8, 0.5, 0.5)] {
                if mask & bit != 0 {
                    shapes.push(fraction(x, y, x + 0.5, y + 0.5));
                }
            }
        }
        _ => return None,
    }
    Some(shapes)
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    #[test]
    fn adjacent_narrow_mode_badges_have_equal_visual_cells() {
        use crate::backend::RenderCell;
        use alacritty_terminal::index::{Column, Line};
        for badge in ['\u{23f5}', '\u{25b6}', '\u{23f8}'] {
            let cells: Vec<_> = [badge, badge, ' ', 'a']
                .into_iter()
                .enumerate()
                .map(|(column, c)| RenderCell {
                    point: TerminalGridPoint::new(Line(0), Column(column)),
                    cell: cell::Cell {
                        c,
                        ..Default::default()
                    },
                })
                .collect();
            let first = glyph_cluster(&cells, 0);
            let second = glyph_cluster(&cells, 1);
            assert_eq!(
                first.2, second.2,
                "adjacent badge {badge} changed size before padding"
            );
            assert_eq!(
                (first.1, second.1),
                (0, 1),
                "badges must remain separate cells"
            );
            assert_eq!(first.0, second.0);
            assert_eq!(
                glyph_cluster(&cells[1..], 0).2,
                2,
                "an isolated mode badge may use its following blank"
            );
        }
    }
    #[test]
    fn terminal_rules_and_blocks_fill_their_cells() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(9.0, 24.0));
        let line = grid_glyph('─', rect, egui::Color32::WHITE).unwrap();
        assert!(line.iter().any(
            |s| matches!(s,Shape::LineSegment{points,..} if points.contains(&rect.left_center()))
        ));
        assert!(line.iter().any(
            |s| matches!(s,Shape::LineSegment{points,..} if points.contains(&rect.right_center()))
        ));
        let block = grid_glyph('█', rect, egui::Color32::WHITE).unwrap();
        assert!(matches!(&block[0],Shape::Rect(shape) if shape.rect==rect));
        assert!(grid_glyph('你', rect, egui::Color32::WHITE).is_none());
    }
}
