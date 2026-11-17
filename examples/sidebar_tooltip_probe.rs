//! Production sidebar tooltip captures. Owned metadata and local egui input only; no PTY or Agent.
use ai_terminal::{
    appearance,
    model::{Agent, SessionColor, SessionGroup},
    sidebar::{self, Entry, Icons},
    status::Activity,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const CASES: &[(bool, f32)] = &[
    (true, 1.0),
    (false, 1.0),
    (true, 1.14),
    (false, 1.14),
    (true, 2.0),
    (false, 2.0),
    (true, 1.0), // Header metadata tooltip.
    (false, 2.0),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    icons: Icons,
    output: PathBuf,
    stage: usize,
    frames: usize,
    requested: bool,
    began: Instant,
    records: Vec<serde_json::Value>,
    header_point: Option<egui::Pos2>,
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
            "sidebar tooltip probe timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .focused = Some(true);
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let (collapsed, zoom) = CASES[self.stage];
                let name = format!(
                    "tooltip-{}-{}-{}.png",
                    self.stage,
                    if self.stage >= 6 {
                        "header"
                    } else if collapsed {
                        "collapsed"
                    } else {
                        "expanded"
                    },
                    (zoom * 100.0).round() as u32
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
                self.records.push(serde_json::json!({"image":name,"collapsed":collapsed,"zoom":zoom,"tooltip_open":true}));
                self.stage += 1;
                self.frames = 0;
                self.requested = false;
            }
        }
        if self.frames == 3 {
            let point = if self.stage >= 6 {
                self.header_point
            } else {
                ctx.read_response(egui::Id::new(("session-row", "leader")))
                    .map(|r| r.rect.center())
            };
            if let Some(point) = point {
                input.events.push(egui::Event::PointerMoved(point));
            }
        } else if self.frames < 3 {
            input.events.push(egui::Event::PointerGone);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == CASES.len() {
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"synthetic_metadata":true,"model_requests":false,"user_configuration":false,"pty_created":false,"system_input_injection":false,"screenshots":self.records})).unwrap()).unwrap();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let (collapsed, zoom) = CASES[self.stage];
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(zoom);
        }
        if self.stage >= 6 {
            egui::CentralPanel::default().show(ui, |ui| {
                self.header_point = Some(ui.cursor().min + egui::vec2(24.0, 10.0));
                ai_terminal::git_ui::header(
                    ui,
                    ai_terminal::git_ui::Header {
                        name: "AI终端 ❤️ 👩🏽‍💻",
                        directory: Path::new("C:/项目/🚀/文件"),
                        scale: zoom,
                        connection: None,
                    },
                    None,
                    &mut false,
                );
                ui.label("Header metadata hover / native emoji");
            });
            self.frames += 1;
            if self.frames >= 18 && !self.requested {
                self.requested = true;
                ui.ctx()
                    .send_viewport_cmd(
                        egui::ViewportCommand::Screenshot(egui::UserData::default()),
                    );
            }
            return;
        }
        let entries = [Entry {
            id: "leader",
            name: "大管家 ❤️ 👩🏽‍💻",
            directory: Path::new(
                "C:/Workspace/ExampleProject/协作目录🚀/ModuleWithALongName/Documentation/DesignAndImplementationNotes/AdditionalContext/ProjectStructureAndSessionNavigation/MyAgent/子目录",
            ),
            agent: Agent::Codex,
            color: Some(SessionColor::Green),
            activity: Activity::Idle,
            unread: false,
            hint: "就绪 · 最近事件 OSC701".into(),
        }];
        let groups = [SessionGroup {
            id: "team".into(),
            name: "小队 🇨🇳 1️⃣".into(),
            members: vec!["leader".into()],
        }];
        let actions = sidebar::show_with_groups(
            ui,
            &entries,
            Some("leader"),
            collapsed,
            238.0,
            &self.icons,
            &groups,
        );
        assert!(actions.select.is_none() && actions.rename.is_none() && actions.reorder.is_none());
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Sidebar tooltip: synthetic Unicode metadata only");
        });
        self.frames += 1;
        if self.frames >= 12
            && !self.requested
            && ui
                .ctx()
                .read_response(egui::Id::new(("session-row", "leader")))
                .is_some_and(|r| r.is_tooltip_open())
        {
            self.requested = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("unique output directory required"),
    );
    assert!(!output.exists());
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal sidebar tooltip verification",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([900.0, 600.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                icons: Icons::new(&cc.egui_ctx),
                output,
                stage: 0,
                frames: 0,
                requested: false,
                began: Instant::now(),
                records: vec![],
                header_point: None,
            }))
        }),
    )
}
