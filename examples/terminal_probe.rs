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
        if self.began.elapsed() > Duration::from_secs(40) {
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
            if text.contains("Claude Code") || text.contains("OpenAI Codex") {
                self.ready_at = Some(Instant::now());
            }
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
        ui.label("测试仅输入文本，不发送 Enter，不请求模型。");
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_focus(true)
            .set_font(ai_terminal::appearance::terminal_font(14.0));
        ui.add(view);
        let ctx = ui.ctx();
        if self
            .ready_at
            .is_some_and(|t| t.elapsed() > Duration::from_secs(3))
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
            let file = std::fs::File::create(self.output.join("terminal.png")).unwrap();
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
            let text = self.terminal.plain_text();
            let snapshot = self.terminal.sync();
            let colored_cells = snapshot
                .cells
                .iter()
                .filter(|c| c.cell.c != ' ' && !format!("{:?}", c.cell.fg).contains("Foreground"))
                .count();
            let report = serde_json::json!({"text_input_visible":text.contains("中文输入 abc123"),"colored_cells":colored_cells,"enter_sent":false,"model_request_sent":false,"text":text,"pty_pid":self.terminal.pty_id(),"manager_resident_bytes":platform::resident_bytes()});
            std::fs::write(
                self.output.join("result.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.began.elapsed() > Duration::from_secs(35) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result {
    launch::prepare_gui_environment();
    let args: Vec<_> = std::env::args().collect();
    let agent = if args.get(1).is_some_and(|s| s == "claude") {
        Agent::Claude
    } else {
        Agent::Codex
    };
    let output = PathBuf::from(
        args.get(2)
            .expect("terminal_probe codex|claude output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("phase.txt"), "launch renderer").unwrap();
    eframe::run_native(
        "ai-terminal TUI check",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default().with_inner_size([1160.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            std::fs::write(output.join("phase.txt"), "renderer ready").unwrap();
            ai_terminal::appearance::setup(&cc.egui_ctx);
            let (sender, _) = mpsc::channel();
            std::fs::write(output.join("phase.txt"), "create PTY").unwrap();
            let terminal = TerminalBackend::new(
                901,
                cc.egui_ctx.clone(),
                sender,
                BackendSettings {
                    shell: launch::executable(agent)
                        .expect("installed agent")
                        .to_string_lossy()
                        .into_owned(),
                    args: if agent == Agent::Codex {
                        vec![
                            "-c".into(),
                            "check_for_update_on_startup=false".into(),
                            "--no-alt-screen".into(),
                        ]
                    } else {
                        vec![]
                    },
                    working_directory: Some(std::env::current_dir()?),
                    env: std::collections::HashMap::from([
                        ("TERM".into(), "xterm-256color".into()),
                        ("COLORTERM".into(), "truecolor".into()),
                        ("CLAUDECODE".into(), String::new()),
                    ]),
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
            }))
        }),
    )
}
