//! Native background-transparency/settings captures with fixed metadata, no PTY or model calls.
use ai_terminal::{
    appearance,
    floating::{self, Action, FloatingBar, Item, Snapshot},
    i18n::{self, Language},
    model::{Agent, UiSettings},
    settings,
    sidebar::Icons,
    status::Activity,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
// Keep the owned capture fixture clear of the system IME candidate strip.
const BACKDROP_Y: i32 = 340;
const PAD: usize = 16;
const CASES: [(Language, f32); 5] = [
    (Language::ZhCn, 1.0),
    (Language::ZhCn, 0.5),
    (Language::ZhCn, 0.2),
    (Language::En, 0.5),
    (Language::En, 0.0),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    _backdrop: Backdrop,
    bar: FloatingBar,
    prefs: UiSettings,
    open: bool,
    output: PathBuf,
    stage: usize,
    frames: usize,
    since: Instant,
    began: Instant,
    pending: bool,
    shots: [bool; 2],
    clicked: u8,
    activated: bool,
    composition_reset: bool,
    records: Vec<serde_json::Value>,
}
impl eframe::App for Probe {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        assert!(
            self.began.elapsed() < Duration::from_secs(30),
            "Opacity capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(40));
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.stage >= CASES.len() {
            return;
        }
        let child = input.viewport_id == floating::viewport_id();
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = format!(
                    "opacity-{}-{}.png",
                    self.stage,
                    if child { "bar" } else { "settings" }
                );
                if child {
                    let scale = image.height() as f32 / 48.0;
                    let pixel = |x: f32, y: f32| {
                        image.pixels[(y * scale).round() as usize * image.width()
                            + (x * scale).round() as usize]
                    };
                    let alpha = pixel(4.0, 24.0).a();
                    assert!(
                        (i32::from(alpha) - (255.0 * CASES[self.stage].1).round() as i32).abs()
                            <= 2,
                        "Unexpected native background alpha: {alpha}"
                    );
                    assert_eq!(pixel(28.0, 24.0).a(), 255, "Avatar must remain opaque");
                    let native = owned_bar().expect("Owned native bar");
                    let (desktop, backdrop) =
                        desktop_capture(native, if self.stage == 3 { 128 } else { 255 });
                    let desktop_pixel = desktop.pixels[((24.0 * scale).round() as usize + PAD)
                        * desktop.width()
                        + (4.0 * scale).round() as usize
                        + PAD];
                    save_image(
                        &self
                            .output
                            .join(format!("opacity-{}-desktop.png", self.stage)),
                        &desktop,
                    );
                    let desktop_avatar = desktop.pixels[((24.0 * scale).round() as usize + PAD)
                        * desktop.width()
                        + (28.0 * scale).round() as usize
                        + PAD];
                    let mut composite_error = 0_u8;
                    let mut halo_error = 0_u8;
                    for y in 0..desktop.height() {
                        for x in 0..desktop.width() {
                            let actual = desktop.pixels[y * desktop.width() + x].to_array();
                            if x >= PAD
                                && y >= PAD
                                && x < PAD + image.width()
                                && y < PAD + image.height()
                            {
                                let source =
                                    image.pixels[(y - PAD) * image.width() + x - PAD].to_array();
                                for c in 0..3 {
                                    let expected = (u32::from(source[c])
                                        + (u32::from(backdrop[c]) * (255 - u32::from(source[3]))
                                            + 127)
                                            / 255)
                                        .min(255)
                                        as u8;
                                    composite_error =
                                        composite_error.max(actual[c].abs_diff(expected));
                                }
                            } else {
                                for c in 0..3 {
                                    halo_error = halo_error.max(actual[c].abs_diff(backdrop[c]));
                                }
                            }
                        }
                    }
                    self.records.push(serde_json::json!({"stage":self.stage,"opacity":CASES[self.stage].1,"background_alpha":alpha,"avatar_alpha":255,"native":native,"desktop_rgb":[desktop_pixel.r(),desktop_pixel.g(),desktop_pixel.b()],"desktop_avatar_rgb":[desktop_avatar.r(),desktop_avatar.g(),desktop_avatar.b()],"backdrop_rgb":backdrop,"client_composite_error":composite_error,"outside_halo_error":halo_error}));
                }
                save_image(&self.output.join(name), image);
                self.shots[usize::from(child)] = true;
            }
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        if child && self.stage == 4 && self.shots.iter().all(|s| *s) && self.clicked < 2 {
            input.focused = true;
            input
                .viewports
                .entry(floating::viewport_id())
                .or_default()
                .focused = Some(true);
            let pos = floating::content_transform(ctx, self.prefs.floating_scale)
                * egui::pos2(76.0, 24.0);
            input.events.extend([
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: self.clicked == 0,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            self.clicked += 1;
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= CASES.len() {
            return;
        }
        if self.frames == 0 {
            let (language, opacity) = CASES[self.stage];
            self.prefs.language = language;
            self.prefs.floating_opacity = opacity;
            i18n::apply(ui.ctx(), language);
            self._backdrop.gray(self.stage == 3);
            self.since = Instant::now();
        }
        settings::show(ui.ctx(), &mut self.open, &mut self.prefs);
        self.bar.set_scale(ui.ctx(), self.prefs.floating_scale);
        self.bar.set_opacity(ui.ctx(), self.prefs.floating_opacity);
        self.bar.set_border(ui.ctx(), self.prefs.floating_border);
        self.bar.update(
            Snapshot {
                groups: Vec::new(),
                items: [Agent::Shell, Agent::Bash, Agent::Codex, Agent::Claude]
                    .into_iter()
                    .enumerate()
                    .map(|(n, agent)| Item {
                        id: n.to_string(),
                        name: format!("Session {n} ✨"),
                        agent,
                        color: None,
                        activity: if n == 2 {
                            Activity::Completed
                        } else {
                            Activity::Idle
                        },
                        unread: n == 2,
                    })
                    .collect(),
                active: Some("0".into()),
                locale: self.prefs.language.resolve(),
            },
            ui.ctx(),
        );
        for action in self.bar.actions() {
            if action == Action::Activate("1".into()) {
                self.activated = true;
            }
        }
        self.bar
            .show(ui.ctx(), Some([60.0, (BACKDROP_Y + 20) as f32]));
        if self.stage == 1 && self.frames == 4 && !self.composition_reset {
            let native = owned_bar().expect("Owned native bar before reset");
            simulate_composition_reset(native[0]);
            self.composition_reset = true;
        }
        self.frames += 1;
        ui.ctx().request_repaint_of(floating::viewport_id());
        if self.shots.iter().all(|s| *s) && (self.stage != 4 || self.activated) {
            self.stage += 1;
            self.frames = 0;
            self.pending = false;
            self.shots = [false; 2];
            if self.stage == CASES.len() {
                assert!(
                    self.activated,
                    "Transparent bar did not activate its session"
                );
                assert!(
                    self.records
                        .iter()
                        .all(|r| r["native"] == self.records[0]["native"]),
                    "Opacity must not recreate or move the native window"
                );
                let channel =
                    |stage: usize| self.records[stage]["desktop_rgb"][0].as_i64().unwrap();
                let composition_ok = channel(4) >= 250
                    && self.records[4]["background_alpha"].as_u64().unwrap() == 0
                    && channel(0) < 25
                    && channel(1) > 40
                    && channel(2) > 150
                    && channel(2) > channel(1) + 20
                    && (channel(3)
                        - (self.records[3]["backdrop_rgb"][0].as_i64().unwrap() + 8) / 2)
                        .abs()
                        <= 5
                    && self.records.iter().all(|r| {
                        (0..3).all(|c| {
                            (r["desktop_avatar_rgb"][c].as_i64().unwrap()
                                - self.records[0]["desktop_avatar_rgb"][c].as_i64().unwrap())
                            .abs()
                                <= 3
                        })
                    });
                let edges_ok = self.records.iter().all(|r| {
                    r["client_composite_error"].as_u64().unwrap() <= 12
                        && r["outside_halo_error"].as_u64().unwrap() <= 12
                });
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":composition_ok&&edges_ok,"native_edges_ok":edges_ok,"native_desktop_composition_ok":composition_ok,"composition_reset_simulated":self.composition_reset,"version":env!("CARGO_PKG_VERSION"),"screenshots":15,"stages":self.records,"transparent_avatar_activated":self.activated,"ptys":0,"model_requests":false,"system_input_injected":false,"synthetic_egui_input":true})).unwrap()).unwrap();
                assert!(
                    composition_ok && edges_ok,
                    "Native desktop does not show the white backdrop through the bar: {:?}",
                    self.records
                );
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if !self.pending
            && self.frames > 10
            && self.since.elapsed() > Duration::from_millis(600)
        {
            self.pending = true;
            for id in [egui::ViewportId::ROOT, floating::viewport_id()] {
                ui.ctx().send_viewport_cmd_to(
                    id,
                    egui::ViewportCommand::Screenshot(egui::UserData::default()),
                );
            }
        }
    }
}
#[cfg(windows)]
fn owned_bar() -> Option<[i64; 5]> {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::WindowsAndMessaging::*,
    };
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == std::process::id() {
            let mut title = [0; 80];
            let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), 80) };
            if String::from_utf16_lossy(&title[..len as usize]) == "ai-terminal status" {
                unsafe { *(data as *mut HWND) = hwnd };
                return 0;
            }
        }
        1
    }
    let mut hwnd: HWND = std::ptr::null_mut();
    unsafe { EnumWindows(Some(visit), &mut hwnd as *mut HWND as LPARAM) };
    if hwnd.is_null() {
        return None;
    }
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) };
    Some([
        hwnd as i64,
        i64::from(rect.left),
        i64::from(rect.top),
        i64::from(rect.right - rect.left),
        i64::from(rect.bottom - rect.top),
    ])
}
#[cfg(not(windows))]
fn owned_bar() -> Option<[i64; 5]> {
    None
}
#[cfg(windows)]
fn simulate_composition_reset(handle: i64) {
    use windows_sys::Win32::{Graphics::Dwm::*, UI::WindowsAndMessaging::*};
    let hwnd = handle as windows_sys::Win32::Foundation::HWND;
    let disabled = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE,
        fEnable: 0,
        ..Default::default()
    };
    unsafe {
        assert!(DwmEnableBlurBehindWindow(hwnd, &disabled) >= 0);
        assert_ne!(PostMessageW(hwnd, WM_DWMCOMPOSITIONCHANGED, 0, 0), 0);
    }
}
#[cfg(not(windows))]
fn simulate_composition_reset(_: i64) {
    panic!("Native composition reset requires Windows")
}

