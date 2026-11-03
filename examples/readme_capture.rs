//! Public workspace capture using the production sidebar and terminal.
//! Synthetic metadata and fixed console text only; no Agent/model/user profile.
use ai_terminal::{
    appearance, chrome,
    floating::{Item, Snapshot},
    i18n::{self, Language, Locale},
    model::{Agent, SessionColor},
    sidebar::{self, Entry, Icons},
    status::Activity,
};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Capture {
    renderer: egui_term::TextRendererSession,
    backend: TerminalBackend,
    icons: Icons,
    snapshot: Snapshot,
    output: PathBuf,
    began: Instant,
    requested: bool,
    captured: bool,
}
impl eframe::App for Capture {
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.captured || self.began.elapsed() > Duration::from_secs(18) {
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":self.captured,"version":env!("CARGO_PKG_VERSION"),"synthetic_metadata":true,"example_paths":true,"model_requests":false,"user_profiles":false,"pty_pid":self.backend.pty_id()})).unwrap()).unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        // Discard incidental typing/hover within this isolated capture window.
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        input.events.push(egui::Event::PointerGone);
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let file = std::fs::File::create(self.output.join("workspace.png")).unwrap();
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
                self.captured = true;
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let paths: Vec<_> = [
            "web-app",
            "api-service",
            "desktop-app",
            "docs",
            "workspace",
            "workspace",
            "workspace",
        ]
        .map(|name| PathBuf::from(format!("C:\\Workspace\\{name}")))
        .into();
        let entries: Vec<_> = self
            .snapshot
            .items
            .iter()
            .zip(&paths)
            .map(|(item, directory)| Entry {
                id: &item.id,
                name: &item.name,
                directory,
                agent: item.agent,
                color: item.color,
                activity: item.activity,
                unread: item.unread,
                hint: String::new(),
            })
            .collect();
        sidebar::show(ui, &entries, Some("readme-0"), false, 238.0, &self.icons);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ai_terminal::ui_text::label(ui, "网站开发 ✨", 14.0, chrome::TEXT);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("100%");
                        ui.colored_label(chrome::WORKING, "进行中");
                        ui.colored_label(chrome::SECONDARY, "C:\\Workspace\\web-app");
                    });
                });
                ui.add_space(8.0);
                let view = TerminalView::new(ui, &mut self.backend)
                    .set_font(appearance::terminal_font(14.0))
                    .set_size(ui.available_size())
                    .set_focus(true);
                ui.add(view);
            });
        if !self.requested
            && self.began.elapsed() > Duration::from_secs(3)
            && self
                .backend
                .last_content()
                .cells
                .iter()
                .map(|cell| cell.cell.c)
                .collect::<String>()
                .contains("AI Terminal")
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.requested = true;
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(
        !output.join("workspace.png").exists(),
        "Use a fresh directory"
    );
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal public README capture",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1120.0, 680.0])
                .with_transparent(true)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, Language::ZhCn);
            ai_terminal::window_frame::apply_dark(cc);
            let lines = [
                "AI Terminal  ·  多会话，也可以从容",
                "",
                "› 帮我整理今天的开发任务。",
                "",
                "• 三个项目正在并行推进，你可以随时切换回来。",
                "",
                "  网站开发    正在完善界面与交互",
                "  接口联调    有一项操作等待你确认",
                "  发布准备    构建已完成，等你回来查看",
                "",
                "  这次更新",
                "  ✓ 更清晰的头像与完整的表情 ✨",
                "  ✓ 跨屏拖动、缩放与窗口恢复更稳定",
                "  ✓ 紧凑的工作区，也保留舒适的留白",
                "",
                "  Ctrl + 滚轮缩放 · 拖动会话排序 · 点击头像返回终端",
                "",
                "回到任何一个会话，继续你的工作。",
            ];
            let expression = lines
                .into_iter()
                .map(|line| format!("'{}'", line.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join("+\"`r`n\"+");
            let script = format!(
                "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $e=[char]27; [Console]::Write($e+'[2J'+$e+'[H'+{expression}); [Console]::WriteLine(''); Start-Sleep -Seconds 25"
            );
            let (tx, _) = mpsc::channel();
            let backend = TerminalBackend::new(
                902,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: "powershell.exe".into(),
                    args: vec![
                        "-NoLogo".into(),
                        "-NoProfile".into(),
                        "-NonInteractive".into(),
                        "-Command".into(),
                        script,
                    ],
                    working_directory: Some(PathBuf::from(
                        std::env::var_os("SystemRoot").expect("Windows SystemRoot"),
                    )),
                    ..Default::default()
                },
            )
            .unwrap();
            let icons = Icons::new(&cc.egui_ctx);
            let snapshot = Snapshot {
                items: [
                    (
                        "网站开发 ✨",
                        Agent::Codex,
                        Activity::Working,
                        Some(SessionColor::Blue),
                    ),
                    (
                        "接口联调",
                        Agent::Claude,
                        Activity::Waiting,
                        Some(SessionColor::Purple),
                    ),
                    (
                        "发布准备 🚀",
                        Agent::Codex,
                        Activity::Completed,
                        Some(SessionColor::Green),
                    ),
                    ("文档整理", Agent::Claude, Activity::Idle, None),
                    ("PowerShell", Agent::Shell, Activity::Idle, None),
                    ("CMD", Agent::Cmd, Activity::Idle, None),
                    ("Git Bash", Agent::Bash, Activity::Idle, None),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (name, agent, activity, color))| Item {
                    id: format!("readme-{i}"),
                    name: name.into(),
                    agent,
                    color,
                    activity,
                    unread: matches!(activity, Activity::Completed | Activity::Waiting),
                })
                .collect(),
                active: Some("readme-0".into()),
                locale: Locale::ZhCn,
            };
            Ok(Box::new(Capture {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                backend,
                icons,
                snapshot,
                output,
                began: Instant::now(),
                requested: false,
                captured: false,
            }))
        }),
    )?;
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(verify.join("result.json")).expect("capture result"))
            .unwrap();
    assert_eq!(result["ok"], true);
    Ok(())
}
