//! Owned-window native tooltip and partial-lifecycle acceptance. Synthetic data only.
#[cfg(not(windows))]
fn main() {
    panic!("Native tooltip acceptance requires Windows");
}
#[cfg(windows)]
fn main() -> eframe::Result {
    windows::run()
}
#[cfg(windows)]
mod windows {
    use ai_terminal::{
        appearance,
        floating::{self, FloatingBar, Item, Snapshot},
        i18n::{Language, Locale},
        model::Agent,
        sidebar::Icons,
        status::{Activity, HookEvent, Status},
    };
    use std::{
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Gdi::*,
        UI::{Controls::*, WindowsAndMessaging::*},
    };
    #[link(name = "user32")]
    unsafe extern "system" {
        fn PrintWindow(hwnd: HWND, dc: HDC, flags: u32) -> i32;
    }
    struct Probe {
        renderer: egui_term::TextRendererSession,
        bar: FloatingBar,
        icons: Icons,
        snapshot: Snapshot,
        output: PathBuf,
        began: Instant,
        since: Instant,
        stage: usize,
        checks: Vec<serde_json::Value>,
        root_capture: bool,
        bar_capture: bool,
        capture_requested: bool,
        hover_stage: Option<usize>,
    }
    fn own_windows() -> (HWND, HWND) {
        unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            if pid != std::process::id() {
                return 1;
            }
            let mut title = [0u16; 100];
            let mut class = [0u16; 100];
            let a = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), 100) };
            let c = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), 100) };
            let result = unsafe { &mut *(data as *mut (HWND, HWND)) };
            if String::from_utf16_lossy(&title[..a as usize]) == "ai-terminal status" {
                result.0 = hwnd;
            }
            if String::from_utf16_lossy(&class[..c as usize]) == "tooltips_class32"
                && unsafe { IsWindowVisible(hwnd) } != 0
            {
                result.1 = hwnd;
            }
            1
        }
        let mut result = (std::ptr::null_mut(), std::ptr::null_mut());
        unsafe { EnumWindows(Some(visit), &mut result as *mut _ as LPARAM) };
        result
    }
    fn rect(hwnd: HWND) -> [i32; 4] {
        let mut r = RECT::default();
        assert_ne!(unsafe { GetWindowRect(hwnd, &mut r) }, 0);
        [r.left, r.top, r.right, r.bottom]
    }
    fn save(path: &Path, width: u32, height: u32, pixels: &[u8]) {
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }
    fn capture(hwnd: HWND, path: &Path) {
        unsafe {
            let r = rect(hwnd);
            let (w, h) = (r[2] - r[0], r[3] - r[1]);
            assert!(w > 0 && h > 0 && w < 8000 && h < 8000);
            let source = GetWindowDC(hwnd);
            let dc = CreateCompatibleDC(source);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                source,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            assert!(!bitmap.is_null());
            let old = SelectObject(dc, bitmap);
            let printed = PrintWindow(hwnd, dc, 2);
            // Layered common controls may not implement PrintWindow; ask the
            // same owned control to paint its client into the capture DC.
            if !std::slice::from_raw_parts(bits.cast::<u8>(), w as usize * h as usize * 4)
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 150)
            {
                SendMessageW(
                    hwnd,
                    WM_PRINTCLIENT,
                    dc as usize,
                    (PRF_CLIENT | PRF_ERASEBKGND) as isize,
                );
            }
            let mut pixels =
                std::slice::from_raw_parts(bits.cast::<u8>(), w as usize * h as usize * 4).to_vec();
            SelectObject(dc, old);
            DeleteObject(bitmap);
            DeleteDC(dc);
            ReleaseDC(hwnd, source);
            assert_ne!(printed, 0);
            for p in pixels.as_chunks_mut::<4>().0 {
                p.swap(0, 2);
                p[3] = 255;
            }
            assert!(
                pixels.as_chunks::<4>().0.iter().any(|p| p[0] > 150),
                "native text was painted"
            );
            save(path, w as u32, h as u32, &pixels);
        }
    }
    fn fixture(completed: bool) -> Status {
        let mut status = Status::default();
        let mut event:HookEvent=serde_json::from_value(serde_json::json!({"event_id":"one","local_id":"one","generation":"g","name":"UserPromptSubmit","session_id":"root","turn_id":"turn","background_running":false})).unwrap();
        status.apply_partial_lifecycle(&event, false);
        assert_eq!(status.activity, Activity::Unconfirmed);
        if completed {
            event.name = "Stop".into();
            status.apply_partial_lifecycle(&event, false);
        }
        status
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
            for event in &input.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    let raw: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    save(
                        &self
                            .output
                            .join(if input.viewport_id == floating::viewport_id() {
                                "capsule-unconfirmed.png"
                            } else {
                                "partial-status.png"
                            }),
                        image.width() as u32,
                        image.height() as u32,
                        &raw,
                    );
                    if input.viewport_id == floating::viewport_id() {
                        self.bar_capture = true;
                    } else {
                        self.root_capture = true;
                    }
                }
            }
            input
                .events
                .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
            if input.viewport_id == floating::viewport_id() && self.hover_stage != Some(self.stage)
            {
                input.events.push(if self.stage < 3 {
                    egui::Event::PointerMoved(egui::pos2(76.0, 24.0))
                } else {
                    egui::Event::PointerGone
                });
                self.hover_stage = Some(self.stage);
            }
        }
        fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            self.bar.update(self.snapshot.clone(), ctx);
            self.bar.show(ctx, Some([40.0, 650.0]));
            ctx.request_repaint_of(floating::viewport_id());
            ctx.request_repaint_after(Duration::from_millis(100));
            if self.since.elapsed() > Duration::from_secs(2) && self.stage < 3 {
                if self.stage == 1 && !(self.root_capture && self.bar_capture) {
                    if !self.capture_requested {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                            egui::UserData::default(),
                        ));
                        ctx.send_viewport_cmd_to(
                            floating::viewport_id(),
                            egui::ViewportCommand::Screenshot(egui::UserData::default()),
                        );
                        self.capture_requested = true;
                    }
                    // Keep zoom/language stable until both native framebuffers arrive.
                    return;
                }
                let (bar, tip) = own_windows();
                assert!(
                    !bar.is_null() && !tip.is_null(),
                    "owned native hint is visible"
                );
                let br = rect(bar);
                let tr = rect(tip);
                let styles = unsafe { GetWindowLongPtrW(tip, GWL_EXSTYLE) } as u32;
                assert_eq!(unsafe { GetWindow(tip, GW_OWNER) }, bar);
                assert_ne!(styles & WS_EX_NOACTIVATE, 0);
                assert_ne!(styles & WS_EX_TRANSPARENT, 0);
                assert_ne!(unsafe { GetForegroundWindow() }, tip);
                assert!(tr[2] - tr[0] > br[2] - br[0]);
                assert!(
                    tr[1] >= br[3] || tr[3] <= br[1],
                    "hint extends outside capsule, without covering avatars"
                );
                let mut text = [0u16; 4096];
                let mut info = TTTOOLINFOW {
                    cbSize: std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32,
                    hwnd: bar,
                    uId: 1,
                    lpszText: text.as_mut_ptr(),
                    ..Default::default()
                };
                unsafe {
                    SendMessageW(tip, TTM_GETTEXTW, text.len(), &mut info as *mut _ as isize)
                };
                let actual =
                    String::from_utf16_lossy(&text[..text.iter().position(|c| *c == 0).unwrap()]);
                let expected = format!(
                    "{} · ZCode · {}",
                    self.snapshot.items[1].name,
                    ai_terminal::i18n::activity(
                        self.snapshot.locale,
                        self.snapshot.items[1].activity
                    )
                );
                assert_eq!(actual, expected);
                capture(tip, &self.output.join(format!("hint-{}.png", self.stage)));
                self.checks.push(serde_json::json!({"bar":br,"hint":tr,"text":actual,"scale":ctx.zoom_factor(),"no_activate":true,"mouse_passthrough":true}));
                self.stage += 1;
                self.since = Instant::now();
                if self.stage < 3 {
                    ctx.set_zoom_factor([1.0, 1.42, 2.0][self.stage]);
                    self.snapshot.locale = if self.stage == 2 {
                        Locale::En
                    } else {
                        Locale::ZhCn
                    };
                    ai_terminal::i18n::apply(
                        ctx,
                        if self.stage == 2 {
                            Language::En
                        } else {
                            Language::ZhCn
                        },
                    );
                    self.snapshot.items[1].activity = fixture(false).activity;
                    self.snapshot.items[1].name = if self.stage == 2 {
                        "PowerShell & code review — a longer session title for native wrapping"
                            .into()
                    } else {
                        "PowerShell".into()
                    };
                }
            } else if self.stage == 3 && self.since.elapsed() > Duration::from_secs(1) {
                let (_, tip) = own_windows();
                assert!(tip.is_null(), "hint disappears on pointer leave");
                assert!(self.root_capture);
                assert!(self.bar_capture);
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"checks":self.checks,"hide_on_leave":true,"model_requests":false,"synthetic_events":true})).unwrap()).unwrap();
                self.stage = 4;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            assert!(
                self.began.elapsed() < Duration::from_secs(25),
                "probe timed out"
            );
        }
        fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("ZCode — local fixture");
                let status = fixture(false);
                assert_eq!(status.activity, Activity::Unconfirmed);
                ai_terminal::connection_ui::show(ui, Agent::ZCode, &status);
                let (avatar, _) =
                    ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                self.icons.paint(ui.painter(), avatar, Agent::ZCode);
                ai_terminal::sidebar::paint_status(
                    ui,
                    avatar,
                    status.activity,
                    status.unread,
                    egui::Color32::from_gray(16),
                );
            });
        }
    }
    pub fn run() -> eframe::Result {
        ai_terminal::launch::prepare_gui_environment();
        eprintln!("environment ready");
        let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
        std::fs::create_dir_all(&output).unwrap();
        let verify = output.clone();
        eframe::run_native(
            "ai-terminal hint acceptance",
            eframe::NativeOptions {
                renderer: eframe::Renderer::Glow,
                viewport: egui::ViewportBuilder::default()
                    .with_icon(appearance::window_icon())
                    .with_active(false)
                    .with_inner_size([840.0, 300.0]),
                ..Default::default()
            },
            Box::new(move |cc| {
                eprintln!("creation ready");
                appearance::setup(&cc.egui_ctx);
                ai_terminal::i18n::apply(&cc.egui_ctx, Language::ZhCn);
                ai_terminal::window_frame::apply_dark(cc);
                Ok(Box::new(Probe {
                    renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                    bar: FloatingBar::new(Icons::new(&cc.egui_ctx)),
                    icons: Icons::new(&cc.egui_ctx),
                    snapshot: Snapshot {
                        items: vec![
                            Item {
                                id: "kimi".into(),
                                name: "Kimi".into(),
                                agent: Agent::Kimi,
                                color: None,
                                activity: Activity::Idle,
                                unread: false,
                            },
                            Item {
                                id: "zcode".into(),
                                name: "PowerShell".into(),
                                agent: Agent::ZCode,
                                color: None,
                                activity: fixture(true).activity,
                                unread: true,
                            },
                        ],
                        active: Some("zcode".into()),
                        locale: Locale::ZhCn,
                    },
                    output,
                    began: Instant::now(),
                    since: Instant::now(),
                    stage: 0,
                    checks: vec![],
                    root_capture: false,
                    bar_capture: false,
                    capture_requested: false,
                    hover_stage: None,
                }))
            }),
        )?;
        let result: serde_json::Value = serde_json::from_slice(
            &std::fs::read(verify.join("result.json")).expect("probe result"),
        )
        .unwrap();
        assert_eq!(result["ok"], true);
        Ok(())
    }
}
