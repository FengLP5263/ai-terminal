//! Qoder CN startup or synthetic lifecycle captures; never submit model prompts.
use ai_terminal::{
    appearance,
    bridge::Bridge,
    floating,
    i18n::Locale,
    launch,
    model::{Agent, SessionSpec},
    sidebar::{self, Entry, Icons},
    status::{HookEvent, Status},
};
use egui_term::{BackendCommand, BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    _job: platform::ProcessJob,
    bridge: Bridge,
    icons: Icons,
    status: Status,
    output: PathBuf,
    synthetic: bool,
    input_probe: bool,
    typed: bool,
    input_checks: Vec<serde_json::Value>,
    stable_marker_frames: usize,
    trace: bool,
    layout_trace: Vec<serde_json::Value>,
    began: Instant,
    changed: Instant,
    stage: usize,
    requested: bool,
    frames: usize,
    event_names: Vec<String>,
    shots: usize,
    finished: bool,
}
impl Probe {
    fn synthetic_status(&mut self) {
        let (name, tool, notification, mode, tool_id) = match self.stage {
            0 => ("SessionStart", None, None, None, None),
            1 => ("UserPromptSubmit", None, None, None, None),
            2 => (
                "Notification",
                Some("Bash"),
                Some("permission_prompt"),
                Some("auto"),
                Some("automatic"),
            ),
            3 => (
                "PreToolUse",
                Some("AskUserQuestion"),
                None,
                Some("auto"),
                Some("question"),
            ),
            4 | 5 => ("Stop", None, None, None, None),
            6 | 7 => ("StopFailure", None, None, None, None),
            8 | 10 => (
                "Notification",
                Some("Bash"),
                Some("permission_prompt"),
                Some("default"),
                Some("manual"),
            ),
            _ => (
                "PostToolUseFailure",
                Some("Bash"),
                None,
                Some("default"),
                Some("manual"),
            ),
        };
        let event = HookEvent {
            invocation: None,
            process_id: None,
            parent_invocation: None,
            console_processes: None,
            event_id: self.stage.to_string(),
            local_id: "fixture".into(),
            generation: "fixture".into(),
            name: name.into(),
            session_id: "qoder-fixture".into(),
            turn_id: None,
            tool_name: tool.map(String::from),
            tool_use_id: tool_id.map(String::from),
            notification_type: notification.map(String::from),
            permission_mode: mode.map(String::from),
            source: Some("startup".into()),
            background_running: false,
            exit_code: None,
        };
        if self.stage == 8 {
            let mut start = event.clone();
            start.name = "UserPromptSubmit".into();
            start.notification_type = None;
            self.status.apply_qoder(&start, false);
        }
        self.status.apply_qoder(&event, false);
        if matches!(self.stage, 5 | 7) {
            self.status.acknowledge();
        }
    }
    fn finish(&mut self, ctx: &egui::Context) {
        assert!(
            !self.input_probe
                || !self
                    .event_names
                    .iter()
                    .any(|name| name == "UserPromptSubmit"),
            "input probe must never submit a model prompt"
        );
        self.finished = true;
        std::fs::write(
            self.output.join("result.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "ok": self.shots == if self.synthetic { 11 } else if self.input_probe || self.trace { 4 } else { 1 },
                "synthetic": self.synthetic, "screenshots": self.shots, "version": env!("CARGO_PKG_VERSION"),
            "hook_connected": !self.synthetic && self.status.last_event.is_some(), "event_names": self.event_names,
                "activity": format!("{:?}", self.status.activity), "model_request_sent": false,
            "trust_approved": false, "manager_modified_user_configuration": false,
            "input_probe": self.input_probe, "input_checks": self.input_checks, "pty_pid": self.terminal.pty_id(),
            "input_started": self.typed, "elapsed_ms": self.began.elapsed().as_millis(),
            "layout_trace": self.layout_trace,
            }))
            .unwrap(),
        )
        .unwrap();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
