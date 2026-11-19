//! Large normal-buffer output, real scroll gestures and final-row visibility; no Agent/model.
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

const SCALES: [f32; 5] = [1.0, 1.46, 1.46, 0.97, 1.46];
struct Probe {
    renderer: egui_term::TextRendererSession,
    backend: TerminalBackend,
    _job: platform::ProcessJob,
    output: PathBuf,
    began: Instant,
    changed: Instant,
    stage: usize,
    frames: usize,
    triggered: bool,
    scrolled: bool,
    pending: bool,
    terminal_rect: Option<egui::Rect>,
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
            self.began.elapsed() < Duration::from_secs(40),
            "output/scroll probe timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(60));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .events
            .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
        if (self.stage == 1 || self.stage == 2) && self.frames >= 6 && !self.scrolled {
            let rect = self.terminal_rect.unwrap();
            input.events.push(egui::Event::PointerMoved(rect.center()));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, if self.stage == 1 { 160.0 } else { -10000.0 }),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            });
            self.scrolled = true;
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == SCALES.len() {
            return;
        }
        ui.label(format!(
            "Large output / local scroll / {:.0}% — no Agent or refresh key",
            SCALES[self.stage] * 100.0
        ));
        let clip = ui.clip_rect();
        let view = TerminalView::new(ui, &mut self.backend)
            .set_focus(true)
            .request_focus(!self.triggered)
            .set_font(appearance::terminal_font(14.0));
        let rect = ui.add(view).rect;
        self.terminal_rect = Some(rect);
        self.frames += 1;
        if !self.triggered
            && self.frames >= 4
            && self.changed.elapsed() > Duration::from_millis(400)
        {
            self.backend
                .process_command(BackendCommand::Write(b"r\r".to_vec()));
            self.triggered = true;
        }
        let content = self.backend.sync();
        let size: egui_term::WindowSize = content.terminal_size.into();
        let text: String = content.cells.iter().map(|cell| cell.cell.c).collect();
        let marker_visible = text.contains("LIVE_COMPOSER_READY");
        let history = content.history_size;
        let offset = content.display_offset;
        let cursor_row = content.cursor_point.line.0 + offset as i32;
        let cell_height = content.terminal_size.cell_height;
        let cell_width = content.terminal_size.cell_width;
        let cursor_bottom = rect.top() + (cursor_row + 1) as f32 * cell_height;
        let expected = if self.stage == 1 {
            self.scrolled && offset > 0 && !marker_visible
        } else {
            marker_visible
                && offset == 0
                && cursor_row >= 0
                && cursor_row < i32::from(size.num_lines)
                && cursor_bottom <= clip.bottom() + 0.5
                && rect.bottom() <= clip.bottom() + 0.5
        };
        if self.pending
            && let Some(image) = ui.input(|i| {
                i.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            assert!(
                expected,
                "current composer was lost after scrolling/resizing"
            );
            let scale = ui.ctx().pixels_per_point();
            let mut ink = 0;
            if self.stage != 1 {
                let left = (rect.left() * scale) as usize;
                let top = ((rect.top() + cursor_row as f32 * cell_height) * scale) as usize;
                let right = ((rect.left() + 28.0 * cell_width) * scale) as usize;
                let bottom = (cursor_bottom * scale) as usize;
                for y in top..bottom.min(image.height()) {
                    for x in left..right.min(image.width()) {
                        if image.pixels[y * image.width() + x].to_array()[..3]
                            .iter()
                            .all(|c| *c > 100)
                        {
                            ink += 1;
                        }
                    }
                }
                assert!(
                    ink > 64,
                    "the live composer exists in the grid but is clipped in the screenshot"
                );
            }
            let mut encoder = png::Encoder::new(
                std::fs::File::create(self.output.join(format!("stage-{}.png", self.stage)))
                    .unwrap(),
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
            self.checks.push(serde_json::json!({"stage":self.stage,"zoom":SCALES[self.stage],"rows":size.num_lines,"columns":size.num_cols,"history_size":history,"display_offset":offset,"composer_visible":marker_visible,"cursor_row":cursor_row,"cursor_bottom":cursor_bottom,"clip_bottom":clip.bottom(),"composer_ink":ink,"pty_pid":self.backend.pty_id()}));
            self.stage += 1;
            self.frames = 0;
            self.scrolled = false;
            self.pending = false;
            self.changed = Instant::now();
            if self.stage == SCALES.len() {
                std::fs::write(self.output.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"checks":self.checks,"model_requests":false,"user_data":false,"input":"one fixture trigger only; wheel gestures do not enter the PTY"})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().set_zoom_factor(SCALES[self.stage]);
            }
        } else if !self.pending
            && expected
            && self.frames >= 9
            && self.changed.elapsed() > Duration::from_millis(900)
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
            .expect("terminal_output_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "AI Terminal output / scroll probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([1100.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            zoom::initialize(&cc.egui_ctx, SCALES[0]);
            let (tx, _) = mpsc::channel();
            let script = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $null=[Console]::ReadLine(); $lines=1..2048|ForEach-Object {'HISTORY_'+$_+(' long output 中英混排 👩🏽‍💻 '+('x'*160))}; [Console]::Write([string]::Join(\"`r`n\",$lines)); [Console]::WriteLine(); [Console]::Write('> LIVE_COMPOSER_READY'); Start-Sleep -Seconds 60";
            let backend = TerminalBackend::new(
                7331,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: "powershell.exe".into(),
                    args: vec![
                        "-NoLogo".into(),
                        "-NoProfile".into(),
                        "-Command".into(),
                        script.into(),
                    ],
                    ..Default::default()
                },
            )?;
            let job = platform::ProcessJob::attach(backend.pty_id())?;
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                backend,
                _job: job,
                output,
                began: Instant::now(),
                changed: Instant::now(),
                stage: 0,
                frames: 0,
                triggered: false,
                scrolled: false,
                pending: false,
                terminal_rect: None,
                checks: vec![],
            }))
        }),
    )
}
