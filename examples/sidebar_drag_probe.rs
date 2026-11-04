//! Native production-sidebar capture with synthetic metadata and local egui input only.
//! No PTY, Agent, user configuration, clipboard access, or system input injection.
use ai_terminal::{
    appearance, floating,
    i18n::Locale,
    model::{self, Agent, SessionColor},
    sidebar::{self, Entry, Icons},
    status::Activity,
};
use egui::{Event, PointerButton, Pos2};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const SHOTS: [&str; 8] = [
    "expanded-unread.png",
    "expanded-read.png",
    "expanded-drag.png",
    "expanded-drop.png",
    "collapsed-unread.png",
    "collapsed-read.png",
    "collapsed-drag.png",
    "collapsed-drop.png",
];

struct Probe {
    renderer: egui_term::TextRendererSession,
    icons: Icons,
    ids: Vec<String>,
    output: PathBuf,
    began: Instant,
    stage_started: Instant,
    stage: usize,
    frames: usize,
    pointer: Pos2,
    pending: bool,
    reorders: usize,
}

impl Probe {
    fn tick(&self, ctx: &egui::Context) {
        assert!(
            self.began.elapsed() < Duration::from_secs(30),
            "capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(80));
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
        self.tick(ctx);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .focused = Some(true);
        // Keep this temporary probe's gesture independent of physical mouse input.
        input
            .events
            .retain(|event| matches!(event, Event::Screenshot { .. }));
        if matches!(self.stage, 2 | 6) {
            let row = |id| {
                ctx.read_response(egui::Id::new(("session-row", id)))
                    .unwrap()
                    .rect
            };
            match self.frames {
                0 => {
                    self.pointer = row("codex").min + egui::vec2(17.0, 17.0);
                    input.events.push(Event::PointerMoved(self.pointer));
                }
                1 => {
                    input.events.extend([
                        Event::PointerMoved(self.pointer),
                        Event::PointerButton {
                            pos: self.pointer,
                            button: PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]);
                }
                2 => {
                    self.pointer.y += 15.0;
                    input.events.push(Event::PointerMoved(self.pointer));
                }
                3 => {
                    self.pointer = row(if self.stage == 2 { "shell" } else { "claude" }).min
                        + egui::vec2(29.0, 24.0);
                    input.events.push(Event::PointerMoved(self.pointer));
                }
                _ => {}
            }
        } else if matches!(self.stage, 3 | 7) && self.frames == 0 {
            input.events.push(Event::PointerButton {
                pos: self.pointer,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let collapsed = self.stage >= 4;
        let unread = matches!(self.stage, 0 | 4);
        let entries: Vec<_> = self
            .ids
            .iter()
            .map(|id| {
                let (name, agent, activity, color) = match id.as_str() {
                    "codex" => (
                        "Code review 👩‍💻",
                        Agent::Codex,
                        Activity::Completed,
                        SessionColor::Blue,
                    ),
                    "claude" => (
                        "Release notes ✨",
                        Agent::Claude,
                        Activity::Idle,
                        SessionColor::Purple,
                    ),
                    _ => (
                        "Build failed",
                        Agent::Shell,
                        Activity::Failed,
                        SessionColor::Green,
                    ),
                };
                Entry {
                    id,
                    name,
                    agent,
                    activity,
                    color: Some(color),
                    unread: unread && id == "shell",
                    directory: std::path::Path::new("C:/Projects/demo"),
                    hint: "Synthetic fixture".into(),
                }
            })
            .collect();
        let actions = sidebar::show(ui, &entries, Some("codex"), collapsed, 246.0, &self.icons);
        if matches!(self.stage, 2 | 6) && self.frames >= 6 {
            assert!(
                egui::DragAndDrop::has_any_payload(&ctx),
                "synthetic card drag did not start at stage {}",
                self.stage
            );
        }
        assert!(
            actions.select.is_none() && actions.rename.is_none(),
            "drag accidentally opened a session"
        );
        let snapshot = floating::Snapshot {
            items: entries
                .iter()
                .map(|e| floating::Item {
                    id: e.id.into(),
                    name: e.name.into(),
                    agent: e.agent,
                    color: e.color,
                    activity: e.activity,
                    unread: e.unread,
                })
                .collect(),
            active: Some("codex".into()),
            locale: Locale::En,
        };
        if let Some((from, to, after)) = actions.reorder {
            let source = self.ids.iter().position(|id| id == &from).unwrap();
            let target = self.ids.iter().position(|id| id == &to).unwrap();
            model::reorder(&mut self.ids, source, target, after);
            self.reorders += 1;
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(24),
            )
            .show(ui, |ui| {
                ui.heading("Session navigation");
                ui.add_space(12.0);
                ui.label(if unread {
                    "Unread failure: red badge in the sidebar and floating rail"
                } else {
                    "Viewed failure: badge cleared; failed status preserved"
                });
                ui.add_space(12.0);
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(190.0, 48.0),
                    )),
                    |ui| {
                        floating::body(ui, &snapshot, &self.icons);
                    },
                );
                ui.add_space(24.0);
                ui.label("Build failed · read");
                ui.label("Drag a card to reorder. Esc or release outside the list to cancel.");
                ui.label("Synthetic sessions only · no Agent or model request");
            });
        self.frames += 1;
        if let Some(image) = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        }) && self.pending
        {
            let file = std::fs::File::create(self.output.join(SHOTS[self.stage])).unwrap();
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
            self.pending = false;
            self.stage += 1;
            self.frames = 0;
            self.stage_started = Instant::now();
            if self.stage == SHOTS.len() {
                assert_eq!(self.reorders, 2);
                assert_eq!(self.ids, ["codex", "claude", "shell"]);
                std::fs::write(
                    self.output.join("result.json"),
                    serde_json::to_vec_pretty(&serde_json::json!({
                        "ok": true, "screenshots": SHOTS, "reorders": self.reorders,
                        "synthetic_input_only": true, "model_request_sent": false,
                    }))
                    .unwrap(),
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.stage < SHOTS.len()
            && !self.pending
            && self.frames >= 6
            && self.stage_started.elapsed() > Duration::from_millis(600)
        {
            self.pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        self.tick(&ctx);
    }
}

fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("sidebar_drag_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal sidebar interaction probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([1000.0, 640.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                icons: Icons::new(&cc.egui_ctx),
                ids: ["codex", "claude", "shell"].map(String::from).to_vec(),
                output,
                began: Instant::now(),
                stage_started: Instant::now(),
                stage: 0,
                frames: 0,
                pointer: Pos2::ZERO,
                pending: false,
                reorders: 0,
            }))
        }),
    )
}
