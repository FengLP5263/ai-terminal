use ai_terminal::{
    bridge::Bridge,
    i18n::{self, tr},
    launch,
    model::{self, Agent, SessionSpec, Workspace},
    status::{Activity, HookEvent, Status},
};
use egui::{Color32, RichText};
use egui_term::{
    BackendCommand, PtyEvent, TerminalBackend, TerminalFont, TerminalTheme, TerminalView,
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

const BG: Color32 = ai_terminal::appearance::TERMINAL_BG;
const MUTED: Color32 = ai_terminal::chrome::SECONDARY;
const YELLOW: Color32 = Color32::from_rgb(235, 184, 82);

struct Session {
    terminal_id: u64,
    spec: SessionSpec,
    status: Status,
    backend: Option<TerminalBackend>,
    job: Option<crate::platform::ProcessJob>,
    generation: String,
    error: Option<String>,
    last_event: Option<Instant>,
    seen: VecDeque<String>,
    detected_agent: Option<Agent>,
    invocation: Option<String>,
    agent_process: Option<crate::platform::ProcessWatch>,
}
impl Session {
    fn new(spec: SessionSpec) -> Self {
        Self {
            terminal_id: 0,
            spec,
            status: Status::default(),
            backend: None,
            job: None,
            generation: String::new(),
            error: None,
            last_event: None,
            seen: VecDeque::new(),
            detected_agent: None,
            invocation: None,
            agent_process: None,
        }
    }
    fn display_agent(&self) -> Agent {
        self.detected_agent.unwrap_or(self.spec.agent)
    }
    fn agent_ended(&mut self) {
        self.detected_agent = None;
        self.invocation = None;
        self.agent_process.take();
        let unread = self.status.unread;
        let failed = self.status.activity == Activity::Failed;
        self.status = Status::default();
        self.status.activity = if failed {
            Activity::Failed
        } else {
            Activity::Idle
        };
        self.status.unread = unread;
    }
    fn event(&mut self, event: &HookEvent, viewing: bool) -> bool {
        if self.spec.agent.is_shell() {
            if event.name == "ShellAgentStarted" {
                let agent = match event.source.as_deref() {
                    Some("codex") => Agent::Codex,
                    Some("claude") => Agent::Claude,
                    _ => return false,
                };
                let Some(invocation) = event.invocation.clone() else {
                    return false;
                };
                let Some(process) = event
                    .process_id
                    .and_then(crate::platform::ProcessWatch::open)
                else {
                    return false;
                };
                if process.exited() {
                    return false;
                }
                self.detected_agent = Some(agent);
                self.invocation = Some(invocation);
                self.agent_process = Some(process);
                self.status = Status::default();
                return true;
            }
            if self.invocation.is_none() || self.invocation != event.invocation {
                return false;
            }
            if event.name == "ShellAgentEnded" {
                if event
                    .exit_code
                    .is_some_and(ai_terminal::status::failed_exit)
                {
                    self.status.activity = Activity::Failed;
                    self.status.unread = !viewing;
                }
                self.agent_ended();
                return true;
            }
        }
        self.status.apply(event, viewing)
    }
    fn shutdown(&mut self) {
        self.detected_agent = None;
        self.invocation = None;
        self.agent_process.take();
        self.job.take();
        self.backend.take();
        self.status.activity = Activity::Exited;
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct App {
    renderer: egui_term::TextRendererSession,
    helper: Result<PathBuf, String>,
    sessions: Vec<Session>,
    active: Option<String>,
    last_directory: Option<PathBuf>,
    can_draw: bool,
    terminal_focus_pending: bool,
    terminal_rect: Option<egui::Rect>,
    sidebar_right: f32,
    bridge: Option<Bridge>,
    pty_sender: Sender<(u64, PtyEvent)>,
    pty_receiver: Receiver<(u64, PtyEvent)>,
    directory: PathBuf,
    save_error: Option<String>,
    load_failed: bool,
    dirty: bool,
    rename: Option<String>,
    rename_text: String,
    close_session: Option<String>,
    confirm_exit: bool,
    allow_exit: bool,
    theme: TerminalTheme,
    font: TerminalFont,
    preferences: model::UiSettings,
    settings_open: bool,
    floating_session_count: Option<usize>,
    floating_zoom: Option<f32>,
    floating: ai_terminal::floating::FloatingBar,
    preferences_save_at: Option<Instant>,
    icons: ai_terminal::sidebar::Icons,
    started: Instant,
    frames: u64,
    ui_micros: u128,
    smoke: bool,
    idle_benchmark: bool,
    gpu_info: serde_json::Value,
    smoke_setup: bool,
    startup_pending: bool,
    screenshot_requested: bool,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        directory: PathBuf,
        smoke: bool,
        idle_benchmark: bool,
    ) -> Self {
        ai_terminal::appearance::setup(&cc.egui_ctx);
        let mut error = None;
        let mut workspace = match model::load(&directory.join("sessions.json")) {
            Ok(w) => w,
            Err(e) => {
                error = Some(e);
                Workspace::default()
            }
        };
        // Old direct-Agent tabs become ordinary PowerShell tabs. Keep their names,
        // directories and historical IDs, but never auto-run an Agent command.
        let mut migrated = false;
        for spec in &mut workspace.sessions {
            if !spec.agent.is_shell() {
                spec.agent = Agent::Shell;
                migrated = true;
            }
        }
        let preferences = workspace.ui.normalize();
        cc.egui_ctx.set_zoom_factor(preferences.ui_scale);
        i18n::apply(&cc.egui_ctx, preferences.language);
        let icons = ai_terminal::sidebar::Icons::new(&cc.egui_ctx);
        let floating = ai_terminal::floating::FloatingBar::new(icons.clone());
        let context = cc.egui_ctx.clone();
        let bridge = match Bridge::start(Arc::new(move || {
            context.request_repaint_of(egui::ViewportId::ROOT)
        })) {
            Ok(b) => Some(b),
            Err(e) => {
                error = Some(format!(
                    "{}: {e}",
                    tr(
                        &cc.egui_ctx,
                        "状态服务无法启动",
                        "Unable to start the status service"
                    )
                ));
                None
            }
        };
        let (pty_sender, pty_receiver) = mpsc::channel();
        let load_failed = error.is_some();
        Self {
            renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
            helper: std::env::current_exe()
                .map(|p| p.with_file_name("ai-terminal-hook.exe"))
                .map_err(|e| e.to_string()),
            sessions: workspace.sessions.into_iter().map(Session::new).collect(),
            active: workspace.active,
            last_directory: workspace.last_directory,
            can_draw: true,
            terminal_focus_pending: true,
            terminal_rect: None,
            sidebar_right: 0.0,
            bridge,
            pty_sender,
            pty_receiver,
            directory,
            save_error: error,
            load_failed,
            dirty: migrated,
            rename: None,
            rename_text: String::new(),
            close_session: None,
            confirm_exit: false,
            allow_exit: false,
            theme: TerminalTheme::default(),
            font: ai_terminal::appearance::terminal_font(14.0),
            preferences,
            settings_open: false,
            floating_session_count: None,
            floating_zoom: None,
            floating,
            preferences_save_at: None,
            icons,
            started: Instant::now(),
            frames: 0,
            ui_micros: 0,
            smoke,
            idle_benchmark,
            gpu_info: renderer_info(cc),
            smoke_setup: false,
            startup_pending: !smoke && !idle_benchmark,
            screenshot_requested: false,
        }
    }
    fn persist(&mut self) {
        if !self.dirty || self.load_failed || self.smoke {
            return;
        }
        let workspace = Workspace {
            sessions: self.sessions.iter().map(|s| s.spec.clone()).collect(),
            active: self.active.clone(),
            last_directory: self.last_directory.clone(),
            ui: self.preferences,
        };
        match model::save(&self.directory.join("sessions.json"), &workspace) {
            Ok(()) => {
                self.dirty = false;
                self.save_error = None;
            }
            Err(e) => self.save_error = Some(e),
        }
    }
    fn start_session(&mut self, index: usize, ctx: &egui::Context) {
        let Some(bridge) = self.bridge.as_ref() else {
            return;
        };
        let session = &mut self.sessions[index];
        if session.backend.is_some() {
            return;
        }
        session.terminal_id = uuid::Uuid::new_v4().as_u128() as u64;
        let result =
            (|| -> Result<(TerminalBackend, crate::platform::ProcessJob, String), String> {
                let helper = self.helper.as_ref().map_err(Clone::clone)?;
                if !helper.is_file() {
                    return Err(tr(
                        ctx,
                        "缺少 ai-terminal-hook.exe，请完整构建项目",
                        "Missing ai-terminal-hook.exe. Keep both executables together.",
                    )
                    .into());
                }
                let launch = launch::build(
                    &session.spec,
                    bridge,
                    &self.directory.join("runtime"),
                    helper,
                )?;
                let backend = TerminalBackend::new(
                    session.terminal_id,
                    ctx.clone(),
                    self.pty_sender.clone(),
                    egui_term::BackendSettings {
                        shell: launch.executable,
                        args: launch.arguments,
                        working_directory: Some(session.spec.directory.clone()),
                        env: launch.environment,
                    },
                )
                .map_err(|e| e.to_string())?;
                let job = crate::platform::ProcessJob::attach(backend.pty_id()).map_err(|e| {
                    format!(
                        "{}: {e}",
                        tr(
                            ctx,
                            "无法管理会话进程",
                            "Unable to manage the session process"
                        )
                    )
                })?;
                Ok((backend, job, launch.generation))
            })();
        match result {
            Ok((backend, job, generation)) => {
                session.backend = Some(backend);
                session.job = Some(job);
                session.generation = generation;
                session.status = Status::default();
                if !session.spec.agent.is_shell() {
                    session.status.session_id = session.spec.agent_session_id.clone();
                }
                if session.spec.agent.is_shell() {
                    session.status.activity = Activity::Idle;
                }
                session.error = None;
                session.seen.clear();
                self.terminal_focus_pending = true;
                ctx.request_repaint_after(Duration::from_secs(2));
            }
            Err(e) => {
                session.error = Some(e);
                session.status.activity = Activity::Failed;
            }
        }
        self.sync_visibility();
    }
    fn sync_visibility(&self) {
        for s in &self.sessions {
            if let Some(b) = &s.backend {
                b.set_visible(self.can_draw && self.active.as_deref() == Some(&s.spec.id));
            }
        }
    }
    fn activate(&mut self, id: String, ctx: &egui::Context) {
        self.terminal_focus_pending = true;
        self.active = Some(id.clone());
        if let Some(session) = self.sessions.iter_mut().find(|s| s.spec.id == id) {
            session.status.acknowledge();
            self.last_directory = Some(session.spec.directory.clone());
        }
        if let Some(index) = self.sessions.iter().position(|s| s.spec.id == id) {
            self.start_session(index, ctx);
        }
        self.sync_visibility();
        // Selection is persisted on exit; switching terminals never writes to disk.
    }
    fn background_tick(&mut self, ctx: &egui::Context) {
        let can_draw = !ctx.input(|input| input.viewport().minimized.unwrap_or(false));
        if self.can_draw != can_draw {
            self.can_draw = can_draw;
            self.sync_visibility();
        }
        if self.startup_pending {
            self.startup_pending = false;
            let active = self
                .active
                .as_ref()
                .filter(|id| self.sessions.iter().any(|s| &s.spec.id == *id))
                .cloned()
                .or_else(|| self.sessions.first().map(|s| s.spec.id.clone()));
            if let Some(id) = active {
                self.activate(id, ctx);
            } else if !self.load_failed {
                self.new_shell(Agent::Shell, ctx);
            }
        }
        self.receive_events(ctx);
        if self
            .preferences_save_at
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.preferences_save_at = None;
            self.dirty = true;
        }
        self.manage_floating(ctx);
        self.persist();
    }
    fn manage_floating(&mut self, ctx: &egui::Context) {
        use ai_terminal::floating::{Item, Snapshot};
        let zoom = ctx.zoom_factor();
        let zoom_changed = self.floating_zoom.is_some_and(|old| old != zoom);
        for action in self.floating.actions() {
            // An old child-frame position can be queued while the point scale changes.
            if zoom_changed && matches!(action, ai_terminal::floating::Action::Position(_)) {
                continue;
            }
            self.floating_action(action, ctx);
        }
        if self.preferences.floating_enabled {
            if self.floating_session_count != Some(self.sessions.len()) || zoom_changed {
                // Older workspaces stored egui points. Migrate once using the main viewport scale.
                let saved = self.preferences.floating_position_pixels.or_else(|| {
                    self.preferences.floating_position.map(|p| {
                        let scale = ctx.pixels_per_point();
                        [p[0] * scale, p[1] * scale]
                    })
                });
                let position = Some(
                    ai_terminal::floating::centered_geometry(ctx, self.sessions.len(), saved).0,
                );
                if self.preferences.floating_position_pixels != position {
                    self.preferences.floating_position_pixels = position;
                    self.preferences_changed(ctx);
                }
            }
            self.floating.update(
                Snapshot {
                    items: self
                        .sessions
                        .iter()
                        .map(|s| Item {
                            id: s.spec.id.clone(),
                            name: s.spec.name.clone(),
                            agent: s.display_agent(),
                            color: s.spec.color,
                            activity: s.status.activity,
                            unread: s.status.unread,
                        })
                        .collect(),
                    active: self.active.clone(),
                    locale: i18n::locale(ctx),
                },
                ctx,
            );
            self.floating
                .show(ctx, self.preferences.floating_position_pixels);
        }
        self.floating_session_count = self
            .preferences
            .floating_enabled
            .then_some(self.sessions.len());
        self.floating_zoom = self.preferences.floating_enabled.then_some(zoom);
    }
    fn floating_action(&mut self, action: ai_terminal::floating::Action, ctx: &egui::Context) {
        use ai_terminal::floating::Action;
        match action {
            Action::Activate(id) if self.sessions.iter().any(|s| s.spec.id == id) => {
                self.activate(id, ctx);
                ai_terminal::floating::restore_main(ctx);
            }
            Action::Disable => {
                self.preferences.floating_enabled = false;
                self.preferences_changed(ctx);
            }
            Action::Position(position)
                if position.iter().all(|v| v.is_finite())
                    && self.preferences.floating_position_pixels != Some(position) =>
            {
                self.preferences.floating_position_pixels = Some(position);
                self.preferences_changed(ctx);
            }
            _ => {}
        }
    }
    fn receive_events(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        if let Some(bridge) = &self.bridge {
            for event in bridge.receiver.try_iter().take(256) {
                let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|s| s.spec.id == event.local_id)
                else {
                    continue;
                };
                if session.generation != event.generation
                    || session.backend.is_none()
                    || session.seen.contains(&event.event_id)
                {
                    continue;
                }
                let viewing = focused && self.active.as_deref() == Some(&session.spec.id);
                if session.event(&event, viewing) {
                    session.last_event = Some(Instant::now());
                    if session.status.session_id.is_some()
                        && session.spec.agent_session_id != session.status.session_id
                    {
                        session.spec.agent_session_id = session.status.session_id.clone();
                        self.dirty = true;
                    }
                    if session.seen.len() >= 128 {
                        session.seen.pop_front();
                    }
                    session.seen.push_back(event.event_id);
                }
            }
        }
        for (terminal_id, event) in self.pty_receiver.try_iter().take(256) {
            let Some(session) = self
                .sessions
                .iter_mut()
                .find(|s| s.terminal_id == terminal_id)
            else {
                continue;
            };
            match event {
                PtyEvent::Title(title) if session.spec.agent.is_shell() => {
                    if let Some(path) =
                        ai_terminal::shell::directory_from_title(&title, &session.generation)
                    {
                        if session.agent_process.as_ref().is_some_and(|p| p.exited()) {
                            session.agent_ended();
                        }
                        if session.spec.directory != path {
                            session.spec.directory = path.clone();
                            self.dirty = true;
                        }
                        if self.active.as_deref() == Some(&session.spec.id)
                            && self.last_directory.as_ref() != Some(&path)
                        {
                            self.last_directory = Some(path);
                            self.dirty = true;
                        }
                    }
                }
                PtyEvent::ChildExit(status) => {
                    if !status.success() {
                        session.error = Some(format!(
                            "{}: {status}",
                            tr(ctx, "会话进程已退出", "Session process exited")
                        ));
                    }
                }
                PtyEvent::Exit => {
                    session.agent_ended();
                    session.status.activity = if session.error.is_some() {
                        Activity::Failed
                    } else {
                        Activity::Exited
                    };
                    session.backend.take();
                    session.job.take();
                }
                PtyEvent::Bell => {} // Agent lifecycle events own status; terminal bells are not completion.
                _ => {}
            }
        }
        if focused
            && let Some(session) = self
                .sessions
                .iter_mut()
                .find(|s| Some(&s.spec.id) == self.active.as_ref())
        {
            session.status.acknowledge();
        }
    }
    fn preferences_changed(&mut self, ctx: &egui::Context) {
        self.preferences_save_at = Some(Instant::now() + Duration::from_millis(400));
        ctx.request_repaint_after(Duration::from_millis(420));
    }
    fn set_ui_scale(&mut self, scale: f32, ctx: &egui::Context) {
        if self.preferences.ui_scale != scale {
            self.preferences.ui_scale = scale;
            ctx.set_zoom_factor(scale);
            self.preferences_changed(ctx);
        }
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let locale = i18n::locale(ui.ctx());
        let entries: Vec<_> = self
            .sessions
            .iter()
            .map(|s| ai_terminal::sidebar::Entry {
                id: &s.spec.id,
                name: &s.spec.name,
                directory: &s.spec.directory,
                agent: s.display_agent(),
                color: s.spec.color,
                activity: s.status.activity,
                unread: s.status.unread,
                hint: format!(
                    "{}{}",
                    if s.backend.is_some() || s.status.activity != Activity::Unknown {
                        i18n::activity(locale, s.status.activity)
                    } else {
                        ""
                    },
                    s.last_event
                        .map(|t| format!(
                            " · {} {} · {} {}",
                            locale.text("最近事件", "Last event"),
                            s.status.last_event.as_deref().unwrap_or(""),
                            t.elapsed().as_secs(),
                            locale.text("秒前", "seconds ago")
                        ))
                        .unwrap_or_default()
                ),
            })
            .collect();
        let actions = ai_terminal::sidebar::show(
            ui,
            &entries,
            self.active.as_deref(),
            self.preferences.sidebar_collapsed,
            self.preferences.sidebar_width,
            &self.icons,
        );
        self.sidebar_right = ui.cursor().min.x;
        let ctx = ui.ctx();
        if actions.settings {
            self.settings_open = !self.settings_open;
        }
        if let Some(shell) = actions.create {
            self.new_shell(shell, ctx);
        }
        if actions.toggle {
            self.preferences.sidebar_collapsed = !self.preferences.sidebar_collapsed;
            self.terminal_focus_pending = true;
            self.preferences_changed(ctx);
        } else if !self.preferences.sidebar_collapsed
            && (actions.width - self.preferences.sidebar_width).abs() > 0.5
        {
            self.preferences.sidebar_width = actions.width;
            self.preferences_changed(ctx);
        }
        if let Some(id) = actions.select {
            self.activate(id, ctx);
        }
        if let Some((id, color)) = actions.color {
            self.set_session_color(&id, color);
        }
        if let Some(id) = actions.rename
            && let Some(session) = self.sessions.iter().find(|s| s.spec.id == id)
        {
            self.rename_text = session.spec.name.clone();
            self.rename = Some(id);
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("session-rename-input")));
        }
        if actions.close.is_some() {
            self.close_session = actions.close;
        }
        if let Some((from, to, after)) = actions.reorder
            && let Some(source) = self.sessions.iter().position(|s| s.spec.id == from)
            && let Some(target) = self.sessions.iter().position(|s| s.spec.id == to)
            && model::reorder(&mut self.sessions, source, target, after)
        {
            self.dirty = true;
        }
    }
    fn set_session_color(&mut self, id: &str, color: Option<model::SessionColor>) {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.spec.id == id)
            && session.spec.color != color
        {
            session.spec.color = color;
            self.dirty = true;
        }
    }
    fn terminal(&mut self, ui: &mut egui::Ui) {
        self.terminal_rect = None;
        let ctx = ui.ctx().clone();
        let selected = self
            .sessions
            .iter()
            .position(|s| Some(&s.spec.id) == self.active.as_ref());
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(12))
            .show(ui, |ui| {
                let Some(index) = selected else {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading(tr(&ctx,"打开一个终端","Open a terminal"));
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(
                                    tr(&ctx,"选择 PowerShell、CMD 或 Bash，在终端中启动 codex 或 claude。","Choose PowerShell, CMD or Bash, then launch codex or claude."),
                                )
                                .color(MUTED),
                            );
                            ui.add_space(20.0);
                            if let Some(shell) = ai_terminal::sidebar::new_session_menu_with_id(
                                ui,
                                egui::Id::new("new-terminal-empty"),
                            ) {
                                self.new_shell(shell, &ctx);
                            }
                        });
                    });
                    return;
                };
                let focus = self.rename.is_none()
                    && self.close_session.is_none()
                    && !self.confirm_exit
                    && !self.settings_open
                    && !egui::Popup::is_any_open(&ctx);
                let mut reset = false;
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ai_terminal::chrome::CONTROL),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let session = &self.sessions[index];
                        let name_width = ui.available_width() * 0.45;
                        ai_terminal::ui_text::label_with_width(
                            ui,
                            &session.spec.name,
                            14.0,
                            ai_terminal::chrome::TEXT,
                            name_width,
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            reset = ui
                                .add_sized(
                                    [56.0, ai_terminal::chrome::CONTROL],
                                    egui::Button::new(format!(
                                        "{:.0}%",
                                        self.preferences.ui_scale * 100.0
                                    ))
                                    .frame(false),
                                )
                                .on_hover_text(tr(&ctx,"Ctrl＋滚轮缩放整个界面 · 点击恢复 100%","Ctrl+wheel zooms the window · Click to reset to 100%"))
                                .clicked();
                            if session.backend.is_some() && !session.display_agent().is_shell() {
                                ai_terminal::connection_ui::show(
                                    ui,
                                    session.display_agent(),
                                    &session.status,
                                );
                            }
                            ui.add(
                                egui::Label::new(
                                    RichText::new(session.spec.directory.to_string_lossy())
                                        .size(12.0)
                                        .color(MUTED),
                                )
                                .truncate(),
                            )
                            .on_hover_text(session.spec.directory.display().to_string());
                        });
                    },
                );
                ui.add_space(8.0);
                if reset {
                    self.set_ui_scale(1.0, &ctx);
                    self.terminal_focus_pending = true;
                }
                let session = &mut self.sessions[index];
                if let Some(error) = &session.error {
                    ui.colored_label(YELLOW, error);
                }
                let native_image_paste = !session.display_agent().is_shell();
                if let Some(backend) = &mut session.backend {
                    let view = TerminalView::new(ui, backend)
                        .set_native_image_paste(native_image_paste)
                        .set_theme(self.theme.clone())
                        .set_font(self.font.clone())
                        .set_focus(focus)
                        .request_focus(focus && std::mem::take(&mut self.terminal_focus_pending))
                        .set_size(ui.available_size());
                    self.terminal_rect = Some(ui.add(view).rect);
                } else if session.error.is_none() {
                    ui.label(
                        RichText::new(
                            tr(&ctx,"终端已退出。点击左侧会话重新打开，或按 Ctrl＋Shift＋N 新建终端。","Terminal exited. Select it in the sidebar to reopen, or press Ctrl+Shift+N for a new terminal."),
                        )
                        .color(MUTED),
                    );
                }
            });
    }
    fn new_shell(&mut self, shell: Agent, ctx: &egui::Context) {
        if !shell.is_shell() {
            return;
        }
        let active = self
            .sessions
            .iter()
            .find(|s| Some(&s.spec.id) == self.active.as_ref())
            .map(|s| s.spec.directory.as_path());
        let directory = model::default_directory(active, self.last_directory.as_deref());
        let spec = SessionSpec {
            id: uuid::Uuid::new_v4().to_string(),
            name: shell.label().into(),
            directory: directory.clone(),
            agent: shell,
            agent_session_id: None,
            color: None,
        };
        let id = spec.id.clone();
        self.sessions.push(Session::new(spec));
        self.last_directory = Some(directory);
        self.dirty = true;
        self.activate(id, ctx);
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if ai_terminal::settings::show(ctx, &mut self.settings_open, &mut self.preferences) {
            i18n::apply(ctx, self.preferences.language);
            self.preferences_changed(ctx);
            ctx.request_repaint();
        }
        if let Some(id) = self.rename.clone() {
            let mut done = false;
            let mut open = true;
            egui::Window::new(tr(ctx, "重命名", "Rename"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let input = ai_terminal::ui_text::edit(
                        ui,
                        &mut self.rename_text,
                        egui::Id::new("session-rename-input"),
                    )
                    .response;
                    done = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    done |= ui
                        .add_enabled(
                            !self.rename_text.trim().is_empty(),
                            egui::Button::new(tr(ctx, "保存", "Save")),
                        )
                        .clicked();
                });
            if done && !self.rename_text.trim().is_empty() {
                if let Some(s) = self.sessions.iter_mut().find(|s| s.spec.id == id) {
                    s.spec.name = self.rename_text.trim().into();
                }
                self.dirty = true;
                self.rename = None;
                self.terminal_focus_pending = true;
            }
            if !open {
                self.rename = None;
                self.terminal_focus_pending = true;
            }
        }
        if let Some(id) = self.close_session.clone() {
            let mut close = false;
            let mut cancel = false;
            egui::Window::new(tr(ctx, "关闭会话？", "Close session?"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(tr(
                        ctx,
                        "该会话的运行进程将结束，对话历史仍由 Agent 保留。",
                        "This session will stop. Conversation history remains with the agent.",
                    ));
                    ui.horizontal(|ui| {
                        close = ui.button(tr(ctx, "关闭会话", "Close session")).clicked();
                        cancel = ui.button(tr(ctx, "取消", "Cancel")).clicked();
                    });
                });
            if close {
                if let Some(index) = self.sessions.iter().position(|s| s.spec.id == id) {
                    self.sessions[index].shutdown();
                    self.sessions.remove(index);
                }
                if self.active.as_deref() == Some(&id) {
                    if let Some(next) = self.sessions.first().map(|s| s.spec.id.clone()) {
                        self.activate(next, ctx);
                    } else {
                        self.active = None;
                    }
                }
                self.close_session = None;
                self.dirty = true;
                self.sync_visibility();
            }
            if cancel {
                self.close_session = None;
            }
        }
        if self.confirm_exit {
            let mut exit = false;
            let mut cancel = false;
            egui::Window::new(tr(ctx, "退出 ai-terminal？", "Quit ai-terminal?"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(tr(
                        ctx,
                        "运行中的会话将结束，下次可以恢复对话历史。",
                        "Running sessions will stop. You can resume conversations next time.",
                    ));
                    ui.horizontal(|ui| {
                        exit = ui.button(tr(ctx, "退出", "Quit")).clicked();
                        cancel = ui.button(tr(ctx, "取消", "Cancel")).clicked();
                    });
                });
            if exit {
                self.allow_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if cancel {
                self.confirm_exit = false;
            }
        }
    }
    fn smoke_step(&mut self, ctx: &egui::Context) {
        if self.idle_benchmark {
            let elapsed = self.started.elapsed();
            if self.frames == 1 {
                let _ = std::fs::create_dir_all(&self.directory);
                let _ = std::fs::write(self.directory.join("ready.json"), b"{}");
            }
            if elapsed >= Duration::from_secs(8) {
                let report = serde_json::json!({"manager_resident_bytes":crate::platform::resident_bytes(),"frames":self.frames,"ui_cpu_micros":self.ui_micros,"gpu":self.gpu_info,"elapsed_ms":elapsed.as_millis()});
                let _ = std::fs::write(
                    self.directory.join("idle.json"),
                    serde_json::to_vec_pretty(&report).unwrap_or_default(),
                );
                self.allow_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(Duration::from_secs(8).saturating_sub(elapsed));
            }
            return;
        }
        if !self.smoke {
            return;
        }
        if !self.smoke_setup {
            self.smoke_setup = true;
            for (index, name) in ["接口联调 🚀", "桌宠修复 👋🏽", "AI 终端 ❤️"]
                .into_iter()
                .enumerate()
            {
                let spec = SessionSpec {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.into(),
                    directory: std::env::current_dir().unwrap_or_default(),
                    agent: Agent::Shell,
                    agent_session_id: None,
                    color: Some(
                        [
                            model::SessionColor::Blue,
                            model::SessionColor::Pink,
                            model::SessionColor::Purple,
                        ][index],
                    ),
                };
                self.sessions.push(Session::new(spec));
                let index = self.sessions.len() - 1;
                self.active = Some(self.sessions[index].spec.id.clone());
                self.start_session(index, ctx);
                if let Some(backend) = &mut self.sessions[index].backend {
                    backend.process_command(BackendCommand::Write("Write-Output 'ai-terminal · 原生终端已连接'; Write-Output 'UTF-8 / ANSI / 多会话'; Write-Output '后台任务完成后，左侧会话会显示绿点。'\r".as_bytes().to_vec()));
                }
            }
            if let Some(bridge) = &self.bridge {
                let endpoint = bridge.endpoint.clone();
                let token = bridge.token.clone();
                let sessions: Vec<_> = self
                    .sessions
                    .iter()
                    .map(|s| (s.spec.id.clone(), s.generation.clone()))
                    .collect();
                std::thread::spawn(move || {
                    for (index, (local_id, generation)) in sessions.into_iter().enumerate() {
                        for name in [
                            "ShellAgentStarted",
                            "SessionStart",
                            "UserPromptSubmit",
                            match index {
                                0 => "Stop",
                                1 => "PermissionRequest",
                                _ => "PreToolUse",
                            },
                        ] {
                            let event = HookEvent {
                                invocation: Some(format!("fixture-run-{index}")),
                                process_id: Some(std::process::id()),
                                event_id: uuid::Uuid::new_v4().to_string(),
                                local_id: local_id.clone(),
                                generation: generation.clone(),
                                name: name.into(),
                                session_id: format!("fixture-{index}"),
                                turn_id: Some("one".into()),
                                tool_name: None,
                                tool_use_id: None,
                                notification_type: None,
                                source: if name == "ShellAgentStarted" {
                                    Some(if index == 1 { "claude" } else { "codex" }.into())
                                } else {
                                    None
                                },
                                background_running: false,
                                exit_code: None,
                            };
                            let _ = ai_terminal::bridge::send(&endpoint, &token, event);
                        }
                    }
                });
            }
            ctx.request_repaint_after(Duration::from_secs(5));
        }
        if self.started.elapsed() > Duration::from_secs(5) && !self.screenshot_requested {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            let _ = std::fs::create_dir_all(&self.directory);
            if let Ok(file) = std::fs::File::create(self.directory.join("smoke.png")) {
                let mut encoder = png::Encoder::new(
                    std::io::BufWriter::new(file),
                    image.width() as u32,
                    image.height() as u32,
                );
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                if let Ok(mut writer) = encoder.write_header() {
                    let bytes: Vec<_> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    let _ = writer.write_image_data(&bytes);
                }
            }
        }
        if self.started.elapsed() > Duration::from_secs(7) {
            let sessions:Vec<_>=self.sessions.iter_mut().map(|s|serde_json::json!({"name":s.spec.name,"activity":format!("{:?}",s.status.activity),"unread":s.status.unread,"pty_pid":s.backend.as_ref().map(|b|b.pty_id()),"terminal_text":s.backend.as_mut().map(|b|b.plain_text())})).collect();
            let pass = self.sessions.len() == 3
                && self.sessions[0].status.unread
                && self.sessions[0].status.activity == Activity::Completed
                && self.sessions[1].status.activity == Activity::Waiting
                && self.sessions[2].status.activity == Activity::Working
                && self.sessions.iter().all(|s| s.backend.is_some());
            let report = serde_json::json!({"ok":pass,"fixture_events":true,"gpu":self.gpu_info,"sessions":sessions,"manager_resident_bytes":crate::platform::resident_bytes(),"frames":self.frames,"ui_cpu_micros":self.ui_micros,"elapsed_ms":self.started.elapsed().as_millis()});
            let _ = std::fs::create_dir_all(&self.directory);
            let _ = std::fs::write(
                self.directory.join("smoke.json"),
                serde_json::to_vec_pretty(&report).unwrap_or_default(),
            );
            self.allow_exit = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if input.viewport_id == egui::ViewportId::ROOT
            && let Some(scale) = ai_terminal::zoom::input(input, self.preferences.ui_scale)
        {
            self.set_ui_scale(scale, ctx);
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // eframe calls logic even when painting is suspended (e.g. minimized).
        self.background_tick(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let began = Instant::now();
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_exit
            && self.sessions.iter().any(|s| s.backend.is_some())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_exit = true;
            if !self.can_draw {
                ai_terminal::floating::restore_main(&ctx);
            }
        }
        if !self.can_draw {
            return;
        }
        self.frames += 1;
        self.smoke_step(&ctx);
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::N)
        }) {
            self.new_shell(Agent::Shell, &ctx);
        }
        if let Some(error) = &self.save_error {
            egui::Panel::top("save_error").show(ui, |ui| {
                ui.colored_label(YELLOW, error);
            });
        }
        self.sidebar(ui);
        self.terminal(ui);
        self.dialogs(&ctx);
        self.persist();
        self.ui_micros += began.elapsed().as_micros();
    }
}
impl Drop for App {
    fn drop(&mut self) {
        self.dirty = true;
        self.persist();
        for session in &mut self.sessions {
            session.shutdown();
        }
    }
}

