//! Native local scrollback drag captures; only this probe's own no-profile PTY/input.
use ai_terminal::appearance;
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
const SCALES: [f32; 3] = [1.0, 1.03, 2.0];
struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    output: PathBuf,
    began: Instant,
    stage: usize,
    frames: usize,
    input_step: u8,
    pending: bool,
    rect: Option<egui::Rect>,
    records: Vec<serde_json::Value>,
    pid: u32,
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
        assert!(
            self.began.elapsed() < Duration::from_secs(45),
            "history probe timeout"
        );
        ctx.request_repaint_after(Duration::from_millis(65));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        if self.stage >= 9
            || self.frames < 5
            || self.terminal.sync().history_size < 200
            || self.input_step >= 3
        {
            return;
        }
        let Some(rect) = self.rect else {
            return;
        };
        let y = match self.stage % 3 {
            0 => rect.top() + 2.0,
            1 => rect.center().y,
            _ => rect.bottom() - 2.0,
        };
        let mut pos = egui::pos2(rect.right() - 6.0, y);
        if self.input_step == 1 {
            pos.y = match self.stage % 3 {
                0 => rect.top() - 24.0,
                1 => y,
                _ => rect.bottom() + 24.0,
            };
        }
        input.events.push(egui::Event::PointerMoved(pos));
        if self.input_step != 1 {
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: self.input_step == 0,
                modifiers: egui::Modifiers::NONE,
            });
        }
        self.input_step += 1;
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= 9 {
            return;
        }
        let scale = SCALES[self.stage / 3];
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(scale);
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                ui.label(format!(
                    "Local terminal history · {}% · {}",
                    (scale * 100.0).round(),
                    ["oldest", "middle", "latest"][self.stage % 3]
                ));
                ui.separator();
                let view = TerminalView::new(ui, &mut self.terminal)
                    .set_focus(true)
                    .set_font(appearance::current_terminal_font(ui.ctx(), 14.0));
                self.rect = Some(ui.add(view).rect);
            });
        self.frames += 1;
        assert_eq!(self.terminal.pty_id(), self.pid);
        let screenshot = ui.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if self.pending
            && let Some(image) = screenshot
        {
            let name = format!(
                "history-{}-{}.png",
                (scale * 100.0).round(),
                ["oldest", "middle", "latest"][self.stage % 3]
            );
            let mut encoder = png::Encoder::new(
                std::fs::File::create(self.output.join(&name)).unwrap(),
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
            let c = self.terminal.sync();
            let size: egui_term::WindowSize = c.terminal_size.into();
            self.records.push(serde_json::json!({"image":name,"scale":scale,"offset":c.display_offset,"history":c.history_size,"columns":size.num_cols,"rows":size.num_lines}));
            self.stage += 1;
            self.frames = 0;
            self.input_step = 0;
            self.pending = false;
            if self.stage == 9 {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":self.records,"pty_pid":self.pid,"model_requests":false})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if !self.pending && self.frames > 10 && self.input_step == 3 {
            let c = self.terminal.sync();
            let ok = match self.stage % 3 {
                0 => c.display_offset == c.history_size,
                1 => {
                    c.display_offset > c.history_size / 4
                        && c.display_offset < c.history_size * 3 / 4
                }
                _ => c.display_offset == 0,
            };
            assert!(
                ok,
                "scroll position {} / {}",
                c.display_offset, c.history_size
            );
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal terminal history acceptance",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1000.0, 640.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let (tx, _) = mpsc::channel();
            let terminal=TerminalBackend::new(1270,cc.egui_ctx.clone(),tx,BackendSettings{shell:"powershell.exe".into(),args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); Start-Sleep -Milliseconds 500; 0..399 | ForEach-Object {[Console]::WriteLine(('HISTORY_LINE_{0:D4} | Terminal history / 终端历史内容' -f $_))}; [Console]::WriteLine('HISTORY_READY'); Start-Sleep -Seconds 55".into()],..Default::default()})?;
            let pid = terminal.pty_id();
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                output,
                began: Instant::now(),
                stage: 0,
                frames: 0,
                input_step: 0,
                pending: false,
                rect: None,
                records: vec![],
                pid,
            }))
        }),
    )
}
