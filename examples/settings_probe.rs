//! Wide/narrow bilingual native form captures using production components only.
use ai_terminal::{
    appearance, dialogs,
    i18n::{self, Language},
    model::UiSettings,
    settings,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
const CASES: &[(Language, [f32; 2], f32, u8, bool)] = &[
    (Language::ZhCn, [1000.0, 720.0], 1.0, 0, true),
    (Language::En, [1000.0, 720.0], 1.0, 0, true),
    (Language::ZhCn, [400.0, 820.0], 1.0, 0, true),
    (Language::En, [400.0, 820.0], 1.0, 0, true),
    (Language::ZhCn, [1000.0, 720.0], 1.5, 0, false),
    (Language::En, [1000.0, 720.0], 1.5, 0, false),
    (Language::ZhCn, [900.0, 650.0], 1.0, 1, true),
    (Language::En, [900.0, 650.0], 1.0, 1, true),
    (Language::ZhCn, [900.0, 650.0], 1.0, 2, true),
    (Language::En, [900.0, 650.0], 1.0, 3, true),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    output: PathBuf,
    stage: usize,
    frames: usize,
    pending: bool,
    began: Instant,
    since: Instant,
    prefs: UiSettings,
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
            self.began.elapsed() < Duration::from_secs(50),
            "Settings capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(50));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = format!("settings-{}.png", self.stage);
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
                self.records.push(serde_json::json!({"image":name,"language":format!("{:?}",CASES[self.stage].0),"viewport":CASES[self.stage].1,"scale":CASES[self.stage].2,"kind":CASES[self.stage].3}));
                self.stage += 1;
                self.frames = 0;
                self.pending = false;
                self.since = Instant::now();
            }
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        if matches!(self.stage, 4 | 5) && self.frames == 5 {
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(320.0, 300.0)));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -8.0),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            });
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == CASES.len() {
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":self.records,"model_requests":false,"process_actions":false})).unwrap()).unwrap();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let (language, size, zoom, kind, enabled) = CASES[self.stage];
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(zoom);
            i18n::apply(ui.ctx(), language);
            self.prefs.language = language;
            self.prefs.floating_enabled = enabled;
        }
        if self.frames == 1 {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(size[0], size[1]) / ui.ctx().zoom_factor(),
            ));
        }
        match kind {
            0 => {
                settings::show(ui.ctx(), &mut true, &mut self.prefs);
            }
            1 => {
                dialogs::rename(ui.ctx(), &mut true, &mut "会话名称 👩🏽‍💻 ❤️".to_owned());
            }
            2 => {
                dialogs::confirm(
                    ui.ctx(),
                    "probe-confirm",
                    i18n::tr(ui.ctx(), "关闭会话？", "Close session?"),
                    i18n::tr(
                        ui.ctx(),
                        "该会话的运行进程将结束，对话历史仍由 Agent 保留。",
                        "This session will stop. Conversation history remains with the agent.",
                    ),
                    i18n::tr(ui.ctx(), "关闭会话", "Close session"),
                );
            }
            _ => {
                dialogs::confirm(
                    ui.ctx(),
                    "probe-confirm",
                    i18n::tr(ui.ctx(), "退出 ai-terminal？", "Quit ai-terminal?"),
                    i18n::tr(
                        ui.ctx(),
                        "终端会话将关闭，下次可以恢复对话历史。",
                        "Terminal sessions will close. You can resume conversations next time.",
                    ),
                    i18n::tr(ui.ctx(), "退出", "Quit"),
                );
            }
        }
        self.frames += 1;
        if !self.pending && self.frames > 8 && self.since.elapsed() > Duration::from_millis(450) {
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
    eframe::run_native(
        "ai-terminal workbench forms",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1000.0, 720.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                output,
                stage: 0,
                frames: 0,
                pending: false,
                began: Instant::now(),
                since: Instant::now(),
                prefs: UiSettings::default(),
                records: vec![],
            }))
        }),
    )
}
