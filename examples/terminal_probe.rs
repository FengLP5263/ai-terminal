//! Bounded developer check: real agent TUI, synthetic text input, no Enter/model request.
use ai_terminal::{launch, model::Agent};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

#[path = "../src/platform.rs"]
mod platform;

struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    _job: platform::ProcessJob,
    began: Instant,
    input_stage: u8,
    ready_at: Option<Instant>,
    screenshot: bool,
    output: PathBuf,
    shell_pending: bool,
    through_shell: bool,
    bridge: Option<ai_terminal::bridge::Bridge>,
    managed_events: Vec<String>,
    paste_image: bool,
    paste_sent: bool,
    startup_requested: bool,
    startup_saved: bool,
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
        if self.shell_pending && self.terminal.plain_text().contains("PS ") {
            self.terminal
                .process_command(egui_term::BackendCommand::Write(
                    b"codex -c check_for_update_on_startup=false\r".to_vec(),
                ));
            self.shell_pending = false;
        }
        if let Some(bridge) = &self.bridge {
            for event in bridge.receiver.try_iter().take(64) {
                if self.managed_events.len() < 32 {
                    self.managed_events.push(format!(
                        "{}:{}",
                        event.name,
                        event.source.as_deref().unwrap_or("")
                    ));
                }
            }
        }
        if self.began.elapsed() > Duration::from_secs(60) {
            let _ = std::fs::write(
                self.output.join("timeout.json"),
                "{\"ui_capture_timeout\":true}",
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        // Synthetic input uses a focused viewport without activating the OS window.
        input.focused = true;
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .focused = Some(true);
        if self.ready_at.is_none() {
            let text = self.terminal.plain_text();
            if (text.contains("Claude Code") || text.contains("OpenAI Codex"))
                && (text.contains("❯")
                    || (text.contains("Ask Codex")
                        && text
                            .lines()
                            .any(|line| line.contains("GPT-") || line.contains("gpt-"))))
            {
                self.ready_at = Some(Instant::now());
            }
        }
        if self.paste_image {
            if self.startup_saved
                && !self.paste_sent
                && self
                    .ready_at
                    .is_some_and(|t| t.elapsed() > Duration::from_millis(1200))
            {
                input.events.push(egui::Event::Paste(String::new()));
                self.paste_sent = true;
            }
            return;
        }
        let elapsed = self
            .ready_at
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let next = if elapsed > 1.5 {
            3
        } else if elapsed > 1.0 {
            2
        } else if elapsed > 0.5 {
            1
        } else {
            0
        };
        if next > self.input_stage {
            input.events.push(match next {
                1 => egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: "zhongwen".into(),
                    active_range_chars: None,
                }),
                2 => egui::Event::Ime(egui::ImeEvent::Commit("中文输入".into())),
                _ => egui::Event::Text(" abc123".into()),
            });
            self.input_stage = next;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.heading("Agent 终端交互核查");
        ui.label(if self.through_shell {
            "Shell 中只提交启动命令；Agent 中只输入文本，不提交模型请求。"
        } else {
            "测试只操作未发送的输入，不发送 Enter，不请求模型。"
        });
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_native_image_paste(true)
            .set_drag_selection(true)
            .set_focus(true)
            .set_font(ai_terminal::appearance::terminal_font(14.0));
        ui.add(view);
        let ctx = ui.ctx();
        if self.paste_image
            && !self.startup_requested
            && self
                .ready_at
                .is_some_and(|t| t.elapsed() > Duration::from_millis(350))
        {
            self.startup_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if self.ready_at.is_some_and(|t| {
            t.elapsed() > Duration::from_secs(if self.paste_image { 15 } else { 3 })
        }) && (self.paste_image && self.paste_sent
            || self.terminal.plain_text().contains("中文输入 abc123"))
            && !self.screenshot
        {
            self.screenshot = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            std::fs::create_dir_all(&self.output).unwrap();
            let startup = self.paste_image && !self.startup_saved;
            let file = std::fs::File::create(self.output.join(if startup {
                "startup.png"
            } else {
                "terminal.png"
            }))
            .unwrap();
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
                        .flat_map(|c| c.to_array())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            if startup {
                self.startup_saved = true;
                return;
            }
            let text = self.terminal.plain_text();
            let snapshot = self.terminal.sync();
            let colored_cells = snapshot
                .cells
                .iter()
                .filter(|c| c.cell.c != ' ' && !format!("{:?}", c.cell.fg).contains("Foreground"))
                .count();
            let report = serde_json::json!({"text_input_visible":text.contains("中文输入 abc123"),"colored_cells":colored_cells,"image_paste_sent":self.paste_sent,"image_attachment_marker":text.contains("Image #1")||text.contains("Image 1")||text.contains("image 1")||text.contains("1 image"),"terminal_mode":format!("{:?}",snapshot.terminal_mode),"enter_sent":false,"shell_command_enter_sent":self.through_shell,"managed_events":self.managed_events,"model_request_sent":false,"text":text,"pty_pid":self.terminal.pty_id(),"manager_resident_bytes":platform::resident_bytes()});
            std::fs::write(
                self.output.join("result.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.began.elapsed() > Duration::from_secs(55) {
            let _ = std::fs::write(
                self.output.join("startup-text.txt"),
                self.terminal.plain_text(),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result {
    launch::prepare_gui_environment();
    let args: Vec<_> = std::env::args().collect();
    let through_shell = args.get(1).is_some_and(|mode| mode == "shell-codex");
    let paste_image = args.get(1).is_some_and(|s| s.ends_with("-image"));
    let agent = if args.get(1).is_some_and(|s| s.starts_with("claude")) {
        Agent::Claude
    } else {
        Agent::Codex
    };
    let output = PathBuf::from(
        args.get(2)
            .expect("terminal_probe codex|shell-codex|claude|claude-manual output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("phase.txt"), "launch renderer").unwrap();
    let verify = output.clone();
    let manual_mode = args.get(1).is_some_and(|mode| mode == "claude-manual");
    eframe::run_native(
        "ai-terminal TUI check",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(ai_terminal::appearance::window_icon())
                .with_inner_size([1160.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            std::fs::write(output.join("phase.txt"), "renderer ready").unwrap();
            ai_terminal::appearance::setup(&cc.egui_ctx);
            let (sender, _) = mpsc::channel();
            std::fs::write(output.join("phase.txt"), "create PTY").unwrap();
            let binary = launch::executable(agent).expect("installed agent");
            let helper = std::env::current_exe()?
                .parent()
                .and_then(std::path::Path::parent)
                .expect("example output directory")
                .join("ai-terminal-hook.exe");
            let cli_args = if agent == Agent::Codex {
                vec![
                    "--no-daemon".into(),
                    "-c".into(),
                    "check_for_update_on_startup=false".into(),
                ]
            } else if manual_mode {
                vec!["--permission-mode".into(), "default".into()]
            } else {
                vec![]
            };
            let (mut program, mut cli_args) =
                launch::pty_command(&binary, cli_args, &helper).expect("prepare installed agent");
            let mut environment = std::collections::HashMap::from([
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
                ("CLAUDECODE".into(), String::new()),
            ]);
            let mut shell_bridge = None;
            if through_shell {
                let context = cc.egui_ctx.clone();
                let bridge = ai_terminal::bridge::Bridge::start(std::sync::Arc::new(move || {
                    context.request_repaint()
                }))?;
                let spec = ai_terminal::model::SessionSpec {
                    id: "native-shell-probe".into(),
                    name: "PowerShell".into(),
                    directory: std::env::current_dir()?,
                    agent: Agent::Shell,
                    agent_session_id: None,
                    color: None,
                };
                let plan = launch::build(&spec, &bridge, &output.join("runtime"), &helper)
                    .map_err(std::io::Error::other)?;
                program = plan.executable;
                cli_args = plan.arguments;
                environment = plan.environment;
                shell_bridge = Some(bridge);
            }
            let terminal = TerminalBackend::new(
                901,
                cc.egui_ctx.clone(),
                sender,
                BackendSettings {
                    shell: program,
                    args: cli_args,
                    working_directory: Some(std::env::current_dir()?),
                    env: environment,
                },
            )?;
            std::fs::write(output.join("phase.txt"), "PTY ready").unwrap();
            let job = platform::ProcessJob::attach(terminal.pty_id())?;
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                _job: job,
                began: Instant::now(),
                input_stage: 0,
                ready_at: None,
                screenshot: false,
                output,
                shell_pending: through_shell,
                through_shell,
                bridge: shell_bridge,
                managed_events: Vec::new(),
                paste_image,
                paste_sent: false,
                startup_requested: false,
                startup_saved: false,
            }))
        }),
    )?;
    let report = std::fs::read(verify.join("result.json")).expect(
        "TUI capture incomplete: inspect startup-text.txt / trust prompt; no automatic approval",
    );
    let report: serde_json::Value = serde_json::from_slice(&report).expect("TUI report");
    assert_eq!(report["model_request_sent"], false);
    assert!(
        verify.join("terminal.png").is_file(),
        "TUI screenshot missing"
    );
    Ok(())
}
