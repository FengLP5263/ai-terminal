//! Resize a real, isolated ConPTY with previously painted full-row backgrounds.
use ai_terminal::{appearance, zoom};
use egui_term::{BackendCommand, BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

const CASES: [(f32, f32); 5] = [
    (1.14, 900.0),
    (0.97, 900.0),
    (0.97, 1400.0),
    (1.25, 800.0),
    (0.97, 1400.0),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    backend: TerminalBackend,
    _job: platform::ProcessJob,
    output: PathBuf,
    began: Instant,
    changed: Instant,
    stage: usize,
    frames: usize,
    sent: bool,
    pending: bool,
    checks: Vec<serde_json::Value>,
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
            self.began.elapsed() < Duration::from_secs(35),
            "resize probe timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(60));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= CASES.len() {
            return;
        }
        ui.label(format!(
            "Terminal resize · case {} · static history, no Agent",
            self.stage
        ));
        let view =
            TerminalView::new(ui, &mut self.backend).set_font(appearance::terminal_font(14.0));
        let rect = ui.add(view).rect;
        self.frames += 1;
        if !self.sent && self.frames >= 4 && self.changed.elapsed() > Duration::from_millis(500) {
            self.backend
                .process_command(BackendCommand::Write(b"r\r".to_vec()));
            self.sent = true;
        }
        let content = self.backend.sync();
        let cell_height = content.terminal_size.cell_height;
        let cell_width = content.terminal_size.cell_width;
        let size: egui_term::WindowSize = content.terminal_size.into();
        let inner_width = f32::from(size.num_cols) * cell_width;
        let row_line = |character| {
            content
                .cells
                .iter()
                .find(|cell| cell.point.column.0 == 0 && cell.cell.c == character)
                .map(|cell| cell.point.line.0 + content.display_offset as i32)
        };
        let background_line = row_line('>');
        let inline_line = row_line('I');
        if self.pending
            && let Some(image) = ui.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            let scale = ui.ctx().pixels_per_point();
            let inset = if self.stage == 0 {
                cell_width * 2.0
            } else {
                1.0
            };
            let x = ((rect.left() + inner_width - inset) * scale).floor() as usize;
            let y = ((rect.top() + cell_height * (background_line.unwrap() as f32 + 0.5)) * scale)
                .floor() as usize;
            let pixel = image.pixels[y * image.width() + x].to_array();
            let inline_y = ((rect.top() + cell_height * (inline_line.unwrap() as f32 + 0.5))
                * scale)
                .floor() as usize;
            let inline = image.pixels[inline_y * image.width() + x].to_array();
            assert_ne!(&inline[..3], &[65, 40, 85], "inline highlight expanded");
            let mut file =
                std::fs::File::create(self.output.join(format!("case-{}.png", self.stage)))
                    .unwrap();
            let mut encoder =
                png::Encoder::new(&mut file, image.width() as u32, image.height() as u32);
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
            assert_eq!(
                &pixel[..3],
                &[40, 65, 85],
                "old full-row background stops before right edge, case {}",
                self.stage
            );
            self.checks.push(serde_json::json!({"stage":self.stage,"zoom":CASES[self.stage].0,"columns":size.num_cols,"rows":size.num_lines,"background_reaches_right_edge":true,"inline_highlight_kept_small":true,"pty_pid":self.backend.pty_id()}));
            self.stage += 1;
            self.frames = 0;
            self.pending = false;
            self.changed = Instant::now();
            if self.stage == CASES.len() {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"checks":self.checks,"model_requests":false,"user_data":false,"terminal_input":"one local fixture trigger; no input during resizing"})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().set_zoom_factor(CASES[self.stage].0);
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        CASES[self.stage].1,
                        650.0,
                    )));
            }
        } else if !self.pending
            && self.frames >= 6
            && self.changed.elapsed() > Duration::from_millis(800)
            && self.backend.plain_text().contains("WIDTH_BACKGROUND_READY")
        {
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("terminal_resize_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "AI Terminal resize acceptance",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([CASES[0].1, 650.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            zoom::initialize(&cc.egui_ctx, CASES[0].0);
            let (tx, _) = mpsc::channel();
            let backend = TerminalBackend::new(95081,cc.egui_ctx.clone(),tx,BackendSettings {
            shell:"powershell.exe".into(),
            args:vec!["-NoLogo".into(),"-NoProfile".into(),"-NonInteractive".into(),"-Command".into(),
                "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); [void][Console]::ReadLine(); $e=[char]27; $w=[Console]::WindowWidth; $s=('> input background original width='+$w).PadRight($w-1); [Console]::Write($e+'[2J'+$e+'[H'+$e+'[48;2;40;65;85m'+$s+$e+'[0m'+$e+'[3;1H'+$e+'[48;2;65;40;85m'+'INLINE ONLY'+$e+'[0m'+$e+'[5;1H'+'中文 👩‍💻 · hard line / code = 1;'+$e+'[7;1H'+'WIDTH_BACKGROUND_READY'); Start-Sleep -Seconds 30".into()],
            ..Default::default()
        }).unwrap();
            let job = platform::ProcessJob::attach(backend.pty_id()).unwrap();
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                backend,
                _job: job,
                output,
                began: Instant::now(),
                changed: Instant::now(),
                stage: 0,
                frames: 0,
                sent: false,
                pending: false,
                checks: vec![],
            }))
        }),
    )
}
