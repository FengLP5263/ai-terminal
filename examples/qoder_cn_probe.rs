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
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
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
        let (name, tool, notification) = match self.stage {
            0 => ("SessionStart", None, None),
            1 => ("UserPromptSubmit", None, None),
            2 => ("PreToolUse", Some("AskUserQuestion"), None),
            3 | 4 => ("Stop", None, None),
            _ => ("StopFailure", None, None),
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
            tool_use_id: None,
            notification_type: notification,
            source: Some("startup".into()),
            background_running: false,
            exit_code: None,
        };
        self.status.apply(&event, false);
        if matches!(self.stage, 4 | 6) {
            self.status.acknowledge();
        }
    }
    fn finish(&mut self, ctx: &egui::Context) {
        self.finished = true;
        std::fs::write(
            self.output.join("result.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "ok": self.shots == if self.synthetic { 7 } else { 1 },
                "synthetic": self.synthetic, "screenshots": self.shots,
            "hook_connected": !self.synthetic && self.status.last_event.is_some(), "event_names": self.event_names,
                "activity": format!("{:?}", self.status.activity), "model_request_sent": false,
            "trust_approved": false, "manager_modified_user_configuration": false,
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
            self.status.apply(&event, false);
            self.changed = Instant::now();
        }
        if self.began.elapsed() > Duration::from_secs(35) {
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
            self.stage == 6,
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
                    } else {
                        "Qoder CN startup only — no prompt or trust confirmation"
                    });
                    ui.label(format!("{:?}", self.status.activity));
                });
                let snapshot = floating::Snapshot {
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
            let name = if self.synthetic {
                [
                    "ready",
                    "working",
                    "question",
                    "completed-unread",
                    "completed-read",
                    "failed-unread",
                    "failed-read-rail",
                ][self.stage]
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
            if !self.synthetic || self.stage == 6 {
                self.finish(&ctx);
            } else {
                self.stage += 1;
                self.synthetic_status();
                self.frames = 0;
                self.changed = Instant::now();
            }
        }
        let delay = if self.synthetic {
            Duration::from_millis(800)
        } else if self.status.last_event.is_some() {
            Duration::from_secs(3)
        } else {
            Duration::from_secs(25)
        };
        if !self.requested
            && self.changed.elapsed() > delay
            && self.frames >= 4
            && !self.terminal.plain_text().trim().is_empty()
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
            .expect("qoder_cn_probe output-directory [fixture]"),
    );
    let synthetic = std::env::args().nth(2).as_deref() == Some("fixture");
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
                    directory: std::env::current_dir().unwrap(),
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
