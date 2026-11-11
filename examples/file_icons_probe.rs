//! Native file-glyph matrix: production mapping/font, fixed names, no PTY or model.
use ai_terminal::{appearance, file_icons, font_choice::FontChoice};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
const CASES: &[(FontChoice, f32)] = &[
    (FontChoice::JetBrainsMono, 1.0),
    (FontChoice::Hack, 1.33),
    (FontChoice::CascadiaMono, 2.0),
];
const FILES: &[&str] = &[
    "analyze.py",
    "index.html",
    "main.rs",
    "app.js",
    "app.ts",
    "main.go",
    "component.jsx",
    "view.tsx",
    "style.css",
    "page.vue",
    "page.svelte",
    "theme.scss",
    "Main.java",
    "Main.kt",
    "main.cpp",
    "Main.cs",
    "lib.rb",
    "script.php",
    "setup.ps1",
    "run.sh",
    "start.cmd",
    "data.json",
    "notes.md",
    "config.yaml",
    "query.sql",
    "Dockerfile",
    "package.json",
    "Cargo.toml",
    "photo.png",
    "movie.mp4",
    "sound.mp3",
    "manual.pdf",
    "data.csv",
    "table.xlsx",
    "data.zip",
    "unknown.xyz",
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    output: PathBuf,
    stage: usize,
    frames: usize,
    began: Instant,
    since: Instant,
    pending: bool,
    records: Vec<serde_json::Value>,
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
            self.began.elapsed() < Duration::from_secs(22),
            "Native icon capture timed out"
        );
        if self.stage == CASES.len() {
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":self.records,"model_requests":false,"user_profiles":false,"synthetic_names":true,"ptys":0})).unwrap()).unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(60));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = format!("file-icons-{}.png", self.stage);
                let file = std::fs::File::create(self.output.join(&name)).unwrap();
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
                self.records.push(serde_json::json!({"image":name,"font":CASES[self.stage].0.family(),"zoom":CASES[self.stage].1,"file_types":FILES.len()}));
                self.stage += 1;
                self.frames = 0;
                self.pending = false;
                self.since = Instant::now();
            }
        }
        input.events.clear();
        input.events.push(egui::Event::PointerGone);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= CASES.len() {
            return;
        }
        if self.frames == 0 {
            appearance::set_terminal_font(ui.ctx(), CASES[self.stage].0);
            ui.ctx().set_zoom_factor(CASES[self.stage].1);
        }
        ui.style_mut().override_font_id = Some(egui::FontId::monospace(14.0));
        ui.heading("File types");
        ui.label(format!(
            "{} · {:.0}%",
            CASES[self.stage].0.family(),
            CASES[self.stage].1 * 100.0
        ));
        ui.add_space(4.0);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        let width = (ui.available_width() - 16.0) / 3.0;
        egui::Grid::new("file-icons-matrix")
            .num_columns(3)
            .spacing([8.0, 2.0])
            .show(ui, |ui| {
                for (i, path) in FILES.iter().enumerate() {
                    let icon = file_icons::icon(path, false);
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, 22.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            icon.show(ui, 14.0);
                            ui.add(egui::Label::new(*path).truncate());
                        },
                    );
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        self.frames += 1;
        if !self.pending && self.frames > 8 && self.since.elapsed() > Duration::from_millis(600) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.pending = true;
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(!output.join("result.json").exists());
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal file icons",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 720.0])
                .with_active(false)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            ai_terminal::window_frame::apply_dark(cc);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                output,
                stage: 0,
                frames: 0,
                began: Instant::now(),
                since: Instant::now(),
                pending: false,
                records: Vec::new(),
            }))
        }),
    )?;
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(verify.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["ok"], true);
    Ok(())
}
