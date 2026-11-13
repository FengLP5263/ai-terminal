//! Native multi-window probe using production settings, island rendering and restore commands.
//! Synthetic metadata only; never starts an Agent or submits a model request.
use ai_terminal::{
    appearance,
    floating::{self, Action, FloatingBar, Item, Snapshot},
    i18n::{self, Language, Locale},
    model::UiSettings,
    settings,
    sidebar::Icons,
    status::Activity,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
struct Probe {
    renderer: egui_term::TextRendererSession,
    bar: FloatingBar,
    snapshot: Snapshot,
    prefs: UiSettings,
    open: bool,
    began: Instant,
    stage: u8,
    click_stage: u8,
    output: PathBuf,
    shots: usize,
    minimized: bool,
    restored: bool,
    selected: bool,
    root_frames: u64,
    frames_while_minimized: u64,
    closed: bool,
    reopened: bool,
    island_visible_minimized: bool,
    island_topmost: bool,
}
fn save_image(path: &std::path::Path, image: &egui::ColorImage) {
    let file = std::fs::File::create(path).unwrap();
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
                .flat_map(|c| c.to_array())
                .collect::<Vec<_>>(),
        )
        .unwrap();
}
impl eframe::App for Probe {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let child = input.viewport_id == floating::viewport_id();
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = match (child, self.stage >= 4) {
                    (true, false) => "island-zh.png",
                    (false, false) => "settings-zh.png",
                    (true, true) => "island-en.png",
                    (false, true) => "settings-en.png",
                };
                save_image(&self.output.join(name), image);
                self.shots += 1;
            }
        }
        if child && self.stage == 3 && self.click_stage < 2 {
            input.focused = true;
            input
                .viewports
                .entry(floating::viewport_id())
                .or_default()
                .focused = Some(true);
            let pos = egui::pos2(124.0, 24.0);
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: self.click_stage == 0,
                modifiers: egui::Modifiers::NONE,
            });
            self.click_stage += 1;
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let elapsed = self.began.elapsed().as_secs_f32();
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if minimized {
            self.minimized = true;
            let (visible, topmost) = owned_island_state();
            self.island_visible_minimized |= visible;
            self.island_topmost |= topmost;
        }
        if self.selected && self.minimized && !minimized {
            self.restored = true;
        }
        for action in self.bar.actions() {
            match action {
                Action::Activate(id) => {
                    self.selected = id == "session-2";
                    self.snapshot.active = Some(id);
                    floating::restore_main(ctx);
                }
                Action::Disable => self.prefs.floating_enabled = false,
                Action::Position(pos) => self.prefs.floating_position_pixels = Some(pos),
            }
        }
        match self.stage {
            0 if elapsed > 1.5 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                ctx.send_viewport_cmd_to(
                    floating::viewport_id(),
                    egui::ViewportCommand::Screenshot(egui::UserData::default()),
                );
                self.stage = 1;
            }
            1 if elapsed > 3.0 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                self.stage = 2;
            }
            2 if elapsed > 4.0 => {
                self.snapshot.items[0].activity = Activity::Completed;
                self.stage = 3;
                ctx.request_repaint_of(floating::viewport_id());
            }
            3 if elapsed > 6.0 => {
                self.prefs.language = Language::En;
                i18n::apply(ctx, Language::En);
                self.snapshot.locale = Locale::En;
                for (i, item) in self.snapshot.items.iter_mut().take(6).enumerate() {
                    item.name = [
                        "Web app ✨",
                        "API service",
                        "Release 🚀",
                        "Documentation",
                        "Code review",
                        "Desktop app",
                    ][i]
                        .into();
                }
                self.stage = 4;
            }
            4 if elapsed > 7.0 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                ctx.send_viewport_cmd_to(
                    floating::viewport_id(),
                    egui::ViewportCommand::Screenshot(egui::UserData::default()),
                );
                self.stage = 5;
            }
            5 if elapsed > 9.0 => {
                self.prefs.floating_enabled = false;
                self.stage = 6;
            }
            6 if elapsed > 10.0 => {
                self.closed =
                    !ctx.input(|i| i.raw.viewports.contains_key(&floating::viewport_id()));
                self.prefs.floating_enabled = true;
                self.stage = 7;
            }
            7 if elapsed > 11.0 => {
                self.reopened =
                    ctx.input(|i| i.raw.viewports.contains_key(&floating::viewport_id()));
                let result = serde_json::json!({"ok":self.minimized&&self.restored&&self.selected&&self.shots>=4 && self.closed && self.reopened && self.island_visible_minimized && self.island_topmost,"main_minimized":self.minimized,"main_restored":self.restored,"session_clicked":self.selected,"screenshots":self.shots,"island_closed":self.closed,"island_reopened":self.reopened,"island_visible_while_main_minimized":self.island_visible_minimized,"island_topmost":self.island_topmost,"root_frames":self.root_frames,"root_frames_while_minimized":self.frames_while_minimized,"real_model_requests":false});
                std::fs::write(
                    self.output.join("result.json"),
                    serde_json::to_vec_pretty(&result).unwrap(),
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                self.stage = 8;
            }
            _ => {}
        }
        if self.prefs.floating_enabled {
            self.bar.update(self.snapshot.clone(), ctx);
            self.bar.show(ctx, self.prefs.floating_position_pixels);
        }
        if elapsed > 20.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.root_frames += 1;
        if ui.input(|i| i.viewport().minimized.unwrap_or(false)) {
            self.frames_while_minimized += 1;
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(appearance::TERMINAL_BG)
                    .inner_margin(20),
            )
            .show(ui, |ui| {
                ui.heading("ai-terminal · native multi-window check");
                ui.label("Synthetic task states. No Agent or model request.");
            });
        if settings::show(ui.ctx(), &mut self.open, &mut self.prefs) {
            i18n::apply(ui.ctx(), self.prefs.language);
            self.snapshot.locale = self.prefs.language.resolve();
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("island_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal island probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 680.0])
                .with_transparent(true)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, Language::ZhCn);
            let icons = Icons::new(&cc.egui_ctx);
            let snapshot = Snapshot {
                groups: Vec::new(),
                items: [
                    ("网站开发 ✨", Activity::Working),
                    ("接口联调", Activity::Waiting),
                    ("发布准备 🚀", Activity::Completed),
                    ("文档整理", Activity::Working),
                    ("代码审查", Activity::Idle),
                    ("桌面应用", Activity::Failed),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (name, activity))| Item {
                    id: format!("session-{i}"),
                    name: name.into(),
                    agent: [
                        ai_terminal::model::Agent::Codex,
                        ai_terminal::model::Agent::Claude,
                        ai_terminal::model::Agent::QoderCn,
                        ai_terminal::model::Agent::Kimi,
                        ai_terminal::model::Agent::ZCode,
                        ai_terminal::model::Agent::Trae,
                    ][i],
                    color: None,
                    activity,
                    unread: true,
                })
                .chain((6..12).map(|i| Item {
                    id: format!("session-{i}"),
                    name: format!("Session {i}"),
                    agent: ai_terminal::model::Agent::Shell,
                    color: None,
                    activity: Activity::Idle,
                    unread: false,
                }))
                .collect(),
                active: Some("session-0".into()),
                locale: Locale::ZhCn,
            };
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                bar: FloatingBar::new(icons),
                snapshot,
                prefs: UiSettings {
                    language: Language::ZhCn,
                    floating_enabled: true,
                    floating_position_pixels: Some([60.0, 60.0]),
                    ..Default::default()
                },
                open: true,
                began: Instant::now(),
                stage: 0,
                click_stage: 0,
                output,
                shots: 0,
                minimized: false,
                restored: false,
                selected: false,
                root_frames: 0,
                frames_while_minimized: 0,
                closed: false,
                reopened: false,
                island_visible_minimized: false,
                island_topmost: false,
            }))
        }),
    )?;
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(verify.join("result.json")).expect("probe report"))
            .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    Ok(())
}

#[cfg(windows)]
fn owned_island_state() -> (bool, bool) {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::*,
    };
    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> i32 {
        // Read window metadata for this probe process only; never send OS input.
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            if pid == std::process::id() {
                let mut title = [0u16; 80];
                let len = GetWindowTextW(window, title.as_mut_ptr(), 80);
                if String::from_utf16_lossy(&title[..len as usize]) == "ai-terminal status" {
                    let result = &mut *(data as *mut (bool, bool));
                    *result = (
                        IsWindowVisible(window) != 0,
                        GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0,
                    );
                }
            }
            1
        }
    }
    let mut result = (false, false);
    unsafe {
        EnumWindows(Some(visit), &mut result as *mut _ as LPARAM);
    };
    result
}
#[cfg(not(windows))]
fn owned_island_state() -> (bool, bool) {
    (false, false)
}