impl eframe::App for Probe {
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }

    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.finished {
            return;
        }
        for event in self.bridge.receiver.try_iter().take(32) {
            if self.event_names.len() < 32 {
                self.event_names.push(event.name.clone());
            }
            self.status.apply_qoder(&event, false);
            self.changed = Instant::now();
        }
        if self.began.elapsed() > Duration::from_secs(if self.trace { 60 } else { 35 }) {
            self.finish(ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .events
            .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
        input.events.push(egui::Event::PointerGone);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.finished {
            return;
        }
        let ctx = ui.ctx().clone();
        let entries = [Entry {
            id: "qoder",
            name: "Qoder CN 👋",
            directory: std::path::Path::new("C:/Projects/example"),
            agent: Agent::QoderCn,
            color: None,
            activity: self.status.activity,
            unread: self.status.unread,
            hint: "Native startup".into(),
        }];
        sidebar::show(
            ui,
            &entries,
            Some("qoder"),
            self.stage == 7 || self.trace,
            238.0,
            &self.icons,
        );
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(if self.synthetic {
                        "Synthetic Qoder CN lifecycle — no Agent/model"
                    } else if self.input_probe {
                        "Qoder CN unsent input / zoom / cancel — no Enter"
                    } else {
                        "Qoder CN startup only — no prompt or trust confirmation"
                    });
                    ui.label(format!("{:?}", self.status.activity));
                });
                let snapshot = floating::Snapshot {
                    groups: Vec::new(),
                    items: vec![floating::Item {
                        id: "qoder".into(),
                        name: "Qoder CN 👋".into(),
                        agent: Agent::QoderCn,
                        color: None,
                        activity: self.status.activity,
                        unread: self.status.unread,
                    }],
                    active: Some("qoder".into()),
                    locale: Locale::En,
                };
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(70.0, 48.0),
                    )),
                    |ui| {
                        floating::body(ui, &snapshot, &self.icons);
                    },
                );
                ui.add_space(8.0);
                let size = ui.available_size();
                let view = TerminalView::new(ui, &mut self.terminal)
                    .set_font(appearance::terminal_font(14.0))
                    .set_native_image_paste(true)
                    .set_drag_selection(true)
                    .set_size(size);
                ui.add(view);
            });
        self.frames += 1;
        if self.trace {
            let size = self.terminal.sync().terminal_size;
            let wire: egui_term::WindowSize = size.into();
            let text = self.terminal.plain_text();
            let sample = serde_json::json!({"columns":wire.num_cols,"rows":wire.num_lines,"cell_width":size.cell_width,"cell_height":size.cell_height,"scale":ctx.pixels_per_point(),"content_size":[ctx.content_rect().width(),ctx.content_rect().height()],"raw_screen":ctx.input(|i|i.raw.screen_rect.map(|r|[r.width(),r.height()])),"sizing_pass":ui.is_sizing_pass(),"visible":ui.is_visible(),"discard":ctx.will_discard(),"banners":text.lines().filter(|line|line.contains("Signed in")).count(),"trust_questions":text.matches("Do you trust the files in this folder?").count()});
            if self
                .layout_trace
                .last()
                .is_none_or(|old| old["sample"] != sample)
                && self.layout_trace.len() < 128
            {
                self.layout_trace.push(serde_json::json!({"elapsed_ms":self.began.elapsed().as_millis(),"sample":sample}));
            }
        }
        if let Some(image) = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        }) && self.requested
        {
            if self.input_probe
                && !self.terminal.plain_text().contains(if self.stage == 3 {
                    "INPUT_AFTER_CANCEL"
                } else {
                    "INPUT_PROBE"
                })
            {
                // A resize can temporarily clear the live grid while a queued
                // screenshot arrives. Wait for a stable composer before retrying.
                self.requested = false;
                self.stable_marker_frames = 0;
                return;
            }
            let name = if self.synthetic {
                [
                    "ready",
                    "working",
                    "auto-permission-working",
                    "question",
                    "completed-unread",
                    "completed-read",
                    "failed-unread",
                    "failed-read-rail",
                    "manual-permission",
                    "permission-resolved-working",
                    "late-notice-ignored",
                ][self.stage]
            } else if self.trace {
                [
                    "trace-initial",
                    "trace-narrow",
                    "trace-narrower",
                    "trace-restored",
                ][self.stage]
            } else if self.input_probe {
                ["input-100", "input-103", "input-110", "input-after-cancel"][self.stage]
            } else {
                "qoder-cn-startup"
            };
            let mut encoder = png::Encoder::new(
                std::fs::File::create(self.output.join(format!("{name}.png"))).unwrap(),
                image.width() as u32,
                image.height() as u32,
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(
                    &image
                        .pixels
                        .iter()
                        .flat_map(|p| p.to_array())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            self.shots += 1;
            self.requested = false;
            if self.input_probe {
                let text = self.terminal.plain_text();
                let marker = if self.stage == 3 {
                    "INPUT_AFTER_CANCEL"
                } else {
                    "INPUT_PROBE"
                };
                assert!(text.contains(marker), "unsent input remains visible");
                assert!(
                    !matches!(
                        self.status.activity,
                        ai_terminal::status::Activity::Exited
                            | ai_terminal::status::Activity::Failed
                    ),
                    "Qoder remains open after a single cancel"
                );
                assert!(
                    !text.contains("[42;4R"),
                    "cursor response leaked into composer"
                );
                self.input_checks.push(serde_json::json!({"stage":self.stage,"marker_visible":true,"zoom":ctx.zoom_factor(),"unsubmitted":true}));
            }
            if (!self.synthetic && !self.input_probe && !self.trace)
                || (self.synthetic && self.stage == 10)
                || (self.input_probe && self.stage == 3)
                || (self.trace && self.stage == 3)
            {
                self.finish(&ctx);
            } else {
                self.stage += 1;
                if self.synthetic {
                    self.synthetic_status();
                }
                if self.input_probe {
                    ctx.set_zoom_factor([1.0, 1.03, 1.1, 1.1][self.stage]);
                    if self.stage == 3 {
                        self.terminal
                            .process_command(BackendCommand::Write(vec![3]));
                        self.typed = false;
                    }
                }
                if self.trace {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        [1120.0, 800.0, 500.0, 1120.0][self.stage],
                        760.0,
                    )));
                }
                self.frames = 0;
                self.stable_marker_frames = 0;
                self.changed = Instant::now();
            }
        }
        let text = self.terminal.plain_text();
        if self.input_probe
            && !self.typed
            && self.status.last_event.is_some()
            // Probe readiness only: SessionStart does not prove authentication
            // or the empty input composer has finished initializing.
            && text.contains("Type your message")
            && self.frames >= 8
            && self.changed.elapsed() > Duration::from_millis(700)
        {
            let marker = if self.stage == 3 {
                "取消后输入框 INPUT_AFTER_CANCEL"
            } else {
                "输入框定位测试 INPUT_PROBE"
            };
            self.terminal
                .process_command(BackendCommand::Write(marker.as_bytes().to_vec()));
            self.typed = true;
            self.changed = Instant::now();
        }
        if self.input_probe {
            let marker = if self.stage == 3 {
                "INPUT_AFTER_CANCEL"
            } else {
                "INPUT_PROBE"
            };
            self.stable_marker_frames = if text.contains(marker) {
                self.stable_marker_frames + 1
            } else {
                0
            };
        }
        let delay = if self.synthetic {
            Duration::from_millis(800)
        } else if self.status.last_event.is_some()
            || (self.trace && (self.stage > 0 || text.contains("Do you trust the files")))
        {
            Duration::from_secs(3)
        } else {
            Duration::from_secs(25)
        };
        if !self.requested
            && self.changed.elapsed() > delay
            && self.frames >= 4
            && !text.trim().is_empty()
            && (!self.input_probe || (self.typed && self.stable_marker_frames >= 3))
        {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}

