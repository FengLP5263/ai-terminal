//! Native application screenshots of CMD -> PowerShell -> CMD and per-level exit.
//! Only synthetic input into this isolated window; no Agent, model or user profile.
#[path = "../src/app.rs"]
mod app;
#[path = "../src/platform.rs"]
mod platform;
#[path = "../src/shutdown.rs"]
mod shutdown;
#[path = "../src/window_geometry.rs"]
mod window_geometry;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Probe {
    inner: app::App,
    output: PathBuf,
    stage: usize,
    stage_began: Instant,
    capture_pending: bool,
    input_pending: Option<&'static str>,
    finished: bool,
}

impl eframe::App for Probe {
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.inner.on_exit(gl);
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.inner.on_exit();
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.finished {
            self.inner.logic(ctx, frame);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.finished {
            return;
        }
        input.focused = true;
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .focused = Some(true);
        if let Some(command) = self.input_pending.take() {
            input.events.push(egui::Event::Text(command.into()));
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        self.inner.raw_input_hook(ctx, input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.finished {
            return;
        }
        self.inner.ui(ui, frame);
        let ctx = ui.ctx();
        if self.stage_began.elapsed() > Duration::from_secs(5) && !self.capture_pending {
            self.capture_pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            let name = [
                "root-cmd",
                "powershell",
                "nested-cmd",
                "return-powershell",
                "return-root-cmd",
            ][self.stage];
            let mut encoder = png::Encoder::new(
                std::fs::File::create(self.output.join(format!("{name}.png"))).unwrap(),
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
                        .flat_map(|pixel| pixel.to_array())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            self.stage += 1;
            if self.stage == 5 {
                std::fs::write(self.output.join("capture.json"), r#"{"screenshots":5,"visual_review_required":true,"model_request_sent":false,"isolated_sessions":true}"#).unwrap();
                self.finished = true;
                // Bypass confirmation only for our completed probe, with normal on_exit/drop.
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.input_pending =
                Some(["powershell -NoLogo -NoProfile", "cmd /d", "exit", "exit"][self.stage - 1]);
            self.stage_began = Instant::now();
            self.capture_pending = false;
        }
    }
}

fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("shell_switch_probe output-directory"),
    );
    let data = output.join("data");
    std::fs::create_dir_all(&data).unwrap();
    assert!(
        !data.join("sessions.json").exists(),
        "Use a fresh output directory"
    );
    let helper = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("ai-terminal-hook.exe");
    std::fs::copy(
        helper,
        std::env::current_exe()
            .unwrap()
            .with_file_name("ai-terminal-hook.exe"),
    )
    .unwrap();
    ai_terminal::model::save(
        &data.join("sessions.json"),
        &ai_terminal::model::Workspace {
            sessions: vec![ai_terminal::model::SessionSpec {
                id: "shell-switch".into(),
                name: "Shell switch preview".into(),
                directory: PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot")),
                agent: ai_terminal::model::Agent::Cmd,
                agent_session_id: None,
                color: None,
            }],
            active: Some("shell-switch".into()),
            ..Default::default()
        },
    )
    .unwrap();
    eframe::run_native(
        "ai-terminal shell switching probe",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(ai_terminal::appearance::window_icon())
                .with_inner_size([1000.0, 600.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Probe {
                inner: app::App::new(cc, data, false, false),
                output,
                stage: 0,
                stage_began: Instant::now(),
                capture_pending: false,
                input_pending: None,
                finished: false,
            }))
        }),
    )
}