fn renderer_info(cc: &eframe::CreationContext<'_>) -> serde_json::Value {
    #[cfg(feature = "renderer-glow")]
    {
        use eframe::glow::HasContext;
        cc.gl.as_ref().map(|gl|unsafe{
            // The creation callback owns the current OpenGL context.
            serde_json::json!({"backend":"OpenGL","name":gl.get_parameter_string(eframe::glow::RENDERER),"version":gl.get_parameter_string(eframe::glow::VERSION)})
        }).unwrap_or_default()
    }
    #[cfg(all(not(feature = "renderer-glow"), feature = "renderer-wgpu"))]
    {
        cc.wgpu_render_state.as_ref().map(|render|{
            let info=render.adapter.get_info();
            serde_json::json!({"name":info.name,"backend":format!("{:?}",info.backend),"device_type":format!("{:?}",info.device_type)})
        }).unwrap_or_default()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn completion_updates_while_minimized_without_any_paint() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, directory.path().to_owned(), false, false);
        let id = uuid::Uuid::new_v4().to_string();
        let mut session = Session::new(SessionSpec {
            id: id.clone(),
            name: "background".into(),
            directory: directory.path().into(),
            agent: Agent::Codex,
            agent_session_id: None,
            color: None,
        });
        session.terminal_id = 99;
        session.generation = "background-generation".into();
        let backend = TerminalBackend::new(
            99,
            ctx.clone(),
            app.pty_sender.clone(),
            egui_term::BackendSettings {
                shell: "cmd.exe".into(),
                args: vec!["/Q".into()],
                ..Default::default()
            },
        )
        .unwrap();
        session.job = Some(crate::platform::ProcessJob::attach(backend.pty_id()).unwrap());
        session.backend = Some(backend);
        session.status.activity = Activity::Working;
        app.sessions.push(session);
        let bridge = app.bridge.as_ref().unwrap();
        ai_terminal::bridge::send(
            &bridge.endpoint,
            &bridge.token,
            HookEvent {
                invocation: None,
                process_id: None,
                event_id: "completion".into(),
                local_id: id,
                generation: "background-generation".into(),
                name: "Stop".into(),
                session_id: "main".into(),
                turn_id: None,
                tool_name: None,
                tool_use_id: None,
                notification_type: None,
                source: None,
                background_running: false,
                exit_code: None,
            },
        )
        .unwrap();
        let mut input = egui::RawInput::default();
        let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
        viewport.minimized = Some(true);
        viewport.focused = Some(false);
        ctx.begin_pass(input);
        app.background_tick(&ctx);
        let _ = ctx.end_pass();
        assert_eq!(app.frames, 0, "status must not depend on a paint");
        assert_eq!(app.sessions[0].status.activity, Activity::Completed);
        assert!(app.sessions[0].status.unread);
        assert!(!app.can_draw);
    }

    #[test]
    fn switching_session_restores_focus_once_without_interrupting_composition() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, directory.path().to_owned(), false, false);
        let id = uuid::Uuid::new_v4().to_string();
        let mut session = Session::new(SessionSpec {
            id: id.clone(),
            name: "focus".into(),
            directory: directory.path().into(),
            agent: Agent::Shell,
            agent_session_id: None,
            color: None,
        });
        session.backend = Some(
            TerminalBackend::new(
                771,
                ctx.clone(),
                app.pty_sender.clone(),
                egui_term::BackendSettings {
                    shell: "cmd.exe".into(),
                    args: vec!["/D".into(), "/Q".into()],
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        app.sessions.push(session);
        app.activate(id.clone(), &ctx);
        let raw = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 700.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(raw(), |ui| app.terminal(ui));
        let _ = ctx.run_ui(raw(), |ui| app.terminal(ui));
        let mut button_id = egui::Id::NULL;
        let output = ctx.run_ui(raw(), |ui| {
            let button = ui.button("session switch");
            button_id = button.id;
            button.request_focus();
            app.activate(id.clone(), &ctx);
            app.terminal(ui);
        });
        assert!(
            output.platform_output.ime.is_some(),
            "session activation must focus the terminal"
        );
        assert_ne!(ctx.memory(|m| m.focused()), Some(button_id));
        let mut input = raw();
        input.events.push(egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "nihao".into(),
            active_range_chars: None,
        }));
        let output = ctx.run_ui(input, |ui| app.terminal(ui));
        assert!(
            !output
                .platform_output
                .ime
                .unwrap()
                .should_interrupt_composition,
            "focus restoration must not repeat on the next repaint"
        );
    }

    #[test]
    fn zoom_resizes_live_grid_without_restarting_or_interrupting_ime() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, directory.path().to_owned(), false, false);
        let mut session = Session::new(SessionSpec {
            id: "zoom".into(),
            name: "zoom".into(),
            directory: directory.path().into(),
            agent: Agent::Shell,
            agent_session_id: None,
            color: None,
        });
        session.backend = Some(
            TerminalBackend::new(
                918,
                ctx.clone(),
                app.pty_sender.clone(),
                egui_term::BackendSettings {
                    shell: "cmd.exe".into(),
                    args: vec!["/D".into(), "/Q".into()],
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let pid = session.backend.as_ref().unwrap().pty_id();
        app.sessions.push(session);
        app.activate("zoom".into(), &ctx);
        let raw = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 700.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(raw(), |ui| app.terminal(ui));
        let mut input = raw();
        input.events = vec![
            egui::Event::PointerMoved(egui::pos2(400.0, 350.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 3.0),
                modifiers: egui::Modifiers::CTRL,
                phase: egui::TouchPhase::Move,
            },
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "nihao".into(),
                active_range_chars: None,
            }),
        ];
        eframe::App::raw_input_hook(&mut app, &ctx, &mut input);
        let output = ctx.run_ui(input, |ui| app.terminal(ui));
        assert!((app.preferences.ui_scale - 1.1).abs() < 0.0001);
        assert_eq!(app.font.font_type().size, 14.0);
        assert_eq!(app.sessions[0].backend.as_ref().unwrap().pty_id(), pid);
        assert!(
            !output
                .platform_output
                .ime
                .unwrap()
                .should_interrupt_composition
        );
        assert!(app.preferences_save_at.is_some());
        app.preferences_save_at = Some(Instant::now());
        app.background_tick(&ctx);
        assert_eq!(
            model::load(&directory.path().join("sessions.json"))
                .unwrap()
                .ui
                .ui_scale,
            1.1
        );
    }

    #[test]
    fn entire_window_zoom_keeps_terminal_attached_to_sidebar_and_right_edge() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, directory.path().into(), false, false);
        let mut session = Session::new(SessionSpec {
            id: "geometry".into(),
            name: "geometry".into(),
            directory: directory.path().into(),
            agent: Agent::Shell,
            agent_session_id: None,
            color: None,
        });
        session.backend = Some(
            TerminalBackend::new(
                933,
                ctx.clone(),
                app.pty_sender.clone(),
                egui_term::BackendSettings {
                    shell: "cmd.exe".into(),
                    args: vec!["/D".into(), "/Q".into()],
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let pid = session.backend.as_ref().unwrap().pty_id();
        app.sessions.push(session);
        app.activate("geometry".into(), &ctx);
        let mut sidebar_physical = Vec::new();
        for scale in [0.75, 1.0, 1.5, 2.0] {
            app.set_ui_scale(scale, &ctx);
            for _ in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1160.0 / scale, 760.0 / scale),
                    )),
                    ..Default::default()
                };
                let _ = ctx.run_ui(input, |ui| {
                    app.sidebar(ui);
                    app.terminal(ui);
                });
            }
            let rect = app.terminal_rect.unwrap();
            let screen = ctx.content_rect();
            assert!(
                (rect.left() - app.sidebar_right - 12.0).abs() <= 1.0,
                "terminal detached from sidebar at {scale}: {rect:?}"
            );
            assert!(
                (screen.right() - rect.right() - 12.0).abs() <= 1.0,
                "dead area at right edge at {scale}: {rect:?} / {screen:?}"
            );
            assert!(
                (screen.bottom() - rect.bottom() - 12.0).abs() <= 1.0,
                "dead area below terminal"
            );
            assert_eq!(app.sessions[0].backend.as_ref().unwrap().pty_id(), pid);
            sidebar_physical.push(app.sidebar_right * ctx.pixels_per_point());
        }
        assert!(
            sidebar_physical.windows(2).all(|w| w[1] > w[0]),
            "sidebar did not participate in UI zoom"
        );
    }
    fn shell_metadata(name: &str, invocation: &str, source: &str) -> HookEvent {
        HookEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            local_id: "shell".into(),
            generation: "generation".into(),
            name: name.into(),
            session_id: String::new(),
            turn_id: None,
            tool_name: None,
            tool_use_id: None,
            notification_type: None,
            source: Some(source.into()),
            background_running: false,
            exit_code: None,
            invocation: Some(invocation.into()),
            process_id: Some(std::process::id()),
        }
    }
    #[test]
    fn shell_icons_follow_agent_lifecycles_without_guessing_execution_status() {
        let root = tempfile::tempdir().unwrap();
        let mut session = Session::new(SessionSpec {
            id: "shell".into(),
            name: "my terminal".into(),
            directory: root.path().into(),
            agent: Agent::Cmd,
            agent_session_id: None,
            color: None,
        });
        assert_eq!(session.display_agent(), Agent::Cmd);
        assert!(session.event(&shell_metadata("ShellAgentStarted", "one", "codex"), false));
        assert_eq!(session.display_agent(), Agent::Codex);
        assert_eq!(
            session.status.activity,
            Activity::Unknown,
            "a process is not proof that a turn is working"
        );
        let mut prompt = shell_metadata("UserPromptSubmit", "one", "codex");
        prompt.session_id = "actual-conversation".into();
        assert!(session.event(&prompt, false));
        assert_eq!(session.status.activity, Activity::Working);
        assert!(session.event(&shell_metadata("ShellAgentEnded", "one", "codex"), false));
        assert_eq!(session.display_agent(), Agent::Cmd);
        assert!(session.event(&shell_metadata("ShellAgentStarted", "two", "claude"), false));
        assert_eq!(session.display_agent(), Agent::Claude);
        assert!(
            !session.event(&prompt, false),
            "late previous-agent hooks must be ignored"
        );
        assert!(!session.event(&shell_metadata("ShellAgentEnded", "one", "codex"), false));
        assert_eq!(session.display_agent(), Agent::Claude);
        assert_eq!(
            session.spec.name, "my terminal",
            "automatic icons must not overwrite user names"
        );
    }
    #[test]
    fn shell_status_follows_in_process_resume_and_keeps_completion_after_selection() {
        let root = tempfile::tempdir().unwrap();
        let mut session = Session::new(SessionSpec {
            id: "shell".into(),
            name: "terminal".into(),
            directory: root.path().into(),
            agent: Agent::Shell,
            agent_session_id: None,
            color: None,
        });
        assert!(session.event(&shell_metadata("ShellAgentStarted", "run", "claude"), true));
        let mut e = shell_metadata("SessionStart", "run", "startup");
        e.session_id = "initial".into();
        assert!(session.event(&e, true));
        e.source = Some("resume".into());
        e.session_id = "restored".into();
        assert!(session.event(&e, true));
        e.name = "UserPromptSubmit".into();
        assert!(session.event(&e, true));
        assert_eq!(session.status.activity, Activity::Working);
        e.name = "Stop".into();
        assert!(session.event(&e, true));
        session.status.acknowledge();
        assert_eq!(session.status.activity, Activity::Completed);
        assert!(!session.status.unread);
        assert_eq!(session.display_agent(), Agent::Claude);
    }
    #[test]
    fn new_terminals_use_active_cwd_and_keep_existing_agent_sessions() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("recent project");
        std::fs::create_dir(&project).unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, root.path().join("data"), false, false);
        app.sessions.push(Session::new(SessionSpec {
            id: "legacy".into(),
            name: "saved Codex".into(),
            directory: project.clone(),
            agent: Agent::Codex,
            agent_session_id: Some("old-conversation".into()),
            color: None,
        }));
        app.active = Some("legacy".into());
        app.new_shell(Agent::Cmd, &ctx);
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.sessions[1].spec.directory, project);
        assert_eq!(app.sessions[1].spec.agent, Agent::Cmd);
        assert_eq!(
            app.sessions[0].spec.agent_session_id.as_deref(),
            Some("old-conversation")
        );
        assert_eq!(app.last_directory, Some(project));
    }
    fn use_built_helper(app: &mut App) {
        let exe = std::env::current_exe().unwrap();
        let helper = exe
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ai-terminal-hook.exe");
        assert!(helper.is_file(), "Cargo must build the companion binary");
        app.helper = Ok(helper);
    }

    #[test]
    fn restored_tabs_open_shells_on_selection_without_launching_saved_agents() {
        let root = tempfile::tempdir().unwrap();
        let workspace = Workspace {
            sessions: [Agent::Codex, Agent::Claude, Agent::Cmd]
                .into_iter()
                .enumerate()
                .map(|(i, agent)| SessionSpec {
                    id: format!("saved-{i}"),
                    name: format!("项目 👩🏽‍💻 {i}"),
                    directory: root.path().into(),
                    agent,
                    agent_session_id: Some(format!("history-{i}")),
                    color: None,
                })
                .collect(),
            active: Some("saved-1".into()),
            ..Default::default()
        };
        model::save(&root.path().join("sessions.json"), &workspace).unwrap();
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&cc, root.path().into(), false, false);
        use_built_helper(&mut app);
        assert!(app.sessions.iter().all(|s| s.backend.is_none()));
        assert_eq!(app.sessions[0].spec.agent, Agent::Shell);
        assert_eq!(app.sessions[1].spec.agent, Agent::Shell);
        assert_eq!(app.sessions[2].spec.agent, Agent::Cmd);
        assert_eq!(
            app.sessions[1].spec.agent_session_id.as_deref(),
            Some("history-1")
        );
        assert_eq!(app.sessions[1].spec.name, "项目 👩🏽‍💻 1");
        ctx.begin_pass(egui::RawInput::default());
        app.background_tick(&ctx);
        let _ = ctx.end_pass();
        assert!(
            app.sessions[0].backend.is_none(),
            "hidden tabs must not all launch at startup"
        );
        let pid = app.sessions[1]
            .backend
            .as_ref()
            .expect("selected shell starts immediately")
            .pty_id();
        assert!(app.sessions[1].detected_agent.is_none());
        assert_eq!(app.sessions[1].status.activity, Activity::Idle);
        app.set_session_color("saved-1", Some(model::SessionColor::Pink));
        app.persist();
        assert_eq!(
            model::load(&root.path().join("sessions.json"))
                .unwrap()
                .sessions[1]
                .color,
            Some(model::SessionColor::Pink)
        );
        app.activate("saved-0".into(), &ctx);
        assert!(app.sessions[0].backend.is_some());
        app.activate("saved-1".into(), &ctx);
        assert_eq!(app.sessions[1].spec.color, Some(model::SessionColor::Pink));
        assert_eq!(
            app.sessions[1].backend.as_ref().unwrap().pty_id(),
            pid,
            "switching keeps the live shell"
        );
    }

    #[test]
    fn empty_workspace_opens_one_powershell_without_a_start_screen() {
        let root = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&cc, root.path().into(), false, false);
        use_built_helper(&mut app);
        for _ in 0..2 {
            ctx.begin_pass(egui::RawInput::default());
            app.background_tick(&ctx);
            let _ = ctx.end_pass();
        }
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.sessions[0].spec.agent, Agent::Shell);
        assert!(app.sessions[0].backend.is_some());
        assert_eq!(
            app.active.as_deref(),
            Some(app.sessions[0].spec.id.as_str())
        );
    }
    #[test]
    fn island_selection_restores_existing_pty_and_position_and_disable_are_saved() {
        use ai_terminal::floating::Action;
        let root = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, root.path().into(), false, false);
        use_built_helper(&mut app);
        app.startup_pending = false;
        app.new_shell(Agent::Cmd, &ctx);
        app.new_shell(Agent::Cmd, &ctx);
        let id = app.sessions[0].spec.id.clone();
        let pid = app.sessions[0].backend.as_ref().unwrap().pty_id();
        app.sessions[0].status.activity = Activity::Completed;
        app.sessions[0].status.unread = true;
        app.can_draw = false;
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .minimized = Some(true);
        ctx.begin_pass(input);
        app.floating_action(Action::Activate(id.clone()), &ctx);
        let output = ctx.end_pass();
        assert_eq!(app.active.as_deref(), Some(id.as_str()));
        assert_eq!(app.sessions[0].backend.as_ref().unwrap().pty_id(), pid);
        assert_eq!(app.sessions[0].status.activity, Activity::Completed);
        assert!(!app.sessions[0].status.unread);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Minimized(false))
        );
        app.preferences.floating_enabled = true;
        app.floating_action(Action::Position([-320.0, 80.0]), &ctx);
        assert_eq!(
            app.preferences.floating_position_pixels,
            Some([-320.0, 80.0])
        );
        app.floating_action(Action::Disable, &ctx);
        assert!(!app.preferences.floating_enabled);
        app.dirty = true;
        app.persist();
        let saved = model::load(&root.path().join("sessions.json")).unwrap();
        assert!(!saved.ui.floating_enabled);
        assert_eq!(saved.ui.floating_position_pixels, Some([-320.0, 80.0]));
    }
    #[test]
    fn island_stays_on_screen_when_the_session_count_grows() {
        let root = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, root.path().into(), false, false);
        app.startup_pending = false;
        app.preferences.floating_enabled = true;
        app.preferences.floating_position_pixels = Some([1_000_000.0, 30.0]);
        let make = |i| {
            Session::new(SessionSpec {
                id: format!("session-{i}"),
                name: format!("Session {i}"),
                directory: root.path().into(),
                agent: Agent::Shell,
                agent_session_id: None,
                color: None,
            })
        };
        app.sessions.push(make(0));
        let frame = |app: &mut App| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 800.0),
                    )),
                    ..Default::default()
                },
                |_| app.manage_floating(&ctx),
            )
        };
        frame(&mut app);
        let first = app.preferences.floating_position_pixels.unwrap();
        for i in 1..8 {
            app.sessions.push(make(i));
        }
        frame(&mut app);
        let second = app.preferences.floating_position_pixels.unwrap();
        let (_, first_width) = ai_terminal::floating::centered_geometry(&ctx, 1, Some(first));
        let (_, second_width) = ai_terminal::floating::centered_geometry(&ctx, 8, Some(second));
        assert!(
            (first[0] + first_width * ctx.pixels_per_point() / 2.0
                - second[0]
                - second_width * ctx.pixels_per_point() / 2.0)
                .abs()
                < 0.1,
            "changing session count must preserve the monitor's horizontal center"
        );
        assert_eq!(
            ai_terminal::floating::geometry(&ctx, app.sessions.len(), Some(second)).0,
            second
        );
    }
    #[test]
    fn island_migrates_legacy_logical_position_to_physical_pixels() {
        let root = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, root.path().into(), false, false);
        app.startup_pending = false;
        app.preferences.floating_enabled = true;
        app.preferences.floating_position = Some([300.0, 80.0]);
        assert!(app.preferences.floating_position_pixels.is_none());
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                ..Default::default()
            },
            |_| app.manage_floating(&ctx),
        );
        let pos = app.preferences.floating_position_pixels.unwrap();
        assert!((pos[1] - 80.0 * ctx.pixels_per_point()).abs() < 0.1);
        assert_eq!(app.preferences.floating_position, Some([300.0, 80.0]));
        app.dirty = true;
        app.persist();
        let restored = model::load(&root.path().join("sessions.json")).unwrap();
        assert_eq!(restored.ui.floating_position_pixels, Some(pos));
    }

    #[test]
    fn island_zoom_resizes_and_recenters_while_preserving_vertical_position() {
        let root = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(false);
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&creation, root.path().into(), false, false);
        app.startup_pending = false;
        app.preferences.floating_enabled = true;
        app.preferences.floating_position_pixels = Some([200.0, 80.0]);
        app.sessions.push(Session::new(SessionSpec {
            id: "zoom-test".into(),
            name: "Zoom test".into(),
            directory: root.path().into(),
            agent: Agent::Shell,
            agent_session_id: None,
            color: None,
        }));
        let frame = |app: &mut App| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 800.0),
                    )),
                    ..Default::default()
                },
                |_| app.manage_floating(&ctx),
            )
        };
        frame(&mut app);
        let initial = app.preferences.floating_position_pixels.unwrap();
        for zoom in [0.71_f32, 1.0, 1.5] {
            ctx.set_zoom_factor(zoom);
            let out = frame(&mut app);
            let current = app.preferences.floating_position_pixels.unwrap();
            let centered =
                ai_terminal::floating::centered_geometry(&ctx, app.sessions.len(), Some(current)).0;
            assert!(
                (current[0] - centered[0]).abs() < 0.1,
                "floating window must remain centered at {zoom}"
            );
            assert!(
                (current[1] - initial[1]).abs() < 0.1,
                "y position changed at {zoom}"
            );
            assert!(
                out.viewport_output
                    .get(&ai_terminal::floating::viewport_id())
                    .is_some_and(|v| v.commands.iter().any(|command| matches!(command,
                    egui::ViewportCommand::InnerSize(size) if (size.y - 48.0).abs() < 0.01))),
                "zoom {zoom} must resize the native viewport; outputs={:?}",
                out.viewport_output
                    .iter()
                    .map(|(id, viewport)| (*id, &viewport.commands))
                    .collect::<Vec<_>>()
            );
        }
        app.preferences.floating_position_pixels = Some([1_000_000.0, 80.0]);
        app.floating_session_count = None;
        frame(&mut app);
        ctx.set_zoom_factor(2.0);
        frame(&mut app);
        let edge = app.preferences.floating_position_pixels.unwrap();
        assert_eq!(
            ai_terminal::floating::geometry(&ctx, app.sessions.len(), Some(edge)).0,
            edge,
            "zooming near the edge keeps the floating window in the work area"
        );
    }
}
