//! Local ANSI/ConPTY visual fixture. No Agent launch or model request.
use ai_terminal::{
    appearance,
    model::{self, Agent, UiSettings},
    sidebar::{self, Entry, Icons},
    status::Activity,
};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    icons: Icons,
    preferences: UiSettings,
    ids: Vec<String>,
    active: String,
    output: PathBuf,
    began: Instant,
    stage: usize,
    stage_started: Instant,
    stage_frame: u64,
    pending: Option<&'static str>,
    terminal_rect: egui::Rect,
    wheel_sent: bool,
    name: String,
    selected_name: bool,
    zoom_verified: bool,
    many_sessions: bool,
    rail_scrolled: bool,
    threshold_mode: bool,
    stable_rect: Option<(usize, egui::Rect)>,
    stability_checks: usize,
}
impl Probe {
    fn tick(&mut self, ctx: &egui::Context) {
        if self.began.elapsed() > Duration::from_secs(18) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
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
        if self.threshold_mode && self.stage == 2 && !self.rail_scrolled {
            self.rail_scrolled = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(900.0, 550.0)));
            self.stage_started = Instant::now();
            self.stage_frame = ctx.cumulative_frame_nr();
        }
        if self.many_sessions && self.stage == 2 {
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(49.0, 300.0)));
            if !self.rail_scrolled {
                self.rail_scrolled = true;
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, -3.0),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                });
            }
        }
        if self.stage == 1 && !self.wheel_sent {
            self.wheel_sent = true;
            input
                .events
                .push(egui::Event::PointerMoved(self.terminal_rect.center()));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 12.0),
                modifiers: egui::Modifiers::CTRL,
                phase: egui::TouchPhase::Move,
            });
        }
        if let Some(scale) = ai_terminal::zoom::input(input, self.preferences.ui_scale) {
            self.preferences.ui_scale = scale;
            ctx.set_zoom_factor(scale);
            self.zoom_verified = true;
            self.stage_started = Instant::now();
            self.stage_frame = ctx.cumulative_frame_nr();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let entries: Vec<_> = self
            .ids
            .iter()
            .map(|id| {
                let (name, agent, activity, unread) = match id.as_str() {
                    "codex" => ("接口联调 👩🏽‍💻", Agent::Codex, Activity::Completed, true),
                    "claude" => ("AI 终端 ❤️🚀", Agent::Claude, Activity::Working, false),
                    "cmd" => ("CMD", Agent::Cmd, Activity::Idle, false),
                    "bash" => ("Git Bash", Agent::Bash, Activity::Idle, false),
                    _ => ("PowerShell", Agent::Shell, Activity::Waiting, false),
                };
                Entry {
                    id,
                    name,
                    directory: std::path::Path::new("C:/Projects/ai-terminal"),
                    agent,
                    color: match id.as_str() {
                        "codex" => Some(model::SessionColor::Blue),
                        "claude" => Some(model::SessionColor::Purple),
                        "cmd" => Some(model::SessionColor::Green),
                        "bash" => Some(model::SessionColor::Orange),
                        _ => None,
                    },
                    activity,
                    unread,
                    hint: "本地样例，状态为合成事件".into(),
                }
            })
            .collect();
        let actions = sidebar::show(
            ui,
            &entries,
            Some(&self.active),
            self.preferences.sidebar_collapsed,
            self.preferences.sidebar_width,
            &self.icons,
        );
        if self.threshold_mode && ctx.cumulative_frame_nr().saturating_sub(self.stage_frame) >= 8 {
            let rect = ctx
                .read_response(egui::Id::new(("session-row", "codex")))
                .unwrap()
                .rect;
            if let Some((stage, expected)) = self.stable_rect
                && stage == self.stage
            {
                assert_eq!(
                    rect, expected,
                    "native collapsed sidebar geometry oscillates"
                );
            }
            self.stable_rect = Some((self.stage, rect));
            self.stability_checks += 1;
        }
        if !self.preferences.sidebar_collapsed {
            self.preferences.sidebar_width = actions.width;
        }
        if actions.toggle {
            self.preferences.sidebar_collapsed = !self.preferences.sidebar_collapsed;
        }
        if let Some(id) = actions.select {
            self.active = id;
        }
        if let Some((from, to, after)) = actions.reorder {
            let a = self.ids.iter().position(|s| s == &from).unwrap();
            let b = self.ids.iter().position(|s| s == &to).unwrap();
            model::reorder(&mut self.ids, a, b, after);
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("本地 ANSI / ConPTY 渲染样例");
                    ui.label(format!("{:.0}%", self.preferences.ui_scale * 100.0));
                });
                ai_terminal::ui_text::label(ui, &self.name, 14.0, egui::Color32::LIGHT_GRAY);
                let editor_id = egui::Id::new("emoji-name-editor");
                if self.stage == 1 && !self.selected_name {
                    ctx.memory_mut(|m| m.request_focus(editor_id));
                    if let Some(mut state) = egui::text_edit::TextEditState::load(&ctx, editor_id) {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(self.name.chars().count()),
                            )));
                        state.store(&ctx, editor_id);
                        self.selected_name = true;
                    }
                }
                ai_terminal::ui_text::edit(ui, &mut self.name, editor_id);
                ui.add_space(8.0);
                self.terminal_rect =
                    egui::Rect::from_min_size(ui.cursor().min, ui.available_size());
                let view = TerminalView::new(ui, &mut self.terminal)
                    .set_font(appearance::terminal_font(14.0))
                    .set_focus(true)
                    .set_size(ui.available_size());
                ui.add(view);
            });
        if let Some(image) = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        }) && let Some(name) = self.pending.take()
        {
            let file = std::fs::File::create(self.output.join(name)).unwrap();
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
            self.stage += 1;
            self.stage_started = Instant::now();
            self.stage_frame = ctx.cumulative_frame_nr();
            if self.stage == 2 {
                self.preferences.sidebar_collapsed = true;
            }
            if self.stage == 3 {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":self.zoom_verified,"ui_scale":self.preferences.ui_scale,"collapsed":self.preferences.sidebar_collapsed,"pixels_per_point":ctx.pixels_per_point(),"pty_pid":self.terminal.pty_id(),"model_request_sent":false,"threshold_mode":self.threshold_mode,"stability_checks":self.stability_checks})).unwrap()).unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.pending.is_none()
            && self.stage < 3
            && self.terminal.plain_text().contains("FIXTURE_READY")
            && self.stage_started.elapsed() > Duration::from_secs(1)
            && ctx.cumulative_frame_nr().saturating_sub(self.stage_frame) >= 3
            && (!self.threshold_mode
                || ctx.cumulative_frame_nr().saturating_sub(self.stage_frame) >= 32)
        {
            self.pending = Some(["expanded.png", "zoom.png", "collapsed.png"][self.stage]);
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
            .expect("workspace_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let width = std::env::args()
        .nth(2)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1160.0)
        .clamp(720.0, 1600.0);
    let many_sessions = std::env::args().nth(3).as_deref() == Some("many");
    let threshold_mode = std::env::args().nth(3).as_deref() == Some("threshold");
    eframe::run_native(
        "ai-terminal workspace visual probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(ai_terminal::appearance::window_icon())
                .with_inner_size([width, if threshold_mode { 550.0 } else { 760.0 }]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let (tx, _) = mpsc::channel();
            let terminal = TerminalBackend::new(
                902,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: "powershell.exe".into(),
                    args: vec!["-NoLogo".into(), "-NoProfile".into(),"-NonInteractive".into(),"-Command".into(),"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $e=[char]27; [Console]::Write($e+'[2J'+$e+'[H'+$e+'[38;2;217;119;87m'+$e+'[1m'+'Claude Code'+$e+'[0m'+'  ·  ANSI fixture'+\"`r`n\"+'Hack Regular / Bold / Italic · 中文基线与清晰度'+\"`r`n`r`n\"+'❯ 你好呀'+\"`r`n`r`n\"+'● 你好！我们来验证原生终端的显示效果。'+\"`r`n`r`n\"+'  - '+$e+'[1m'+'AI terminal'+$e+'[0m'+'：纯黑背景、紧凑行距与完整的中文输入。'+\"`r`n\"+'  - 字体对齐物理像素，Ctrl＋滚轮缩放。'+\"`r`n\"+'  - 会话支持拖动排序、编辑名称和收起为头像栏。'+\"`r`n`r`n\"+'  ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789'+\"`r`n\"+'  abcdefghijklmnopqrstuvwxyz {}[]() => !='+\"`r`n\"+'  中文：傻妞桌宠、终端会话、渲染质量。'+\"`r`n\"+'  😀 😂 😡 💣 💥 🔥 ✨ ❤️ 👋 👋🏽 👩‍💻'+\"`r`n`r`n\"+$e+'[3m'+'  Italic / '+$e+'[1m'+'Bold Italic'+$e+'[0m'+\"`r`n\"+'  ┌───────────────┐'+\"`r`n\"+'  │ Native ConPTY │'+\"`r`n\"+'  └───────────────┘'+\"`r`n\"); [Console]::WriteLine('FIXTURE_READY'); Start-Sleep -Seconds 25".into()],
                    ..Default::default()
                },
            )
            .unwrap();

            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                icons: Icons::new(&cc.egui_ctx),
                preferences: UiSettings {
                    sidebar_collapsed: threshold_mode,
                    ..Default::default()
                },
                ids: ["codex", "claude", "shell", "cmd", "bash"]
                    .into_iter()
                    .map(String::from)
                    .chain(
                        (0..if many_sessions {
                            25
                        } else if threshold_mode {
                            2
                        } else {
                            0
                        })
                            .map(|i| format!("shell-{i}")),
                    )
                    .collect(),
                active: "claude".into(),
                output,
                began: Instant::now(),
                stage: 0,
                stage_started: Instant::now(),
                stage_frame: 0,
                pending: None,
                terminal_rect: egui::Rect::NOTHING,
                wheel_sent: false,
                name: "命名 😀 👋🏽 👩‍💻 ❤️ 🚀".into(),
                selected_name: false,
                zoom_verified: false,
                many_sessions,
                rail_scrolled: false,
                threshold_mode,
                stable_rect: None,
                stability_checks: 0,
            }))
        }),
    )
}
