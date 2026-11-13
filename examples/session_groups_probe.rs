//! Native grouped-avatar acceptance with owned metadata and synthetic egui input only.
#[cfg(windows)]
mod native {
    use ai_terminal::{
        appearance,
        floating::{self, Action, FloatingBar, Item, Snapshot},
        i18n::{self, Language, Locale},
        model::{Agent, SessionColor, SessionGroup},
        sidebar::{self, Icons},
        status::Activity,
    };
    use egui::{Event, Pos2};
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::WindowsAndMessaging::*,
    };
    struct Probe {
        renderer: egui_term::TextRendererSession,
        bar: FloatingBar,
        icons: Icons,
        snapshot: Snapshot,
        english: bool,
        output: PathBuf,
        began: Instant,
        since: Instant,
        case: usize,
        phase: usize,
        capture: [bool; 3],
        requested: bool,
        scroll_capture: bool,
        border: bool,
        border_capture: Option<bool>,
        border_off: Option<Vec<egui::Color32>>,
        bar_geometry: Option<(usize, [i32; 4])>,
        click: usize,
        actions: usize,
        checks: Vec<serde_json::Value>,
        input_stage: [Option<(usize, usize)>; 3],
    }
    fn own(title: &str) -> Option<HWND> {
        unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
            let (wanted, found) = unsafe { &mut *(data as *mut (&str, Option<HWND>)) };
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, &mut pid);
            }
            if pid == std::process::id() && unsafe { IsWindowVisible(hwnd) } != 0 {
                let mut text = [0u16; 128];
                let n = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), 128) };
                if String::from_utf16_lossy(&text[..n as usize]) == *wanted {
                    *found = Some(hwnd);
                }
            }
            1
        }
        let mut value = (title, None);
        unsafe {
            EnumWindows(Some(visit), &mut value as *mut _ as LPARAM);
        }
        value.1
    }
    impl Probe {
        fn scale(&self) -> f32 {
            [1.0, 1.42, 2.0][self.case]
        }
        fn advance(&mut self) {
            eprintln!("case {} phase {} complete", self.case, self.phase);
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
            let slot = if input.viewport_id == floating::viewport_id() {
                1
            } else if input.viewport_id == floating::group_viewport_id() {
                2
            } else {
                0
            };
            for event in &input.events {
                if let Event::Screenshot { image, .. } = event {
                    if slot == 2 && self.phase != 4 {
                        let normal = ai_terminal::chrome::session_fill(
                            Some(SessionColor::Pink),
                            false,
                            false,
                        );
                        assert!(
                            image
                                .pixels
                                .iter()
                                .filter(|pixel| **pixel == normal)
                                .count()
                                > 200,
                            "an inactive, non-hovered member must retain its session color"
                        );
                    }
                    if slot == 1 && self.phase >= 6 {
                        self.border_capture = Some(self.border);
                        if !self.border {
                            self.border_off = Some(image.pixels.clone());
                        } else {
                            let off = self.border_off.as_ref().unwrap();
                            assert_eq!(off.len(), image.pixels.len());
                            let changed: Vec<_> = off
                                .iter()
                                .zip(&image.pixels)
                                .enumerate()
                                .filter(|(_, (a, b))| a != b)
                                .map(|(n, _)| n)
                                .collect();
                            assert!(
                                changed.len() > 20,
                                "outline toggle must change physical pixels"
                            );
                            let band = (image.height() as f32 * 13.0 / 48.0).ceil() as usize;
                            assert!(
                                changed.iter().all(|n| {
                                    let x = n % image.width();
                                    let y = n / image.width();
                                    x < band
                                        || x + band >= image.width()
                                        || y < band
                                        || y + band >= image.height()
                                }),
                                "only frame pixels may change"
                            );
                        }
                    }
                    let path = self.output.join(if self.phase >= 6 && slot == 1 {
                        format!("border-{}.png", if self.border { "on" } else { "off" })
                    } else if self.case == 2 && self.phase == 4 && slot == 2 {
                        self.scroll_capture = true;
                        "case-2-members-scrolled.png".into()
                    } else {
                        format!(
                            "case-{}-{}.png",
                            self.case,
                            ["sidebar", "bar", "members"][slot]
                        )
                    });
                    let mut encoder = png::Encoder::new(
                        std::fs::File::create(path).unwrap(),
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
                    self.capture[slot] = true;
                }
            }
            input
                .events
                .retain(|e| matches!(e, Event::Screenshot { .. }));
            let stage = Some((self.case, self.phase));
            if self.input_stage[slot] == stage && !(slot == 2 && self.phase == 3 && self.click < 2)
            {
                return;
            }
            self.input_stage[slot] = stage;
            let ratio = self.scale() / ctx.zoom_factor();
            if slot == 1 {
                input.events.push(if self.phase == 0 || self.phase == 4 {
                    Event::PointerMoved(Pos2::new(28.0, 24.0) * ratio)
                } else {
                    Event::PointerGone
                });
            } else if slot == 2 {
                if matches!(self.phase, 1..=3) {
                    let pos = Pos2::new(120.0, 110.0) * ratio;
                    input.events.push(Event::PointerMoved(pos));
                    if self.phase == 3 && self.click < 2 {
                        input.events.push(Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: self.click == 0,
                            modifiers: egui::Modifiers::NONE,
                        });
                        self.click += 1;
                    }
                } else if self.case == 2 && self.phase == 4 {
                    input
                        .events
                        .push(Event::PointerMoved(Pos2::new(120.0, 200.0) * ratio));
                    input.events.push(Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        phase: egui::TouchPhase::Move,
                        delta: egui::vec2(0.0, -10000.0),
                        modifiers: egui::Modifiers::NONE,
                    });
                } else {
                    input.events.push(Event::PointerGone);
                }
            }
        }
        fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            if ctx.viewport_id() != egui::ViewportId::ROOT {
                return;
            }
            self.bar.set_scale(ctx, self.scale());
            self.bar.set_border(ctx, self.border);
            self.bar.update(self.snapshot.clone(), ctx);
            self.bar.show(ctx, Some([160.0, 100.0]));
            for action in self.bar.actions() {
                if let Action::Activate(id) = action {
                    assert_eq!(id, "review");
                    self.actions += 1;
                    self.snapshot.active = Some(id);
                    self.advance();
                }
            }
            if ctx.input(|i| i.raw.viewports.contains_key(&floating::viewport_id())) {
                ctx.request_repaint_of(floating::viewport_id());
            }
            if ctx.input(|i| i.raw.viewports.contains_key(&floating::group_viewport_id())) {
                ctx.request_repaint_of(floating::group_viewport_id());
            }
            ctx.request_repaint_after(Duration::from_millis(40));
            let elapsed = self.since.elapsed();
            match self.phase {
                0 if elapsed > Duration::from_millis(600) => {
                    assert!(own("ai-terminal group").is_some());
                    self.advance();
                }
                1 if elapsed > Duration::from_millis(600) => {
                    let popup =
                        own("ai-terminal group").expect("crossing the gap keeps members visible");
                    let bar = own("ai-terminal status").unwrap();
                    let style = unsafe { GetWindowLongPtrW(popup, GWL_EXSTYLE) } as u32;
                    assert_ne!(style & WS_EX_NOACTIVATE, 0);
                    assert_eq!(style & WS_EX_TRANSPARENT, 0);
                    assert_eq!(unsafe { GetWindow(popup, GW_OWNER) }, bar);
                    assert_ne!(unsafe { GetForegroundWindow() }, popup);
                    let mut rect = RECT::default();
                    unsafe {
                        GetWindowRect(popup, &mut rect);
                    }
                    self.checks.push(serde_json::json!({"scale":self.scale(),"rectangle":[rect.left,rect.top,rect.right,rect.bottom],"interactive":true,"no_activate":true,"gap_crossed":true}));
                    self.advance();
                }
                2 => {
                    if !self.requested {
                        for id in [
                            egui::ViewportId::ROOT,
                            floating::viewport_id(),
                            floating::group_viewport_id(),
                        ] {
                            ctx.send_viewport_cmd_to(
                                id,
                                egui::ViewportCommand::Screenshot(egui::UserData::default()),
                            );
                        }
                        self.requested = true;
                    }
                    if self.capture.iter().all(|c| *c) {
                        self.advance();
                    }
                }
                4 if elapsed > Duration::from_millis(500) => {
                    if self.case == 2 && !self.scroll_capture {
                        ctx.send_viewport_cmd_to(
                            floating::group_viewport_id(),
                            egui::ViewportCommand::Screenshot(egui::UserData::default()),
                        );
                    } else {
                        self.advance();
                    }
                }
                5 if elapsed > Duration::from_millis(700) => {
                    assert!(
                        own("ai-terminal group").is_none(),
                        "leaving both surfaces closes the popup"
                    );
                    if self.case == 2 {
                        assert_eq!(self.actions, 3);
                        let bar = own("ai-terminal status").unwrap();
                        let mut rect = RECT::default();
                        unsafe {
                            GetWindowRect(bar, &mut rect);
                        }
                        self.bar_geometry =
                            Some((bar as usize, [rect.left, rect.top, rect.right, rect.bottom]));
                        self.border = false;
                        self.requested = false;
                        self.advance();
                    } else {
                        self.case += 1;
                        self.phase = 0;
                        self.since = Instant::now();
                        self.capture = [false; 3];
                        self.requested = false;
                        self.click = 0;
                        if self.case == 2 {
                            for n in 4..=16 {
                                let id = format!("member-{n}");
                                self.snapshot.groups[0].members.push(id.clone());
                                self.snapshot.items.push(Item {
                                    id,
                                    name: format!("Team member {n}"),
                                    agent: Agent::Shell,
                                    color: None,
                                    activity: Activity::Idle,
                                    unread: false,
                                });
                            }
                        }
                        self.snapshot.locale = if self.english || self.case == 2 {
                            Locale::En
                        } else {
                            Locale::ZhCn
                        };
                        i18n::apply(
                            ctx,
                            if self.english || self.case == 2 {
                                Language::En
                            } else {
                                Language::ZhCn
                            },
                        );
                    }
                }
                6 | 7 if elapsed > Duration::from_millis(400) => {
                    let bar = own("ai-terminal status").unwrap();
                    let mut rect = RECT::default();
                    unsafe {
                        GetWindowRect(bar, &mut rect);
                    }
                    assert_eq!(
                        self.bar_geometry,
                        Some((bar as usize, [rect.left, rect.top, rect.right, rect.bottom]))
                    );
                    if !self.requested {
                        ctx.send_viewport_cmd_to(
                            floating::viewport_id(),
                            egui::ViewportCommand::Screenshot(egui::UserData::default()),
                        );
                        self.requested = true;
                    }
                    if self.border_capture == Some(self.border) {
                        if self.phase == 6 {
                            self.border = true;
                            self.border_capture = None;
                            self.requested = false;
                            self.advance();
                        } else {
                            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"checks":self.checks,"member_activations":self.actions,"hide_on_leave":true,"long_group_scroll":self.scroll_capture,"border_toggled":true,"stable_native_geometry":true,"border_changes_only_edge_pixels":true,"model_requests":false,"synthetic_events":true})).unwrap()).unwrap();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
                _ => {}
            }
            assert!(
                self.began.elapsed() < Duration::from_secs(35),
                "native group probe timed out: phase {} case {}",
                self.phase,
                self.case
            );
        }
        fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
            let entries: Vec<_> = self
                .snapshot
                .items
                .iter()
                .map(|item| sidebar::Entry {
                    id: &item.id,
                    name: &item.name,
                    directory: std::path::Path::new("sample-project"),
                    agent: item.agent,
                    color: item.color,
                    activity: item.activity,
                    unread: item.unread,
                    hint: String::new(),
                })
                .collect();
            sidebar::show_with_groups(
                ui,
                &entries,
                self.snapshot.active.as_deref(),
                self.case == 2,
                if self.case == 1 { 190.0 } else { 240.0 },
                &self.icons,
                &self.snapshot.groups,
            );
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::BLACK))
                .show(ui, |ui| {
                    ui.add_space(20.0);
                    ui.heading("会话分组 / Session groups");
                    ui.label("右键会话 → 分组；浮窗悬停查看队伍，点击成员进入。");
                    ui.label("Owned fixtures · synthetic input · no Agent/model requests");
                });
        }
    }
    pub fn run() -> eframe::Result {
        eprintln!("preparing native environment");
        ai_terminal::launch::prepare_gui_environment();
        eprintln!("environment ready");
        let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
        let english = std::env::args().nth(2).is_some_and(|arg| arg == "en");
        std::fs::create_dir_all(&output).unwrap();
        let verify = output.clone();
        eframe::run_native(
            "ai-terminal group acceptance",
            eframe::NativeOptions {
                renderer: eframe::Renderer::Glow,
                viewport: egui::ViewportBuilder::default()
                    .with_icon(appearance::window_icon())
                    .with_active(false)
                    .with_inner_size([900.0, 400.0]),
                ..Default::default()
            },
            Box::new(move |cc| {
                eprintln!("creation context ready");
                appearance::setup(&cc.egui_ctx);
                eprintln!("appearance ready");
                ai_terminal::window_frame::apply_dark(cc);
                i18n::apply(
                    &cc.egui_ctx,
                    if english {
                        Language::En
                    } else {
                        Language::ZhCn
                    },
                );
                let icons = Icons::new(&cc.egui_ctx);
                eprintln!("icons ready");
                Ok(Box::new(Probe {
                    renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                    bar: FloatingBar::new(icons.clone()),
                    icons,
                    english,
                    snapshot: Snapshot {
                        items: [
                            ("lead", "开发负责人 ❤️", Agent::Claude, Activity::Working),
                            ("review", "代码审查 🧑‍💻", Agent::Codex, Activity::Waiting),
                            ("tests", "测试与验证", Agent::Shell, Activity::Completed),
                            ("single", "工具队负责人", Agent::Bash, Activity::Idle),
                            ("tools-check", "工具队验证", Agent::Shell, Activity::Idle),
                        ]
                        .into_iter()
                        .map(|(id, name, agent, activity)| Item {
                            id: id.into(),
                            name: if english {
                                match id {
                                    "lead" => "Product planning ❤️",
                                    "review" => "Code review 🧑‍💻",
                                    "tests" => "Tests and checks",
                                    "single" => "Developer tools",
                                    _ => "Tooling checks",
                                }
                            } else {
                                name
                            }
                            .into(),
                            agent,
                            color: if id == "lead" {
                                Some(SessionColor::Green)
                            } else if id == "review" {
                                Some(SessionColor::Blue)
                            } else if id == "tests" {
                                Some(SessionColor::Pink)
                            } else {
                                None
                            },
                            activity,
                            unread: activity == Activity::Completed,
                        })
                        .collect(),
                        groups: vec![
                            SessionGroup {
                                id: "team".into(),
                                name: if english {
                                    "Product team"
                                } else {
                                    "项目研发队"
                                }
                                .into(),
                                members: vec!["lead".into(), "review".into(), "tests".into()],
                            },
                            SessionGroup {
                                id: "support".into(),
                                name: if english {
                                    "Tooling team"
                                } else {
                                    "工具支持队"
                                }
                                .into(),
                                members: vec!["single".into(), "tools-check".into()],
                            },
                        ],
                        active: Some("lead".into()),
                        locale: if english { Locale::En } else { Locale::ZhCn },
                    },
                    output,
                    began: Instant::now(),
                    since: Instant::now(),
                    case: 0,
                    phase: 0,
                    capture: [false; 3],
                    requested: false,
                    scroll_capture: false,
                    border: true,
                    border_capture: None,
                    border_off: None,
                    bar_geometry: None,
                    click: 0,
                    actions: 0,
                    checks: vec![],
                    input_stage: [None; 3],
                }))
            }),
        )?;
        let result: serde_json::Value = serde_json::from_slice(
            &std::fs::read(verify.join("result.json")).expect("native result"),
        )
        .unwrap();
        assert_eq!(result["ok"], true);
        Ok(())
    }
}
#[cfg(windows)]
fn main() -> eframe::Result {
    native::run()
}
#[cfg(not(windows))]
fn main() {
    eprintln!("session_groups_probe requires Windows");
    std::process::exit(1);
}
