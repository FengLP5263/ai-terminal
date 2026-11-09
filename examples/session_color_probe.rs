//! Production session cards and palette, driven only by local synthetic pointer input.
use ai_terminal::{
    appearance, i18n,
    model::{Agent, SessionColor},
    sidebar::{self, Entry, Icons},
    status::Activity,
};
use egui::Event;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const SCALES: [f32; 3] = [1.0, 1.03, 2.0];
const STATES: [&str; 4] = ["selected", "hovered", "no-color", "collapsed"];

struct Probe {
    renderer: egui_term::TextRendererSession,
    icons: Icons,
    output: PathBuf,
    started: Instant,
    since: Instant,
    stage: usize,
    frames: usize,
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
            self.started.elapsed() < Duration::from_secs(45),
            "color capture timeout"
        );
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .events
            .retain(|event| matches!(event, Event::Screenshot { .. }));
        let hovered = self.stage % STATES.len() == 1;
        let pointer = if hovered {
            ctx.read_response(egui::Id::new(("session-row", "pink")))
                .map(|r| r.rect.center())
                .unwrap_or(egui::pos2(900.0, 580.0))
        } else {
            egui::pos2(900.0, 580.0)
        };
        input.events.push(Event::PointerMoved(pointer));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= SCALES.len() * STATES.len() {
            return;
        }
        let state = self.stage % STATES.len();
        let scale = SCALES[self.stage / STATES.len()];
        if self.frames == 0 {
            // Keep the fixture's logical work area constant at every zoom level.
            // A fixed physical window would crop the palette at 200%.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1000.0, 660.0)));
        }
        let colors = [
            ("none", "本地 Shell", None),
            ("red", "接口联调", Some(SessionColor::Red)),
            ("green", "CMS 维护", Some(SessionColor::Green)),
            ("yellow", "测试验收", Some(SessionColor::Amber)),
            ("blue", "日常开发", Some(SessionColor::Blue)),
            ("pink", "聚合项目 ❤️", Some(SessionColor::Pink)),
            ("purple", "发布准备", Some(SessionColor::Purple)),
        ];
        let entries: Vec<_> = colors
            .iter()
            .map(|(id, name, color)| Entry {
                id,
                name,
                color: *color,
                agent: Agent::Cmd,
                activity: Activity::Idle,
                unread: false,
                directory: std::path::Path::new("C:/Projects/demo"),
                hint: "Local fixture".into(),
            })
            .collect();
        let active = if state == 2 { "none" } else { "green" };
        let actions = sidebar::show(ui, &entries, Some(active), state == 3, 300.0, &self.icons);
        assert!(actions.select.is_none() && actions.color.is_none());
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_gray(18))
                    .inner_margin(24),
            )
            .show(ui, |ui| {
                ui.heading("会话颜色");
                ui.add_space(16.0);
                ui.label("无颜色 · 红 · 绿 · 黄 · 蓝 · 粉 · 紫");
                egui::Frame::new()
                    .fill(egui::Color32::from_gray(30))
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.separator();
                        let _ = sidebar::color_palette(
                            ui,
                            "palette-preview",
                            if state == 2 {
                                None
                            } else {
                                Some(SessionColor::Pink)
                            },
                        );
                    });
                ui.add_space(24.0);
                ui.label(match state {
                    1 => "绿色：选中；粉色：鼠标悬停。两者使用同样的强调效果。",
                    2 => "无颜色：清晰的圆圈与斜线。",
                    3 => "收起侧栏同样保留颜色与无描边效果。",
                    _ => "绿色：选中；其余会话保持柔和底色。",
                });
                ui.label("本地固定样例，无 Agent 或模型请求。");
            });
        self.frames += 1;
        if self.pending
            && let Some(image) = ui.input(|i| {
                i.events.iter().find_map(|event| {
                    if let Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            let name = format!("{}-{}.png", (scale * 100.0).round() as u32, STATES[state]);
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
            self.records
                .push(serde_json::json!({"image":name,"scale":scale,"state":STATES[state]}));
            self.pending = false;
            self.stage += 1;
            self.frames = 0;
            self.since = Instant::now();
            if self.stage == SCALES.len() * STATES.len() {
                std::fs::write(self.output.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":self.records,"model_requests":false,"synthetic_input_only":true})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().set_zoom_factor(SCALES[self.stage / STATES.len()]);
            }
        } else if !self.pending
            && self.frames >= 5
            && self.since.elapsed() > Duration::from_millis(500)
        {
            let screen = ui.ctx().content_rect();
            let palette_end = ui
                .ctx()
                .read_response(egui::Id::new((
                    "session-color",
                    "palette-preview",
                    Some(SessionColor::Purple),
                )))
                .expect("last palette swatch")
                .rect;
            assert!(
                screen.contains_rect(palette_end),
                "palette was cropped at {scale}"
            );
            if state != 3 {
                let last_row = ui
                    .ctx()
                    .read_response(egui::Id::new(("session-row", "purple")))
                    .expect("last session row")
                    .rect;
                assert!(
                    screen.contains_rect(last_row),
                    "session rows were cropped at {scale}"
                );
            }
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("session_color_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal session colors",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1000.0, 660.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, i18n::Language::ZhCn);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                icons: Icons::new(&cc.egui_ctx),
                output,
                started: Instant::now(),
                since: Instant::now(),
                stage: 0,
                frames: 0,
                pending: false,
                records: Vec::new(),
            }))
        }),
    )
}
