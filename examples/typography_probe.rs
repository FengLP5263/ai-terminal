//! Fixed native text/cursor screenshots; no Agent, user configuration or model.
use ai_terminal::appearance;
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
const SCALES: [f32; 5] = [1.0, 1.03, 1.1, 1.5, 2.0];
const LATIN: [&str; 3] = ["Hack", "Consolas", "Cascadia Mono"];
const CJK: [&str; 3] = ["Microsoft YaHei UI", "Microsoft YaHei", "DengXian"];
struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    output: PathBuf,
    began: Instant,
    since: Instant,
    stage: usize,
    pending: bool,
    checks: Vec<serde_json::Value>,
    interactive: bool,
    latin: usize,
    cjk: usize,
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
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.interactive {
            return;
        }
        assert!(
            self.began.elapsed() < Duration::from_secs(25),
            "typography probe timeout"
        );
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == SCALES.len() {
            return;
        }
        if self.interactive {
            ui.label("字体对比：相同字号、原有抗锯齿；这里只显示固定样例。");
            ui.horizontal_wrapped(|ui| {
                ui.label("英文 / Latin");
                egui::ComboBox::from_id_salt("latin-font")
                    .selected_text(LATIN[self.latin])
                    .show_ui(ui, |ui| {
                        for (i, name) in LATIN.iter().enumerate() {
                            ui.selectable_value(&mut self.latin, i, *name);
                        }
                    });
                ui.label("中文 / CJK");
                egui::ComboBox::from_id_salt("cjk-font")
                    .selected_text(CJK[self.cjk])
                    .show_ui(ui, |ui| {
                        for (i, name) in CJK.iter().enumerate() {
                            ui.selectable_value(&mut self.cjk, i, *name);
                        }
                    });
                for scale in [1.0, 1.1, 1.5, 2.0] {
                    if ui.button(format!("{:.0}%", scale * 100.0)).clicked() {
                        ui.ctx().set_zoom_factor(scale);
                    }
                }
            });
        } else {
            ui.label(format!(
                "{} / {} · {:.0}% · fixed local sample",
                LATIN[self.latin],
                CJK[self.cjk],
                SCALES[self.stage] * 100.0
            ));
        }
        let font = appearance::terminal_font(14.0);
        let font = if self.latin == 0 {
            static HACK: [&[u8]; 4] = [
                include_bytes!("../assets/fonts/Hack-Regular.ttf"),
                include_bytes!("../assets/fonts/Hack-Bold.ttf"),
                include_bytes!("../assets/fonts/Hack-Italic.ttf"),
                include_bytes!("../assets/fonts/Hack-BoldItalic.ttf"),
            ];
            font.with_native_font("Hack", &HACK)
        } else if self.latin == 1 {
            font.with_native_font(LATIN[self.latin], &[])
        } else {
            font
        };
        let font = font.with_native_cjk_font(CJK[self.cjk]);
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_focus(!self.interactive)
            .set_font(font);
        ui.add(view);
        if self.interactive {
            return;
        }
        let ready = self.terminal.plain_text().contains("TYPOGRAPHY_READY");
        if self.pending
            && let Some(image) = ui.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            let mut encoder = png::Encoder::new(
                std::fs::File::create(self.output.join(format!("text-{}.png", self.stage)))
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
            let content = self.terminal.sync();
            self.checks.push(serde_json::json!({"latin":LATIN[self.latin],"cjk":CJK[self.cjk],"zoom":SCALES[self.stage],"pixels_per_point":ui.ctx().pixels_per_point(),"cell_width":content.terminal_size.cell_width,"cell_height":content.terminal_size.cell_height,"cursor_column":content.cursor_point.column.0,"cursor_row":content.cursor_point.line.0,"display_offset":content.display_offset}));
            self.stage += 1;
            self.pending = false;
            self.since = Instant::now();
            if self.stage == SCALES.len() {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"checks":self.checks,"pty_pid":self.terminal.pty_id(),"model_requests":false})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ui.ctx().set_zoom_factor(SCALES[self.stage]);
            }
        } else if ready && !self.pending && self.since.elapsed() > Duration::from_millis(650) {
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    let mode = std::env::args().nth(2).unwrap_or_default();
    let interactive = mode == "compare";
    let latin = match mode.as_str() {
        "hack" => 0,
        "consolas" => 1,
        _ => 2,
    };
    let cjk = match std::env::args().nth(3).as_deref() {
        Some("yahei") => 1,
        Some("dengxian") => 2,
        _ => 0,
    };
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        if interactive {
            "ai-terminal font comparison"
        } else {
            "ai-terminal typography acceptance"
        },
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(interactive)
                .with_inner_size([1000.0, 480.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let (tx, _) = mpsc::channel();
            let terminal=TerminalBackend::new(996,cc.egui_ctx.clone(),tx,BackendSettings {
            shell:"powershell.exe".into(),
            args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),
                "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $e=[char]27; [Console]::Write($e+'[2J'+$e+'[H'+$e+']12;#000000'+[char]7+$e+'[48;2;48;48;48m然后还有这个问题'+$e+'[0m'+$e+'[3;1H'+'Windows PowerShell 0123456789 Il1 O0 @{} [] =>'+$e+'[5;1H'+'中文字符清晰度：输入、光标、状态、会话恢复。'+$e+'[7;1H'+'Emoji: ❤️ 💕 🧑‍💻 👍🏽 🇨🇳'+$e+'[9;1H'+'Regular '+$e+'[1m'+'Bold '+$e+'[0;3m'+'Italic '+$e+'[1;3m'+'BoldItalic'+$e+'[0m'+$e+'[11;1H'+'TYPOGRAPHY_READY'+$e+'[1;15H'+$e+'[2 q'); Start-Sleep -Seconds 30".into()],
            ..Default::default()
        })?;
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                output,
                began: Instant::now(),
                since: Instant::now(),
                stage: 0,
                pending: false,
                checks: vec![],
                interactive,
                latin,
                cjk,
            }))
        }),
    )
}
