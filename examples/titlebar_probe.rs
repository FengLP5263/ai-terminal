//! Capture only this probe's native frame before/after theme notifications.
use ai_terminal::{appearance, window_frame};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    UI::{HiDpi::*, WindowsAndMessaging::*},
};
#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
}
struct Probe {
    renderer: egui_term::TextRendererSession,
    output: PathBuf,
    began: Instant,
    phase: usize,
    applied: bool,
    checks: Vec<serde_json::Value>,
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
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        ui.heading("Native dark title bar");
        ui.label("Synthetic UI · no Agent/model request · system personalization is unchanged.");
        if (self.phase == 0 && self.began.elapsed() > Duration::from_secs(1))
            || (self.phase == 1 && self.began.elapsed() > Duration::from_secs(2))
        {
            let handle = frame.window_handle().unwrap();
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                panic!("Windows frame required")
            };
            let hwnd = handle.hwnd.get() as HWND;
            let check = capture(hwnd, &self.output.join(format!("frame-{}.png", self.phase)));
            self.checks.push(check);
            if self.phase == 0 {
                unsafe {
                    SendMessageW(hwnd, WM_THEMECHANGED, 0, 0);
                    let setting: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
                    SendMessageW(hwnd, WM_SETTINGCHANGE, 0, setting.as_ptr() as isize);
                }
            }
            self.phase += 1;
            if self.phase == 2 {
                let ok = self.applied
                    && self.checks.iter().all(|v| {
                        v["dark_requested"] == true
                            && v["caption_is_dark"] == true
                            && v["caption_buttons_drawn"] == true
                    });
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":ok,"policy_installed":self.applied,"checks":self.checks,"system_settings_changed":false})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(80));
    }
}
fn capture(hwnd: HWND, path: &std::path::Path) -> serde_json::Value {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        assert_eq!(pid, std::process::id());
        let mut rect: RECT = std::mem::zeroed();
        assert_ne!(GetWindowRect(hwnd, &mut rect), 0);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        assert!(width > 0 && height > 0 && width < 8000 && height < 8000);
        let source = GetWindowDC(hwnd);
        assert!(!source.is_null());
        let memory = CreateCompatibleDC(source);
        assert!(!memory.is_null());
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
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
        let previous = SelectObject(memory, bitmap);
        let printed = PrintWindow(hwnd, memory, 2);
        let mut pixels =
            std::slice::from_raw_parts(bits.cast::<u8>(), width as usize * height as usize * 4)
                .to_vec();
        SelectObject(memory, previous);
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(hwnd, source);
        assert_ne!(printed, 0);
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        let dpi = GetDpiForWindow(hwnd);
        let y = ((22 * dpi / 96) as usize).min(height as usize - 1);
        let x = width as usize * 2 / 3;
        let rgb = &pixels[(y * width as usize + x) * 4..][..3];
        let from_x = (width as usize).saturating_sub((160 * dpi / 96) as usize);
        let to_y = ((32 * dpi / 96) as usize).min(height as usize);
        let button_pixels = (0..to_y)
            .flat_map(|row| (from_x..width as usize).map(move |column| (row, column)))
            .filter(|&(row, column)| {
                pixels[(row * width as usize + column) * 4..][..3]
                    .iter()
                    .all(|&c| c > 96)
            })
            .count();
        let mut dark = 0_i32;
        let result = DwmGetWindowAttribute(hwnd, 20, (&mut dark as *mut i32).cast(), 4);
        let file = std::fs::File::create(path).unwrap();
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        serde_json::json!({"dark_requested":result>=0&&dark!=0,"caption_rgb":rgb,"caption_is_dark":rgb.iter().all(|&c|c<128),"caption_buttons_drawn":button_pixels>12,"dpi":dpi,"foreground":GetForegroundWindow()==hwnd})
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("titlebar_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "ai-terminal native frame acceptance",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_transparent(true)
                .with_inner_size([700.0, 340.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
            let applied = window_frame::apply_dark(cc);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                output,
                began: Instant::now(),
                phase: 0,
                applied,
                checks: vec![],
            }))
        }),
    )
}
