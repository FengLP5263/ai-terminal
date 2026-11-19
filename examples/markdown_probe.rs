//! Native Markdown screenshots of tracked documentation and isolated fixtures. No PTY/model.
use ai_terminal::{
    appearance,
    i18n::{self, Language},
    project_files::{self, FileClient},
    project_preview::View,
};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

const CASES: &[(&str, f32, f32)] = &[
    ("README.md", 1120.0, 1.0),
    ("README.en.md", 1120.0, 1.0),
    ("agents.md", 1120.0, 1.0),
    ("features.md", 1120.0, 1.0),
    ("features.md", 560.0, 1.0),
    ("agents.md", 700.0, 1.33),
    ("cli.md", 1120.0, 1.0),
    ("cli.md", 560.0, 1.0),
    ("settings-context.md", 900.0, 1.0),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    root: PathBuf,
    output: PathBuf,
    documents: Vec<Arc<project_files::Document>>,
    client: FileClient,
    view: View,
    stage: usize,
    frames: usize,
    pending: bool,
    began: Instant,
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
        assert!(self.began.elapsed() < Duration::from_secs(55));
        self.client.poll();
        ctx.request_repaint_after(Duration::from_millis(70));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let mut encoder = png::Encoder::new(
                    std::fs::File::create(self.output.join(format!("markdown-{}.png", self.stage)))
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
                self.stage += 1;
                self.frames = 0;
                self.pending = false;
            }
        }
        input
            .events
            .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == CASES.len() {
            std::fs::write(self.output.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"screenshots":CASES.len(),"version":env!("CARGO_PKG_VERSION"),"model_requests":false,"user_data":false,"source":"tracked bilingual README and copied public image assets","fixture_root":self.root})).unwrap()).unwrap();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let (name, width, zoom) = CASES[self.stage];
        if self.frames == 0 {
            self.view.clear(ui.ctx());
            ui.ctx().set_zoom_factor(zoom);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(width, 880.0) / zoom,
            ));
        }
        egui::Panel::top("markdown-probe-header").show(ui, |ui| {
            ui.label(format!("Native Markdown · {name}"));
        });
        self.view.show(
            ui,
            &mut self.client,
            &self.documents[self.stage],
            std::path::Path::new(name),
        );
        self.frames += 1;
        if self.frames > 10 && !self.pending {
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
    assert!(!output.join("result.json").exists());
    let root = output.join("fixture");
    std::fs::create_dir(&root).unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for folder in ["assets/branding", "assets/readme"] {
        std::fs::create_dir_all(root.join(folder)).unwrap();
        for item in std::fs::read_dir(source.join(folder)).unwrap() {
            let item = item.unwrap();
            if item.path().extension().is_some_and(|ext| ext == "png") {
                std::fs::copy(item.path(), root.join(folder).join(item.file_name())).unwrap();
            }
        }
    }
    for name in ["README.md", "README.en.md"] {
        std::fs::copy(source.join(name), root.join(name)).unwrap();
    }
    std::fs::copy(source.join("docs/cli.md"), root.join("cli.md")).unwrap();
    std::fs::copy(source.join("README.md"), root.join("settings-context.md")).unwrap();
    let readme = std::fs::read_to_string(source.join("README.md")).unwrap();
    let section = |heading: &str| {
        readme
            .split_once(heading)
            .unwrap()
            .1
            .split("\n## ")
            .next()
            .unwrap()
    };
    std::fs::write(
        root.join("agents.md"),
        format!("## 支持的 Agent\n{}", section("## 支持的 Agent")),
    )
    .unwrap();
    std::fs::write(
        root.join("features.md"),
        format!("## 为多会话而设计\n{}", section("## 为多会话而设计")),
    )
    .unwrap();
    let documents = CASES
        .iter()
        .map(|(name, _, _)| {
            let mut document =
                project_files::read_file(&root, std::path::Path::new(name), || false).unwrap();
            if *name == "settings-context.md" {
                // Prepare the entire README first, retaining its complete image budget.
                // Only the native capture layout is restricted to the reported section.
                let md = document.markdown.as_mut().unwrap();
                assert!(md.images["assets/readme/settings.png"].is_ok());
                let index = md
                    .headings
                    .iter()
                    .position(|heading| heading.text == "设置与浮窗")
                    .unwrap();
                let start = md
                    .text
                    .find(&format!("<{}-heading-{index} />", md.code_marker))
                    .unwrap();
                let end = md.text[start + 1..]
                    .find(&format!("<{}-heading-{} />", md.code_marker, index + 1))
                    .map_or(md.text.len(), |end| start + 1 + end);
                md.text = md.text[start..end].to_owned();
            }
            Arc::new(document)
        })
        .collect();
    eframe::run_native(
        "AI Terminal Markdown preview",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1120.0, 880.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, Language::ZhCn);
            ai_terminal::window_frame::apply_dark(cc);
            let mut client = FileClient::new(cc.egui_ctx.clone());
            client.set_root(Some(&root));
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                root,
                output,
                documents,
                client,
                view: View::default(),
                stage: 0,
                frames: 0,
                pending: false,
                began: Instant::now(),
            }))
        }),
    )
}
