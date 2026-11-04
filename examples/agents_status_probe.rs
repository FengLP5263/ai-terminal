//! Native Codex agents navigation or synthetic mode-badge captures; no model prompts.
use ai_terminal::{
    appearance,
    bridge::Bridge,
    launch,
    model::{Agent, SessionSpec},
    status::Status,
};
use egui_term::{BackendSettings, CodexProgramStatus, PtyEvent, TerminalBackend, TerminalView};
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
    _bridge: Bridge,
    receiver: mpsc::Receiver<(u64, PtyEvent)>,
    status: Status,
    native: Vec<CodexProgramStatus>,
    output: PathBuf,
    badges: bool,
    stage: usize,
    changed: Instant,
    began: Instant,
    requested: bool,
    shots: usize,
    fixture_ready: bool,
    overview_seen: bool,
}
impl Probe {
    fn finish(&self, ctx: &egui::Context) {
        let ok = self.shots == 3
            && if self.badges {
                self.fixture_ready
            } else {
                self.overview_seen && self.native.contains(&CodexProgramStatus::Idle)
            };
        std::fs::write(
            self.output.join("result.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "ok": ok, "badge_fixture": self.badges, "screenshots": self.shots,
                "native_states": self.native.iter().map(|s| format!("{s:?}")).collect::<Vec<_>>(),
                "agents_entry_and_navigation": !self.badges && ok,
                "model_requests": false, "hook_trust_accepted": false,
                "shared_server_stopped": false, "pty_pid": self.terminal.pty_id(),
                "fixture_ready": self.fixture_ready, "overview_seen": self.overview_seen,
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
        for (_, event) in self.receiver.try_iter().take(128) {
            if let PtyEvent::Title(title) = event {
                for state in self.terminal.take_codex_program_statuses(&title) {
                    self.status.codex_program(state, true);
                    if self.native.len() < 64 {
                        self.native.push(state);
                    }
                }
            }
        }
        if self.began.elapsed() > Duration::from_secs(45) {
            self.finish(ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        input.events.push(egui::Event::PointerGone);
        if !self.badges
            && matches!(self.stage, 1 | 3)
            && self.changed.elapsed() > Duration::from_millis(700)
        {
            let key = if self.stage == 1 {
                egui::Key::ArrowLeft
            } else {
                egui::Key::Escape
            };
            for pressed in [true, false] {
                input.events.push(egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            self.stage += 1;
            self.changed = Instant::now();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui.label(if self.badges {
            "Mode badge raster check — synthetic text only"
        } else {
            "Native Codex lifecycle + agents navigation — no model prompt"
        });
        if !self.badges {
            ui.label(format!(
                "Native: {} · {:?}",
                self.status.native_lifecycle, self.status.activity
            ));
        }
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_focus(true)
            .set_font(appearance::terminal_font(14.0))
            .set_size(ui.available_size());
        ui.add(view);
        let text: String = self
            .terminal
            .last_content()
            .cells
            .iter()
            .map(|c| c.cell.c)
            .collect();
        self.fixture_ready |= self.badges && text.contains("EQUAL_MODE_BADGES");
        if !self.badges && self.stage == 2 {
            self.overview_seen |= [
                "Agent command center",
                "Agents",
                "New agent",
                "Recent chats",
                "Command center",
                "Command Center",
                "New chat",
                "Search agents",
            ]
            .iter()
            .any(|s| text.contains(s));
        }
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            let name = if self.badges {
                ["badges-100.png", "badges-146.png", "badges-200.png"][self.shots]
            } else {
                [
                    "codex-startup.png",
                    "agents-overview.png",
                    "codex-restored.png",
                ][self.shots]
            };
            let file = std::fs::File::create(self.output.join(name)).unwrap();
            let mut encoder = png::Encoder::new(
                std::io::BufWriter::new(file),
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
            self.changed = Instant::now();
            if self.shots == 3 {
                self.finish(&ctx);
                return;
            }
            if self.badges {
                ctx.set_zoom_factor([1.0, 1.46, 2.0][self.shots]);
            } else {
                self.stage += 1;
            }
        }
        let ready = if self.badges {
            (self.fixture_ready || self.began.elapsed() > Duration::from_secs(5))
                && self.changed.elapsed() > Duration::from_secs(1)
        } else {
            self.changed.elapsed() > Duration::from_secs(1)
                && match self.stage {
                    0 => {
                        text.contains("for agents")
                            && self.native.contains(&CodexProgramStatus::Idle)
                    }
                    2 => self.changed.elapsed() > Duration::from_secs(2),
                    4 => text.contains("for agents"),
                    _ => false,
                }
        };
        if ready && !self.requested {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    launch::prepare_gui_environment();
    let args: Vec<_> = std::env::args().collect();
    let badges = args
        .get(1)
        .expect("agents_status_probe codex|badges output-directory")
        == "badges";
    let output = PathBuf::from(args.get(2).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(!output.join("result.json").exists());
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal agents/status probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1160.0, 680.0])
                .with_transparent(true)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            ai_terminal::window_frame::apply_dark(cc);
            let context = cc.egui_ctx.clone();
            let bridge = Bridge::start(Arc::new(move || context.request_repaint()))?;
            let helper = std::env::current_exe()?
                .parent()
                .and_then(std::path::Path::parent)
                .unwrap()
                .join("ai-terminal-hook.exe");
            let spec = SessionSpec {
                id: "agents-probe".into(),
                name: "Codex".into(),
                directory: std::env::current_dir()?,
                agent: if badges { Agent::Shell } else { Agent::Codex },
                agent_session_id: None,
                color: None,
            };
            let mut plan = launch::build(&spec, &bridge, &output.join("runtime"), &helper)
                .map_err(std::io::Error::other)?;
            if badges {
                let script = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $e=[char]27; $lines=@('EQUAL_MODE_BADGES','','⏵⏵ auto mode on','▶▶ auto mode on','⏸ manual mode on','⏵ ⏵ isolated badges','▶︎▶︎ text presentation','你好 ✨ 👩‍💻'); [Console]::Write($e.ToString()+'[2J'+$e+'[H'+[string]::Join([Environment]::NewLine,$lines)); [Console]::WriteLine(''); Start-Sleep -Seconds 40";
                plan.executable = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
                    .join("System32/WindowsPowerShell/v1.0/powershell.exe")
                    .to_string_lossy()
                    .into_owned();
                plan.environment.clear();
                plan.arguments = [
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    script,
                ]
                .into_iter()
                .map(String::from)
                .collect();
            }
            let (tx, rx) = mpsc::channel();
            let terminal = TerminalBackend::new(
                776,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: plan.executable,
                    args: plan.arguments,
                    env: plan.environment,
                    working_directory: Some(spec.directory),
                },
            )?;
            terminal.set_codex_notifications(!badges);
            let job =
                platform::ProcessJob::attach(terminal.pty_id()).map_err(std::io::Error::other)?;
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                _job: job,
                _bridge: bridge,
                receiver: rx,
                status: Status::default(),
                native: vec![],
                output,
                badges,
                stage: 0,
                changed: Instant::now(),
                began: Instant::now(),
                requested: false,
                shots: 0,
                fixture_ready: false,
                overview_seen: false,
            }))
        }),
    )?;
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(verify.join("result.json")).expect("probe result"))
            .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    Ok(())
}