fn main() -> eframe::Result {
    launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("qoder_cn_probe output-directory [fixture|input|trace] [isolated-directory]"),
    );
    let synthetic = std::env::args().nth(2).as_deref() == Some("fixture");
    let input_probe = std::env::args().nth(2).as_deref() == Some("input");
    let trace = std::env::args().nth(2).as_deref() == Some("trace");
    let directory = if trace {
        std::env::args()
            .nth(3)
            .map(PathBuf::from)
            .expect("trace requires its own isolated directory")
    } else {
        std::env::current_dir().unwrap()
    };
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal Qoder CN probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([1120.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            if trace {
                ai_terminal::zoom::initialize(&cc.egui_ctx, 1.03);
            }
            let bridge = Bridge::start(Arc::new(|| {})).unwrap();
            let helper = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("ai-terminal-hook.exe");
            let settings = if synthetic {
                BackendSettings { shell: "powershell.exe".into(), args: vec!["-NoLogo".into(), "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(),
                "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); Write-Output 'Qoder CN terminal integration fixture'; Write-Output 'C:/Projects/example'; Write-Output 'Lifecycle: ready, working, needs input, completed, failed'; Write-Output 'Synthetic events only; no Agent or model request'; Start-Sleep -Seconds 30".into()], ..Default::default() }
            } else {
                let spec = SessionSpec {
                    id: "qoder-cn-probe".into(),
                    name: "Qoder CN".into(),
                    directory,
                    agent: Agent::QoderCn,
                    agent_session_id: None,
                    color: None,
                };
                let plan = launch::build(&spec, &bridge, &output.join("runtime"), &helper)
                    .expect("Qoder CN must already be installed");
                BackendSettings {
                    shell: plan.executable,
                    args: plan.arguments,
                    env: plan.environment,
                    working_directory: Some(spec.directory),
                }
            };
            let (sender, _) = mpsc::channel();
            let terminal =
                TerminalBackend::new(967, cc.egui_ctx.clone(), sender, settings).unwrap();
            let job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
            let mut probe = Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                _job: job,
                bridge,
                icons: Icons::new(&cc.egui_ctx),
                status: Status::default(),
                output,
                synthetic,
                input_probe,
                typed: false,
                input_checks: Vec::new(),
                stable_marker_frames: 0,
                trace,
                layout_trace: Vec::new(),
                began: Instant::now(),
                changed: Instant::now(),
                stage: 0,
                requested: false,
                frames: 0,
                event_names: Vec::new(),
                shots: 0,
                finished: false,
            };
            if synthetic {
                probe.synthetic_status();
            }
            Ok(Box::new(probe))
        }),
    )
}
