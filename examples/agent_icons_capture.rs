//! Native README badges using the production avatar painter. No PTY or Agent.
use ai_terminal::{appearance, model::Agent, sidebar::Icons};
use std::path::PathBuf;

const AGENTS: [(&str, Agent); 6] = [
    ("codex", Agent::Codex),
    ("claude", Agent::Claude),
    ("qoder-cn", Agent::QoderCn),
    ("kimi", Agent::Kimi),
    ("zcode", Agent::ZCode),
    ("trae", Agent::Trae),
];

struct Capture {
    icons: Icons,
    output: PathBuf,
    index: usize,
    frames: u8,
    pending: bool,
}

impl eframe::App for Capture {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.index == AGENTS.len() {
            return;
        }
        let ctx = ui.ctx().clone();
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, egui::Color32::from_gray(29));
        self.icons.paint(
            ui.painter(),
            egui::Rect::from_center_size(ui.max_rect().center(), egui::vec2(36.0, 36.0)),
            AGENTS[self.index].1,
        );
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            let mut encoder = png::Encoder::new(
                std::fs::File::create(
                    self.output
                        .join(format!("agent-{}.png", AGENTS[self.index].0)),
                )
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
                        .flat_map(|pixel| pixel.to_array())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            self.index += 1;
            self.frames = 0;
            self.pending = false;
            if self.index == AGENTS.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        self.frames += 1;
        if self.frames >= 3 && !self.pending {
            self.pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

fn main() -> eframe::Result {
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal agent icon capture",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([40.0, 40.0])
                .with_decorations(false)
                .with_resizable(false),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            Ok(Box::new(Capture {
                icons: Icons::new(&cc.egui_ctx),
                output,
                index: 0,
                frames: 0,
                pending: false,
            }))
        }),
    )
}
