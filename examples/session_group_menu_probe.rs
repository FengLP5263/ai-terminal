//! Production context menus, with owned fixtures and bounded synthetic input.
use ai_terminal::{
    appearance,
    i18n::{self, Language},
    model::{Agent, SessionGroup},
    sidebar::{self, Entry, Icons},
    status::Activity,
};
use egui::{Event, Pos2, Rect};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn text(ctx: &egui::Context, wanted: &str) -> Option<(Rect, usize)> {
    fn find(shape: &egui::Shape, wanted: &str) -> Option<(Rect, usize)> {
        match shape {
            egui::Shape::Text(t) if t.galley.job.text == wanted => Some((
                Rect::from_min_size(t.pos, t.galley.size()),
                t.galley.rows.len(),
            )),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, wanted)),
            _ => None,
        }
    }
    let layers: Vec<_> = ctx.memory(|m| m.layer_ids().collect());
    ctx.graphics(|g| {
        layers
            .iter()
            .filter_map(|id| g.get(*id))
            .flat_map(|list| list.all_entries())
            .find_map(|s| find(&s.shape, wanted))
    })
}
struct Probe {
    renderer: egui_term::TextRendererSession,
    icons: Icons,
    output: PathBuf,
    began: Instant,
    since: Instant,
    case: usize,
    phase: usize,
    input_phase: Option<(usize, usize)>,
    target: Option<Pos2>,
    pending: bool,
    done: bool,
    checks: Vec<serde_json::Value>,
}
impl Probe {
    fn scale(&self) -> f32 {
        [1.0, 1.33, 2.0][self.case / 2]
    }
    fn english(&self) -> bool {
        self.case % 2 == 1
    }
    fn advance(&mut self) {
        self.phase += 1;
        self.since = Instant::now();
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
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let Event::Screenshot { image, .. } = event {
                let name = format!(
                    "menu-{}-{}.png",
                    (self.scale() * 100.0).round() as u32,
                    if self.english() { "en" } else { "zh" }
                );
                let mut encoder = png::Encoder::new(
                    std::fs::File::create(self.output.join(name)).unwrap(),
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
                self.done = true;
            }
        }
        input.focused = true;
        input
            .events
            .retain(|e| matches!(e, Event::Screenshot { .. }));
        if self.input_phase == Some((self.case, self.phase)) {
            return;
        }
        self.input_phase = Some((self.case, self.phase));
        let pos = if self.phase < 3 {
            ctx.read_response(egui::Id::new(("session-row", "one")))
                .map(|r| r.rect.center())
        } else {
            self.target
        };
        if let Some(pos) = pos {
            input.events.push(Event::PointerMoved(pos));
            if matches!(self.phase, 1 | 2 | 4 | 5) {
                input.events.push(Event::PointerButton {
                    pos,
                    button: if self.phase < 3 {
                        egui::PointerButton::Secondary
                    } else {
                        egui::PointerButton::Primary
                    },
                    pressed: matches!(self.phase, 1 | 4),
                    modifiers: egui::Modifiers::NONE,
                });
            }
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        assert!(
            self.began.elapsed() < Duration::from_secs(40),
            "menu capture timed out at {} / {}",
            self.case,
            self.phase
        );
        ctx.request_repaint_after(Duration::from_millis(60));
        if self.done {
            if self.case == 5 {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"checks":self.checks,"synthetic_events":true,"model_requests":false})).unwrap()).unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            egui::Popup::close_all(ctx);
            self.case += 1;
            self.phase = 0;
            self.since = Instant::now();
            self.pending = false;
            self.done = false;
            self.target = None;
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.phase == 0 {
            ctx.set_zoom_factor(self.scale());
            i18n::apply(
                &ctx,
                if self.english() {
                    Language::En
                } else {
                    Language::ZhCn
                },
            );
            if self.input_phase != Some((self.case, 0)) {
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(780.0, 460.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(20.0, 20.0)));
        }
        let entries = [Entry {
            id: "one",
            name: "聚合管家 📣",
            directory: std::path::Path::new("sample-project"),
            agent: Agent::Shell,
            color: None,
            activity: Activity::Idle,
            unread: false,
            hint: String::new(),
        }];
        let groups = [
            SessionGroup {
                id: "team".into(),
                name: "聚合团队".into(),
                members: vec!["one".into()],
            },
            SessionGroup {
                id: "long".into(),
                name: "较长的项目队伍名称 / A longer project team name".into(),
                members: vec!["other".into()],
            },
        ];
        sidebar::show_with_groups(
            ui,
            &entries,
            Some("one"),
            false,
            238.0,
            &self.icons,
            &groups,
        );
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::BLACK))
            .show(ui, |_| {});
        match self.phase {
            0 if self.since.elapsed() > Duration::from_millis(350) => self.advance(),
            1 | 2 | 4 | 5 if self.input_phase == Some((self.case, self.phase)) => self.advance(),
            3 => {
                if let Some((rect, _)) = text(&ctx, if self.english() { "Group" } else { "分组" })
                {
                    self.target = Some(rect.center());
                    self.advance();
                }
            }
            6 if self.since.elapsed() > Duration::from_millis(400) && !self.pending => {
                let labels = if self.english() {
                    [
                        "New group…",
                        "Remove from group",
                        "Rename group…",
                        "Ungroup sessions",
                    ]
                } else {
                    ["新建分组…", "移出分组", "重命名分组…", "解散分组"]
                };
                let parent = if self.english() {
                    ["Rename", "Session color", "Close session"]
                } else {
                    ["编辑名称", "会话颜色", "关闭会话"]
                };
                for label in labels.into_iter().chain(parent) {
                    let (rect, rows) =
                        text(&ctx, label).unwrap_or_else(|| panic!("missing {label}"));
                    assert_eq!(rows, 1);
                    assert!(rect.width() <= 200.0);
                }
                self.checks.push(serde_json::json!({"scale":self.scale(),"language":if self.english(){"en"}else{"zh"},"single_line_commands":true}));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.pending = true;
            }
            _ => {}
        }
    }
}
fn main() -> eframe::Result {
    if !cfg!(windows) {
        eprintln!("session_group_menu_probe requires Windows");
        std::process::exit(1);
    }
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal group menu acceptance",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([780.0, 460.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            ai_terminal::window_frame::apply_dark(cc);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                icons: Icons::new(&cc.egui_ctx),
                output,
                began: Instant::now(),
                since: Instant::now(),
                case: 0,
                phase: 0,
                input_phase: None,
                target: None,
                pending: false,
                done: false,
                checks: vec![],
            }))
        }),
    )?;
    let result: serde_json::Value = serde_json::from_slice(
        &std::fs::read(verify.join("result.json")).expect("native menu result"),
    )
    .unwrap();
    assert_eq!(result["ok"], true);
    Ok(())
}
