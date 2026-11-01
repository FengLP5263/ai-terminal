//! Native, model-free fullscreen scroll + status-help visual acceptance.
use ai_terminal::{appearance, connection_ui, model::Agent, status::Status};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;
struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    _job: platform::ProcessJob,
    output: PathBuf,
    began: Instant,
    stage: usize,
    sent: bool,
    released: bool,
    pending: bool,
    terminal_rect: egui::Rect,
    status_rect: egui::Rect,
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
        if self.began.elapsed() > Duration::from_secs(20) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .focused = Some(true);
        if (self.stage == 1 || self.stage == 2) && !self.sent {
            input
                .events
                .push(egui::Event::PointerMoved(self.terminal_rect.center()));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, if self.stage == 1 { 3.0 } else { -3.0 }),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            });
            self.sent = true;
        } else if self.stage == 3 && !self.released {
            let pos = self.status_rect.center();
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: !self.sent,
                modifiers: egui::Modifiers::NONE,
            });
            if self.sent {
                self.released = true;
            }
            self.sent = true;
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.horizontal(|ui| {
            ui.heading("全屏会话滚轮核查 · 本地样例");
            self.status_rect = connection_ui::show(ui, Agent::Codex, &Status::default()).rect;
        });
        ui.label("滚轮向上查看旧消息，向下返回；状态说明只提供查看命令。");
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_focus(self.stage < 3)
            .set_font(appearance::terminal_font(14.0));
        self.terminal_rect = ui.add(view).rect;
        let ctx = ui.ctx();
        if self.pending
            && let Some(image) = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            let names = ["before.png", "up.png", "down.png", "connection.png"];
            let file = std::fs::File::create(self.output.join(names[self.stage])).unwrap();
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
            self.stage += 1;
            self.sent = false;
            self.pending = false;
            if self.stage == 4 {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"fullscreen_mouse_scroll_verified":true,"connection_popup_opened":true,"model_request_sent":false,"pty_pid":self.terminal.pty_id()})).unwrap()).unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        let ready = match self.stage {
            0 | 2 => self.terminal.plain_text().contains("history offset 100"),
            1 => self.terminal.plain_text().contains("history offset 85"),
            3 => egui::Popup::is_any_open(ctx),
            _ => false,
        };
        if !self.pending && ready && self.began.elapsed() > Duration::from_secs(1) {
            self.pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
}
fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().collect();
    let binary = PathBuf::from(
        args.get(1)
            .expect("scroll_probe fixture.exe output-directory"),
    );
    let output = PathBuf::from(args.get(2).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal scroll acceptance",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default().with_inner_size([1160.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let (tx, _) = mpsc::channel();
            let terminal = TerminalBackend::new(
                995,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: binary.to_string_lossy().into_owned(),
                    args: vec!["pager".into()],
                    ..Default::default()
                },
            )?;
            let job = platform::ProcessJob::attach(terminal.pty_id())?;
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                _job: job,
                output,
                began: Instant::now(),
                stage: 0,
                sent: false,
                released: false,
                pending: false,
                terminal_rect: egui::Rect::NOTHING,
                status_rect: egui::Rect::NOTHING,
            }))
        }),
    )
}
