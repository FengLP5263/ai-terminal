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
                    ai_terminal::ui_text::label(
                        ui,
                        &self.snapshot.items[0].name,
                        14.0,
                        chrome::TEXT,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("100%");
                        ui.colored_label(
                            chrome::WORKING,
                            i18n::activity(self.snapshot.locale, Activity::Working),
                        );
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
                // Wide-cell padding is present in the raw grid; compare the
                // fixed sample without whitespace, for both locales.
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>()
                .contains(if self.snapshot.locale == Locale::En {
                    "Returntoanysessionandpickupwhereyouleftoff."
                } else {
                    "回到任何一个会话，继续你的工作。"
                })
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
    let english = std::env::args().nth(2).as_deref() == Some("en");
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
            i18n::apply(
                &cc.egui_ctx,
                if english {
                    Language::En
                } else {
                    Language::ZhCn
                },
            );
            ai_terminal::window_frame::apply_dark(cc);
            let lines = if english {
                vec![
                    "AI Terminal  ·  Your agents, one workspace",
                    "",
                    "› Keep today's development tasks in view.",
                    "",
                    "• Six coding agents. Your familiar terminal.",
                    "",
                    "  Web app       Refining the interface and interactions",
                    "  API service   One action needs your approval",
                    "  Release       Build complete, ready for your review",
                    "",
                    "  Stay in the flow",
                    "  ✓ Native lifecycle events and session hooks",
                    "  ✓ Clear avatars and unread completion badges ✨",
                    "  ✓ Switch sessions without restarting your agents",
                    "",
                    "  Ctrl + wheel to zoom · Drag sessions to reorder",
                    "",
                    "Return to any session and pick up where you left off.",
                ]
            } else {
                vec![
                    "AI Terminal  ·  多会话，也可以从容",
                    "",
                    "› 帮我整理今天的开发任务。",
                    "",
                    "• 六种编程 Agent，在熟悉的终端里继续工作。",
                    "",
                    "  网站开发    正在完善界面与交互",
                    "  接口联调    有一项操作等待你确认",
                    "  发布准备    构建已完成，等你回来查看",
                    "",
                    "  保持专注",
                    "  ✓ 原生生命周期与会话级任务通知",
                    "  ✓ 清晰的头像与未读完成标记 ✨",
                    "  ✓ 随时切换会话，不用重新启动 Agent",
                    "",
                    "  Ctrl + 滚轮缩放 · 拖动会话排序",
                    "",
                    "回到任何一个会话，继续你的工作。",
                ]
            };
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
                        if english {
                            "Web app ✨"
                        } else {
                            "网站开发 ✨"
                        },
                        Agent::Codex,
                        Activity::Working,
                        Some(SessionColor::Blue),
                    ),
                    (
                        if english {
                            "API service"
                        } else {
                            "接口联调"
                        },
                        Agent::Claude,
                        Activity::Waiting,
                        Some(SessionColor::Purple),
                    ),
                    (
                        if english {
                            "Release 🚀"
                        } else {
                            "发布准备 🚀"
                        },
                        Agent::QoderCn,
                        Activity::Completed,
                        Some(SessionColor::Green),
                    ),
                    (
                        if english {
                            "Documentation"
                        } else {
                            "文档整理"
                        },
                        Agent::Kimi,
                        Activity::Working,
                        None,
                    ),
                    (
                        if english {
                            "Code review"
                        } else {
                            "代码审查"
                        },
                        Agent::ZCode,
                        Activity::Idle,
                        None,
                    ),
                    (
                        if english {
                            "Desktop app"
                        } else {
                            "桌面应用"
                        },
                        Agent::Trae,
                        Activity::Waiting,
                        None,
                    ),
                    ("PowerShell", Agent::Shell, Activity::Idle, None),
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
                locale: if english { Locale::En } else { Locale::ZhCn },
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
