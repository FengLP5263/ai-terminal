use ai_terminal::{
    bridge::Bridge,
    launch,
    model::{self, Agent, SessionSpec, Workspace},
    status::{Activity, HookEvent, Status},
};
use egui::{Color32, RichText};
use egui_term::{
    BackendCommand, PtyEvent, TerminalBackend, TerminalFont, TerminalTheme, TerminalView,
};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

const BG: Color32 = ai_terminal::appearance::TERMINAL_BG;
const MUTED: Color32 = Color32::from_rgb(137, 145, 158);
const GREEN: Color32 = Color32::from_rgb(88, 191, 141);
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
        }
    }
    fn shutdown(&mut self) {
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
    sessions: Vec<Session>,
    active: Option<String>,
    can_draw: bool,
    terminal_focus_pending: bool,
    bridge: Option<Bridge>,
    pty_sender: Sender<(u64, PtyEvent)>,
    pty_receiver: Receiver<(u64, PtyEvent)>,
    directory: PathBuf,
    save_error: Option<String>,
    load_failed: bool,
    dirty: bool,
    show_new: bool,
    new_name: String,
    new_directory: String,
    new_agent: Agent,
    new_error: Option<String>,
    folder_receiver: Option<Receiver<Option<PathBuf>>>,
    rename: Option<String>,
    rename_text: String,
    close_session: Option<String>,
    confirm_exit: bool,
    allow_exit: bool,
    theme: TerminalTheme,
    font: TerminalFont,
    started: Instant,
    frames: u64,
    ui_micros: u128,
    smoke: bool,
    idle_benchmark: bool,
    gpu_info: serde_json::Value,
    smoke_setup: bool,
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
        let workspace = match model::load(&directory.join("sessions.json")) {
            Ok(w) => w,
            Err(e) => {
                error = Some(e);
                Workspace::default()
            }
        };
        let context = cc.egui_ctx.clone();
        let bridge = match Bridge::start(Arc::new(move || context.request_repaint())) {
            Ok(b) => Some(b),
            Err(e) => {
                error = Some(format!("状态服务无法启动：{e}"));
                None
            }
        };
        let (pty_sender, pty_receiver) = mpsc::channel();
        let load_failed = error.is_some();
        Self {
            sessions: workspace.sessions.into_iter().map(Session::new).collect(),
            active: workspace.active,
            can_draw: true,
            terminal_focus_pending: true,
            bridge,
            pty_sender,
            pty_receiver,
            directory,
            save_error: error,
            load_failed,
            dirty: false,
            show_new: false,
            new_name: String::new(),
            new_directory: String::new(),
            new_agent: Agent::Codex,
            new_error: None,
            folder_receiver: None,
            rename: None,
            rename_text: String::new(),
            close_session: None,
            confirm_exit: false,
            allow_exit: false,
            theme: TerminalTheme::default(),
            font: ai_terminal::appearance::terminal_font(15.0),
            started: Instant::now(),
            frames: 0,
            ui_micros: 0,
            smoke,
            idle_benchmark,
            gpu_info: renderer_info(cc),
            smoke_setup: false,
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
                let helper = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .with_file_name("ai-terminal-hook.exe");
                if !helper.is_file() {
                    return Err("缺少 ai-terminal-hook.exe，请完整构建项目".into());
                }
                let launch = launch::build(
                    &session.spec,
                    bridge,
                    &self.directory.join("runtime"),
                    &helper,
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
                let job = crate::platform::ProcessJob::attach(backend.pty_id())
                    .map_err(|e| format!("无法管理会话进程：{e}"))?;
                Ok((backend, job, launch.generation))
            })();
        match result {
            Ok((backend, job, generation)) => {
                session.backend = Some(backend);
                session.job = Some(job);
                session.generation = generation;
                session.status = Status::default();
                session.status.session_id = session.spec.agent_session_id.clone();
                if session.spec.agent == Agent::Shell {
                    session.status.activity = Activity::Idle;
                }
                session.error = None;
                session.seen.clear();
                self.terminal_focus_pending = true;
                ctx.request_repaint_after(Duration::from_secs(2));
            }
            Err(e) => {
                session.error = Some(e);
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
    fn activate(&mut self, id: String) {
        self.terminal_focus_pending = true;
        self.active = Some(id.clone());
        if let Some(session) = self.sessions.iter_mut().find(|s| s.spec.id == id) {
            session.status.acknowledge();
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
        self.receive_events(ctx);
        self.persist();
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
                if session.status.apply(&event, viewing) {
                    session.last_event = Some(Instant::now());
                    if session.spec.agent_session_id != session.status.session_id {
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
                PtyEvent::ChildExit(status) => {
                    if !status.success() {
                        session.error = Some(format!("会话进程已退出：{status}"));
                    }
                }
                PtyEvent::Exit => {
                    session.status.activity = if session.error.is_some() {
                        Activity::Waiting
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
        if let Some(rx) = &self.folder_receiver
            && let Ok(path) = rx.try_recv()
        {
            if let Some(path) = path {
                self.new_directory = path.to_string_lossy().into_owned();
            }
            self.folder_receiver = None;
        }
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::left("sessions")
            .exact_size(238.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ai_terminal::appearance::SIDEBAR_BG)
                    .inner_margin(16),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ai-terminal").size(19.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("＋")
                            .on_hover_text("新建会话 · Ctrl+Shift+N")
                            .clicked()
                        {
                            self.show_new = true;
                        }
                    });
                });
                ui.add_space(22.0);
                let mut groups: BTreeMap<PathBuf, Vec<usize>> = BTreeMap::new();
                for (index, session) in self.sessions.iter().enumerate() {
                    groups
                        .entry(session.spec.directory.clone())
                        .or_default()
                        .push(index);
                }
                let mut select = None;
                let mut resume = None;
                let mut rename = None;
                let mut close = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (path, indices) in groups {
                        let project = path
                            .file_name()
                            .unwrap_or(path.as_os_str())
                            .to_string_lossy();
                        ui.label(RichText::new(project).color(MUTED).size(12.0))
                            .on_hover_text(path.to_string_lossy());
                        ui.add_space(6.0);
                        for index in indices {
                            let session = &self.sessions[index];
                            let selected = self.active.as_deref() == Some(&session.spec.id);
                            let fill = if selected {
                                Color32::from_rgb(40, 46, 57)
                            } else {
                                Color32::TRANSPARENT
                            };
                            let response = egui::Frame::new()
                                .fill(fill)
                                .corner_radius(7)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(15.0, 18.0),
                                            egui::Sense::hover(),
                                        );
                                        match session.status.activity {
                                            Activity::Working => {
                                                let time = self.started.elapsed().as_secs_f32();
                                                let center = rect.center();
                                                let points: Vec<_> = (0..13)
                                                    .map(|i| {
                                                        let angle =
                                                            time * 4.0 + i as f32 / 12.0 * 4.6;
                                                        center
                                                            + egui::vec2(angle.cos(), angle.sin())
                                                                * 5.0
                                                    })
                                                    .collect();
                                                ui.painter().add(egui::Shape::line(
                                                    points,
                                                    egui::Stroke::new(
                                                        1.7,
                                                        Color32::from_rgb(133, 167, 240),
                                                    ),
                                                ));
                                                if !ctx.input(|i| {
                                                    i.viewport().minimized.unwrap_or(false)
                                                }) {
                                                    ctx.request_repaint_after(
                                                        Duration::from_millis(100),
                                                    );
                                                }
                                            }
                                            Activity::Waiting => {
                                                ui.painter().circle_filled(
                                                    rect.center(),
                                                    4.0,
                                                    YELLOW,
                                                );
                                            }
                                            _ if session.status.unread => {
                                                ui.painter().circle_filled(
                                                    rect.center(),
                                                    4.0,
                                                    GREEN,
                                                );
                                            }
                                            _ => {}
                                        }
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&session.spec.name).size(14.0));
                                            if session.spec.name != session.spec.agent.label() {
                                                ui.label(
                                                    RichText::new(session.spec.agent.label())
                                                        .size(11.0)
                                                        .color(MUTED),
                                                );
                                            }
                                        });
                                    });
                                })
                                .response;
                            let response = ui.interact(
                                response.rect,
                                response.id.with("choose"),
                                egui::Sense::click(),
                            );
                            if response.clicked() {
                                select = Some(session.spec.id.clone());
                            }
                            if response.double_clicked() && session.backend.is_none() {
                                resume = Some(index);
                            }
                            let hint = match session.status.activity {
                                Activity::Working => "正在工作",
                                Activity::Waiting => "需要你处理",
                                Activity::Unknown if session.backend.is_some() => "状态未连接",
                                _ if session.status.unread => "本轮完成",
                                _ => "",
                            };
                            response.clone().on_hover_text(format!(
                                "{}\n{}",
                                hint,
                                session
                                    .last_event
                                    .map(|t| format!(
                                        "最近事件：{} · {} 秒前",
                                        session.status.last_event.as_deref().unwrap_or(""),
                                        t.elapsed().as_secs()
                                    ))
                                    .unwrap_or_default()
                            ));
                            response.context_menu(|ui| {
                                if ui.button("重命名").clicked() {
                                    rename =
                                        Some((session.spec.id.clone(), session.spec.name.clone()));
                                    ui.close();
                                }
                                if session.backend.is_none()
                                    && ui
                                        .button(if session.spec.agent_session_id.is_some() {
                                            "继续会话"
                                        } else {
                                            "启动会话"
                                        })
                                        .clicked()
                                {
                                    resume = Some(index);
                                    ui.close();
                                }
                                if ui.button("关闭会话").clicked() {
                                    close = Some(session.spec.id.clone());
                                    ui.close();
                                }
                            });
                            ui.add_space(4.0);
                        }
                        ui.add_space(16.0);
                    }
                });
                if let Some(id) = select {
                    self.activate(id);
                }
                if let Some(index) = resume {
                    self.start_session(index, &ctx);
                }
                if let Some((id, name)) = rename {
                    self.rename = Some(id);
                    self.rename_text = name;
                }
                if close.is_some() {
                    self.close_session = close;
                }
            });
    }
    fn terminal(&mut self, ui: &mut egui::Ui) {
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
                            ui.heading("打开一个工作目录");
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("选择 Codex 或 Claude Code，开始工作。").color(MUTED),
                            );
                            ui.add_space(20.0);
                            if ui.button("＋ 新建会话").clicked() {
                                self.show_new = true;
                            }
                        });
                    });
                    return;
                };
                let session = &mut self.sessions[index];
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&session.spec.name).strong());
                    ui.label(
                        RichText::new(session.spec.directory.to_string_lossy())
                            .size(12.0)
                            .color(MUTED),
                    );
                });
                ui.add_space(8.0);
                if let Some(error) = &session.error {
                    ui.colored_label(YELLOW, error);
                }
                if let Some(backend) = &mut session.backend {
                    if session.status.activity == Activity::Unknown
                        && session.spec.agent != Agent::Shell
                    {
                        let hint = if session.spec.agent == Agent::Codex {
                            "状态未连接 · 首次使用请在 /hooks 中检查并启用状态钩子"
                        } else {
                            "状态未连接 · 等待 Agent 的会话事件"
                        };
                        ui.label(RichText::new(hint).size(11.0).color(MUTED));
                        ui.add_space(4.0);
                    }
                    let focus = !self.show_new
                        && self.rename.is_none()
                        && self.close_session.is_none()
                        && !self.confirm_exit
                        && !egui::Popup::is_any_open(&ctx);
                    let view = TerminalView::new(ui, backend)
                        .set_theme(self.theme.clone())
                        .set_font(self.font.clone())
                        .set_focus(focus)
                        .request_focus(focus && std::mem::take(&mut self.terminal_focus_pending))
                        .set_size(ui.available_size());
                    ui.add(view);
                } else {
                    let can_resume = session.spec.agent_session_id.is_some();
                    ui.add_space(45.0);
                    ui.label(
                        RichText::new(if can_resume {
                            "会话已保存，可以继续之前的对话。"
                        } else {
                            "启动这个目录中的终端会话。"
                        })
                        .color(MUTED),
                    );
                    ui.add_space(12.0);
                    if ui
                        .button(if can_resume {
                            "继续会话"
                        } else {
                            "启动会话"
                        })
                        .clicked()
                    {
                        self.start_session(index, &ctx);
                    }
                }
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        let mut create = false;
        let mut show = self.show_new;
        egui::Window::new("新建会话")
            .open(&mut show)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("工作目录");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_directory).desired_width(330.0),
                    );
                    if ui.button("选择…").clicked() && self.folder_receiver.is_none() {
                        let (tx, rx) = mpsc::channel();
                        self.folder_receiver = Some(rx);
                        let context = ctx.clone();
                        std::thread::spawn(move || {
                            let path = rfd::FileDialog::new().pick_folder();
                            let _ = tx.send(path);
                            context.request_repaint();
                        });
                    }
                });
                ui.add_space(12.0);
                ui.label("Agent");
                ui.horizontal(|ui| {
                    for agent in [Agent::Codex, Agent::Claude, Agent::Shell] {
                        ui.selectable_value(&mut self.new_agent, agent, agent.label());
                    }
                });
                ui.add_space(12.0);
                ui.label("名称（可选）");
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_name)
                        .hint_text(self.new_agent.label())
                        .desired_width(f32::INFINITY),
                );
                if let Some(error) = &self.new_error {
                    ui.colored_label(YELLOW, error);
                }
                ui.add_space(16.0);
                if ui.button("开始").clicked() {
                    create = true;
                }
            });
        self.show_new = show;
        if create {
            let directory = PathBuf::from(self.new_directory.trim());
            if !directory.is_dir() {
                self.new_error = Some("请选择存在的目录".into());
            } else {
                let spec = SessionSpec {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: if self.new_name.trim().is_empty() {
                        self.new_agent.label().into()
                    } else {
                        self.new_name.trim().into()
                    },
                    directory,
                    agent: self.new_agent,
                    agent_session_id: None,
                };
                let id = spec.id.clone();
                self.sessions.push(Session::new(spec));
                self.dirty = true;
                self.activate(id);
                self.start_session(self.sessions.len() - 1, ctx);
                self.show_new = false;
                self.new_name.clear();
                self.new_error = None;
            }
        }
        if let Some(id) = self.rename.clone() {
            let mut done = false;
            let mut open = true;
            egui::Window::new("重命名")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.rename_text);
                    if ui.button("保存").clicked() {
                        done = true;
                    }
                });
            if done && !self.rename_text.trim().is_empty() {
                if let Some(s) = self.sessions.iter_mut().find(|s| s.spec.id == id) {
                    s.spec.name = self.rename_text.trim().into();
                }
                self.dirty = true;
                self.rename = None;
            }
            if !open {
                self.rename = None;
            }
        }
        if let Some(id) = self.close_session.clone() {
            let mut close = false;
            let mut cancel = false;
            egui::Window::new("关闭会话？")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("该会话的运行进程将结束，对话历史仍由 Agent 保留。");
                    ui.horizontal(|ui| {
                        close = ui.button("关闭会话").clicked();
                        cancel = ui.button("取消").clicked();
                    });
                });
            if close {
                if let Some(index) = self.sessions.iter().position(|s| s.spec.id == id) {
                    self.sessions[index].shutdown();
                    self.sessions.remove(index);
                }
                if self.active.as_deref() == Some(&id) {
                    self.active = self.sessions.first().map(|s| s.spec.id.clone());
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
            egui::Window::new("退出 ai-terminal？")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("运行中的会话将结束，下次可以恢复对话历史。");
                    ui.horizontal(|ui| {
                        exit = ui.button("退出").clicked();
                        cancel = ui.button("取消").clicked();
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
            for name in ["接口联调", "桌宠修复", "ai-terminal"] {
                let spec = SessionSpec {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.into(),
                    directory: std::env::current_dir().unwrap_or_default(),
                    agent: Agent::Shell,
                    agent_session_id: None,
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
                            "SessionStart",
                            "UserPromptSubmit",
                            match index {
                                0 => "Stop",
                                1 => "PermissionRequest",
                                _ => "PreToolUse",
                            },
                        ] {
                            let event = HookEvent {
                                event_id: uuid::Uuid::new_v4().to_string(),
                                local_id: local_id.clone(),
                                generation: generation.clone(),
                                name: name.into(),
                                session_id: format!("fixture-{index}"),
                                turn_id: Some("one".into()),
                                tool_name: None,
                                tool_use_id: None,
                                notification_type: None,
                                source: None,
                                background_running: false,
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
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // eframe calls logic even when painting is suspended (e.g. minimized).
        self.background_tick(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let began = Instant::now();
        let ctx = ui.ctx().clone();
        self.frames += 1;
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_exit
            && self.sessions.iter().any(|s| s.backend.is_some())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_exit = true;
        }
        self.smoke_step(&ctx);
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::N)
        }) {
            self.show_new = true;
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
            agent: Agent::Shell,
            agent_session_id: None,
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
        assert_eq!(app.sessions[0].status.activity, Activity::Idle);
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
        app.activate(id.clone());
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
            app.activate(id.clone());
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
}
