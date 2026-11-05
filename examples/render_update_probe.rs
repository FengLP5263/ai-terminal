//! Pixel stability under changing glyph/mesh uploads. Own local fixtures only.
use ai_terminal::appearance;
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Probe {
    renderer: egui_term::TextRendererSession,
    subject: TerminalBackend,
    noise: TerminalBackend,
    output: PathBuf,
    began: Instant,
    stage: usize,
    frames: usize,
    requested: bool,
    baseline: Option<Vec<egui::Color32>>,
    grid_hash: Option<u64>,
    captures: usize,
    scale: f32,
    crop: [usize; 4],
    checks: Vec<serde_json::Value>,
    finished: bool,
}
fn save(path: &Path, image: &egui::ColorImage) {
    let mut png = png::Encoder::new(
        std::fs::File::create(path).unwrap(),
        image.width() as u32,
        image.height() as u32,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(
            &image
                .pixels
                .iter()
                .flat_map(|p| p.to_array())
                .collect::<Vec<_>>(),
        )
        .unwrap();
}
impl Probe {
    fn finish(&mut self, ctx: &egui::Context) {
        if self.finished {
            return;
        }
        self.finished = true;
        std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "ok":self.checks.len()==self.captures&&self.checks.iter().all(|c|c["changed_pixels"]==0&&c["grid_unchanged"]==true),
            "checks":self.checks,"model_requests":false,"fixture_only":true,
            "pty_pids":[self.subject.pty_id(),self.noise.pty_id()],"elapsed_ms":self.began.elapsed().as_millis(),
            "version":env!("CARGO_PKG_VERSION"),
            "scale":self.scale,
            "floating_hint_checked_separately":true,
        })).unwrap()).unwrap();
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
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
        if self.began.elapsed() > Duration::from_secs(120) {
            self.finish(ctx);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.finished || ctx.viewport_id() != egui::ViewportId::ROOT {
            return;
        }
        let area = ctx.content_rect();
        let subject_rect =
            egui::Rect::from_min_size(egui::pos2(12., 36.), egui::vec2(area.width() - 24., 320.));
        let noise_rect =
            egui::Rect::from_min_max(egui::pos2(12., 380.), area.max - egui::vec2(12., 12.));
        ui.label(format!(
            "Fixed table / changing glyph uploads · {}",
            self.stage
        ));
        let mut subject_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("subject")
                .max_rect(subject_rect),
        );
        let subject_view = TerminalView::new(&mut subject_ui, &mut self.subject)
            .set_font(appearance::terminal_font(14.));
        let response = subject_ui.add(subject_view);
        let mut noise_ui =
            ui.new_child(egui::UiBuilder::new().id_salt("noise").max_rect(noise_rect));
        let size = [9., 11., 13., 15., 17., 19., 21., 23.][self.stage % 8];
        let noise_view = TerminalView::new(&mut noise_ui, &mut self.noise)
            .set_font(appearance::terminal_font(size));
        noise_ui.add(noise_view);
        let ready = self.subject.plain_text().contains("FIXED_TABLE_READY");
        self.frames += 1;
        if self.requested
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
            let [x, y, w, h] = self.crop;
            let mut pixels = Vec::with_capacity(w * h);
            for row in y..y + h {
                pixels.extend_from_slice(
                    &image.pixels[row * image.width() + x..row * image.width() + x + w],
                );
            }
            let changed = self.baseline.as_ref().map_or(0, |baseline| {
                baseline.iter().zip(&pixels).filter(|(a, b)| a != b).count()
            });
            if self.baseline.is_none() {
                assert!(
                    pixels.iter().filter(|p| p.r() > 60).count() > 300,
                    "blank fixed sample"
                );
                self.baseline = Some(pixels);
            }
            if self.stage == 0
                || self.stage == self.captures / 2
                || self.stage + 1 == self.captures
                || (changed > 0 && self.checks.iter().all(|c| c["changed_pixels"] == 0))
            {
                save(
                    &self.output.join(format!("frame-{}.png", self.stage)),
                    &image,
                );
            }
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            self.subject.plain_text().hash(&mut hash);
            let hash = hash.finish();
            let grid_unchanged = *self.grid_hash.get_or_insert(hash) == hash;
            self.checks.push(serde_json::json!({"stage":self.stage,"changed_pixels":changed,"noise_size":size,"grid_unchanged":grid_unchanged}));
            self.stage += 1;
            self.frames = 0;
            self.requested = false;
            if self.stage == self.captures {
                self.finish(&ctx);
                return;
            }
        }
        if ready && !self.requested && self.frames >= if self.stage == 0 { 4 } else { 2 } {
            let scale = ctx.pixels_per_point();
            let r = response.rect;
            self.crop = [
                (r.min.x * scale).round() as usize,
                (r.min.y * scale).round() as usize,
                (r.width() * scale).floor() as usize,
                (r.height() * scale).floor() as usize,
            ];
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint();
    }
}
fn terminal(ctx: &egui::Context, id: u64, text: &Path, script: &Path) -> TerminalBackend {
    let (tx, _) = mpsc::channel();
    TerminalBackend::new(
        id,
        ctx.clone(),
        tx,
        BackendSettings {
            shell: "powershell.exe".into(),
            args: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-File".into(),
                script.to_string_lossy().into(),
                text.to_string_lossy().into(),
            ],
            ..Default::default()
        },
    )
    .unwrap()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("render_update_probe output-directory [scale] [captures]"),
    );
    let scale = std::env::args()
        .nth(2)
        .map(|s| s.parse::<f32>().expect("numeric scale"))
        .unwrap_or(1.03);
    assert!((0.5..=2.5).contains(&scale));
    let captures = std::env::args()
        .nth(3)
        .map(|s| s.parse::<usize>().expect("capture count"))
        .unwrap_or(48);
    assert!((8..=512).contains(&captures));
    std::fs::create_dir_all(&output).unwrap();
    let result_path = output.join("result.json");
    let script = output.join("fixture.ps1");
    std::fs::write(&script,b"\xef\xbb\xbfparam([string]$TextFile)\n[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)\n[Console]::Write([IO.File]::ReadAllText($TextFile,[Text.Encoding]::UTF8))\nStart-Sleep -Seconds 125\n").unwrap();
    let subject = output.join("table.ansi");
    std::fs::write(&subject,"\x1b[2J\x1b[H\x1b[?25l\x1b[0m文件                                说明\r\n─────────────────────────────────────────────────────────────\r\n\x1b[38;2;170;130;210mquarantined-example.tar.gz.enc\x1b[0m       固定测试文件\r\n\x1b[38;2;170;130;210marchive.sha256\x1b[0m                       校验清单\r\n\x1b[38;2;170;130;210marchive-contents.txt\x1b[0m                 文件清单\r\n\x1b[38;2;170;130;210moriginal-files.sha256\x1b[0m                原始文件的 SHA256\r\n\x1b[38;2;170;130;210mREADME.txt\x1b[0m                           示例说明\r\n─────────────────────────────────────────────────────────────\r\nFIXED_TABLE_READY").unwrap();
    let noise = output.join("noise.ansi");
    let mut noise_text = String::from("\x1b[2J\x1b[H\x1b[?25l");
    for i in 0..2400 {
        noise_text.push(char::from_u32(0x4e00 + i).unwrap());
        if i % 40 == 39 {
            noise_text.push_str("\r\n");
        }
    }
    std::fs::write(&noise, noise_text).unwrap();
    eframe::run_native(
        "ai-terminal render update acceptance",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_transparent(true)
                .with_inner_size([1100. * scale, 780. * scale]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            ai_terminal::zoom::initialize(&cc.egui_ctx, scale);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                subject: terminal(&cc.egui_ctx, 4301, &subject, &script),
                noise: terminal(&cc.egui_ctx, 4302, &noise, &script),
                output,
                began: Instant::now(),
                stage: 0,
                frames: 0,
                requested: false,
                baseline: None,
                grid_hash: None,
                scale,
                captures,
                crop: [0; 4],
                checks: vec![],
                finished: false,
            }))
        }),
    )?;
    let result: serde_json::Value = serde_json::from_slice(&std::fs::read(result_path)?)?;
    if result["ok"] != true {
        return Err(std::io::Error::other("fixed-region pixel/grid stability failed").into());
    }
    Ok(())
}