fn save_image(path: &std::path::Path, image: &egui::ColorImage) {
    let mut png = png::Encoder::new(
        std::fs::File::create(path).unwrap(),
        image.width() as u32,
        image.height() as u32,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(
            &image
                .pixels
                .iter()
                .flat_map(|p| p.to_array())
                .collect::<Vec<_>>(),
        )
        .unwrap();
}
#[cfg(windows)]
struct Backdrop(windows_sys::Win32::Foundation::HWND);
#[cfg(windows)]
impl Backdrop {
    fn new() -> Self {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let title: Vec<u16> = "ai-terminal opacity test backdrop\0"
            .encode_utf16()
            .collect();
        // An opaque white fixture, behind the later-created owned topmost bar.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE | 6, // SS_WHITERECT
                40,
                BACKDROP_Y,
                520,
                160,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null(), "Cannot create owned white backdrop");
        Self(hwnd)
    }
    fn gray(&self, gray: bool) {
        use windows_sys::Win32::{
            Graphics::Gdi::{InvalidateRect, UpdateWindow},
            UI::WindowsAndMessaging::*,
        };
        unsafe {
            let style = GetWindowLongPtrW(self.0, GWL_STYLE);
            SetWindowLongPtrW(self.0, GWL_STYLE, (style & !31) | if gray { 5 } else { 6 });
            InvalidateRect(self.0, std::ptr::null(), 1);
            UpdateWindow(self.0);
        }
    }
}
#[cfg(windows)]
impl Drop for Backdrop {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0) };
    }
}
#[cfg(not(windows))]
struct Backdrop;
#[cfg(not(windows))]
impl Backdrop {
    fn new() -> Self {
        panic!("Native opacity verification requires Windows")
    }
    fn gray(&self, _: bool) {}
}
#[cfg(windows)]
fn desktop_capture(native: [i64; 5], expected: u8) -> (egui::ColorImage, [u8; 3]) {
    use windows_sys::Win32::Graphics::{Dwm::DwmFlush, Gdi::*};
    let (w, h) = (
        native[3] as i32 + 2 * PAD as i32,
        native[4] as i32 + 2 * PAD as i32,
    );
    assert!(
        native[1] - PAD as i64 >= 40
            && native[2] - PAD as i64 >= i64::from(BACKDROP_Y)
            && native[1] + native[3] + PAD as i64 <= 560
            && native[2] + native[4] + PAD as i64 <= i64::from(BACKDROP_Y + 160),
        "Capture must stay inside our opaque backdrop"
    );
    unsafe {
        DwmFlush();
        let screen = GetDC(std::ptr::null_mut());
        assert!(!screen.is_null());
        let fixture = GetPixel(screen, 550, BACKDROP_Y + 150);
        let reference = [
            (fixture & 255) as u8,
            ((fixture >> 8) & 255) as u8,
            ((fixture >> 16) & 255) as u8,
        ];
        assert!(
            fixture != CLR_INVALID
                && if expected == 255 {
                    reference.iter().all(|c| c.abs_diff(255) <= 5)
                } else {
                    reference
                        .iter()
                        .all(|c| (80..=200).contains(c) && c.abs_diff(reference[0]) <= 5)
                },
            "Owned backdrop reference mismatch: {reference:?}"
        );
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        assert!(!memory.is_null() && !bitmap.is_null());
        let old = SelectObject(memory, bitmap);
        let copied = BitBlt(
            memory,
            0,
            0,
            w,
            h,
            screen,
            native[1] as i32 - PAD as i32,
            native[2] as i32 - PAD as i32,
            SRCCOPY | CAPTUREBLT,
        );
        SelectObject(memory, old);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0_u8; (w * h * 4) as usize];
        let rows = GetDIBits(
            memory,
            bitmap,
            0,
            h as u32,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        );
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        assert!(
            copied != 0 && rows == h,
            "Cannot capture the owned test region"
        );
        for p in pixels.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
            p[3] = 255;
        }
        (
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels),
            reference,
        )
    }
}
#[cfg(not(windows))]
fn desktop_capture(_: [i64; 5], _: u8) -> (egui::ColorImage, [u8; 3]) {
    panic!("Native opacity verification requires Windows")
}

fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("Fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(!output.join("result.json").exists());
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal floating transparency",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([900.0, 720.0])
                .with_active(false)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            ai_terminal::window_frame::apply_dark(cc);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                _backdrop: Backdrop::new(),
                bar: FloatingBar::new(Icons::new(&cc.egui_ctx)),
                prefs: UiSettings {
                    floating_border: false,
                    floating_enabled: true,
                    floating_scale: 1.36,
                    ..Default::default()
                },
                open: true,
                output,
                stage: 0,
                frames: 0,
                since: Instant::now(),
                began: Instant::now(),
                pending: false,
                shots: [false; 2],
                clicked: 0,
                activated: false,
                composition_reset: false,
                records: Vec::new(),
            }))
        }),
    )?;
    assert!(
        verify.join("result.json").is_file(),
        "Probe closed before completing native captures"
    );
    Ok(())
}
