//! Fullscreen final-row/cursor regression at restored and changing UI zoom, without a model.
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
const SCALES: [f32; 4] = [1.511_360_8, 1.25, 1.5, 1.511_360_8];
struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    _job: platform::ProcessJob,
    output: PathBuf,
    began: Instant,
    stage_began: Instant,
    stage: usize,
    sent: bool,
    pending: bool,
    frames: usize,
    good_frames: usize,
    checks: Vec<serde_json::Value>,
    latest: serde_json::Value,
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
        if self.began.elapsed() > Duration::from_secs(25) {
            std::fs::write(self.output.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"ok":false,"stage":self.stage,"checks":self.checks,"latest":self.latest,"model_request_sent":false})).unwrap()).unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(60));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .focused = Some(true);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= SCALES.len() {
            return;
        }
        ui.label(format!(
            "Fullscreen composer · {:.2}% · synthetic completed response",
            SCALES[self.stage] * 100.0
        ));
        let clip = ui.clip_rect();
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_focus(true)
            .set_font(appearance::terminal_font(14.0));
        let rect = ui.add(view).rect;
        self.frames += 1;
        if !self.sent && self.frames >= 4 && self.stage_began.elapsed() > Duration::from_millis(300)
        {
            self.terminal
                .process_command(BackendCommand::Write(vec![b'r']));
            self.sent = true;
        }
        let content = self.terminal.sync();
        let size: egui_term::WindowSize = content.terminal_size.into();
        let row_text = |line: i32| {
            content
                .cells
                .iter()
                .filter(|cell| cell.point.line.0 == line)
                .map(|cell| cell.cell.c)
                .collect::<String>()
        };
        let composer = row_text(i32::from(size.num_lines) - 2);
        let status = row_text(i32::from(size.num_lines) - 1);
        let good = composer.contains("COMPOSER> type here")
            && status.contains("STATUS: READY")
            && content.display_offset == 0
            && rect.bottom() <= clip.bottom() + 0.5;
        self.good_frames = if good { self.good_frames + 1 } else { 0 };
        self.latest = serde_json::json!({"scale":SCALES[self.stage],"rows":size.num_lines,"columns":size.num_cols,"cell_width":content.terminal_size.cell_width,"cell_height":content.terminal_size.cell_height,"pixels_per_point":ui.ctx().pixels_per_point(),"composer_visible":composer.contains("COMPOSER> type here"),"status_visible":status.contains("STATUS: READY"),"display_offset":content.display_offset,"terminal_bottom":rect.bottom(),"clip_bottom":clip.bottom(),"terminal_mode":format!("{:?}",content.terminal_mode)});
        if self.pending
            && let Some(image) = ui.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            // The backend can finish a response after TerminalView painted this
            // frame. Validate captured pixels as well as the next grid snapshot.
            let bottom = (rect.bottom() * ui.ctx().pixels_per_point()) as usize;
            let from_y = bottom.saturating_sub(
                (content.terminal_size.cell_height * ui.ctx().pixels_per_point() * 2.0) as usize,
            );
            let to_x =
                (32.0 * content.terminal_size.cell_width * ui.ctx().pixels_per_point()) as usize;
            let ink = (from_y..bottom.min(image.height()))
                .flat_map(|row| (0..to_x.min(image.width())).map(move |column| (row, column)))
                .filter(|&(row, column)| {
                    image.pixels[row * image.width() + column].to_array()[..3]
                        .iter()
                        .all(|&c| c > 96)
                })
                .count();
            self.pending = false;
            if ink < 64 {
                self.good_frames = 0;
                return;
            }
            self.latest["captured_final_row_ink"] = serde_json::json!(ink);
            let file = std::fs::File::create(self.output.join(format!("stage-{}.png", self.stage)))
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
                        .flat_map(|p| p.to_array())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            self.checks.push(self.latest.clone());
            self.stage += 1;
            self.pending = false;
            self.sent = false;
            self.frames = 0;
            self.good_frames = 0;
            self.stage_began = Instant::now();
            if self.stage == SCALES.len() {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"checks":self.checks,"model_request_sent":false,"pty_pid":self.terminal.pty_id()})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().set_zoom_factor(SCALES[self.stage]);
            }
        } else if self.sent && self.good_frames >= 3 && !self.pending {
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().collect();
    let fixture = PathBuf::from(
        args.get(1)
            .expect("composer_probe fixture.exe output-directory"),
    );
    let output = PathBuf::from(args.get(2).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal composer acceptance",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_transparent(true)
                .with_inner_size([1160.0, 760.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            zoom::initialize(&cc.egui_ctx, SCALES[0]);
            let (tx, _) = mpsc::channel();
            let terminal = TerminalBackend::new(
                998,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: fixture.to_string_lossy().into_owned(),
                    args: vec!["composer".into()],
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
                stage_began: Instant::now(),
                stage: 0,
                sent: false,
                pending: false,
                frames: 0,
                good_frames: 0,
                checks: vec![],
                latest: serde_json::Value::Null,
            }))
        }),
    )
}
