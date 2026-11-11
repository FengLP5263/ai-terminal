//! Event-driven geometry for our own undecorated floating viewport.
//! Keep physical position observations out of egui's position-command feedback loop.

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::{
            Dwm::{DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND, DwmEnableBlurBehindWindow},
            Gdi::{
                CreateRectRgn, DeleteObject, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST,
                MONITORINFO, MonitorFromWindow,
            },
        },
        System::Threading::GetCurrentThreadId,
        UI::{HiDpi::GetDpiForWindow, Shell::*, WindowsAndMessaging::*},
    };

    const SUBCLASS: usize = 0xA174;
    const APPLY: u32 = WM_APP + 0x174;
    static PROPERTY: std::sync::LazyLock<Vec<u16>> =
        std::sync::LazyLock::new(|| "ai-terminal.floating.geometry\0".encode_utf16().collect());
    struct Geometry {
        ctx: egui::Context,
        points: [f32; 2],
        zoom: f32,
        position: Option<[f32; 2]>,
        moving: bool,
        posted: bool,
    }

    /// Called on the viewport's UI thread, never from a worker.
    pub fn sync(
        ctx: &egui::Context,
        points: [f32; 2],
        zoom: f32,
        position: Option<[f32; 2]>,
    ) -> Option<[f32; 2]> {
        let key = egui::Id::new("floating-native-hwnd");
        let cached = ctx.data_mut(|d| d.get_temp::<usize>(key));
        let hwnd = cached
            .map(|h| h as HWND)
            .filter(|&h| state(h).is_some())
            .or_else(find_window)?;
        ctx.data_mut(|d| d.insert_temp(key, hwnd as usize));
        let data = if let Some(data) = state(hwnd) {
            data
        } else {
            let data = Box::into_raw(Box::new(Geometry {
                ctx: ctx.clone(),
                points,
                zoom,
                position,
                moving: false,
                posted: false,
            }));
            if unsafe { SetPropW(hwnd, PROPERTY.as_ptr(), data.cast()) } == 0 {
                unsafe { drop(Box::from_raw(data)) };
                return None;
            }
            if unsafe { SetWindowSubclass(hwnd, Some(window_event), SUBCLASS, data as usize) } == 0
            {
                unsafe {
                    RemovePropW(hwnd, PROPERTY.as_ptr());
                    drop(Box::from_raw(data));
                }
                return None;
            }
            enable_alpha(hwnd);
            post(hwnd, data);
            data
        };
        // No reference to this allocation is held across a reentrant Windows call.
        let changed = unsafe { (*data).points != points || (*data).zoom != zoom };
        unsafe {
            (*data).points = points;
            (*data).zoom = zoom;
            if position.is_some() {
                (*data).position = position;
            }
        }
        if changed || position.is_some() {
            post(hwnd, data);
        }
        if unsafe { (*data).moving || (*data).posted || (*data).position.is_some() } {
            return None;
        }
        let mut rect = RECT::default();
        (unsafe { GetWindowRect(hwnd, &mut rect) } != 0)
            .then_some([rect.left as f32, rect.top as f32])
    }

    // A WGL configuration may be conservatively marked opaque by glutin-winit.
    // Its framebuffer still has alpha, but DWM must opt in after the actual HWND exists.
    fn enable_alpha(hwnd: HWND) {
        use windows_sys::Win32::Graphics::Dwm::{
            DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_POLICY,
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
        };
        let policy = DWMNCRP_DISABLED;
        let corners = DWMWCP_DONOTROUND;
        let border = 0xffff_fffe_u32; // DWMWA_COLOR_NONE (Windows 11).
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY as u32,
                (&policy as *const i32).cast(),
                4,
            );
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                (&corners as *const i32).cast(),
                4,
            );
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR as u32,
                (&border as *const u32).cast(),
                4,
            );
        }
        let region = unsafe { CreateRectRgn(0, 0, -1, -1) };
        if region.is_null() {
            return;
        }
        let blur = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            fEnable: 1,
            hRgnBlur: region,
            fTransitionOnMaximized: 0,
        };
        unsafe {
            DwmEnableBlurBehindWindow(hwnd, &blur);
            DeleteObject(region);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    fn state(hwnd: HWND) -> Option<*mut Geometry> {
        let data = unsafe { GetPropW(hwnd, PROPERTY.as_ptr()) };
        (!data.is_null()).then_some(data.cast())
    }

    fn find_window() -> Option<HWND> {
        unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            if pid == std::process::id() {
                let mut title = [0; 80];
                let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
                if String::from_utf16_lossy(&title[..len as usize]) == "ai-terminal status" {
                    unsafe { *(data as *mut Option<HWND>) = Some(hwnd) };
                    return 0;
                }
            }
            1
        }
        let mut result = None;
        unsafe {
            EnumThreadWindows(
                GetCurrentThreadId(),
                Some(visit),
                &mut result as *mut _ as LPARAM,
            );
        }
        result
    }

    fn post(hwnd: HWND, data: *mut Geometry) {
        if !unsafe { (*data).posted } {
            unsafe {
                (*data).posted = PostMessageW(hwnd, APPLY, 0, 0) != 0;
            }
        }
    }

    fn apply(hwnd: HWND, data: *mut Geometry) {
        // Windows owns both size and the mouse anchor during the native move
        // loop. Repairing size here can trigger another monitor/DPI transition.
        if unsafe { (*data).moving } {
            unsafe { (*data).posted = false };
            return;
        }
        let (ctx, points, zoom, position) = unsafe {
            (*data).posted = false;
            let position = (*data).position.take();
            ((*data).ctx.clone(), (*data).points, (*data).zoom, position)
        };
        let scale = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32 / 96.0 * zoom;
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let available = if unsafe {
            GetMonitorInfoW(
                MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
        } != 0
        {
            (monitor.rcWork.right - monitor.rcWork.left) as f32 - 16.0 * scale
        } else {
            f32::MAX
        };
        let width = (points[0] * scale).min(available).round().max(1.0) as i32;
        let height = (points[1] * scale).round().max(1.0) as i32;
        let mut client = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut client) } == 0 {
            return;
        }
        if position.is_some() || client.right != width || client.bottom != height {
            let pos = position.unwrap_or_default();
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    pos[0].round() as i32,
                    pos[1].round() as i32,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE | if position.is_none() { SWP_NOMOVE } else { 0 },
                );
            }
        }
        // A move can finish without another WM_SIZE. Wake the child once so
        // the final physical position is observed and saved even when idle.
        ctx.request_repaint_of(crate::floating::viewport_id());
    }

    unsafe extern "system" fn window_event(
        hwnd: HWND,
        message: u32,
        wparam: usize,
        lparam: isize,
        id: usize,
        data: usize,
    ) -> isize {
        let data = data as *mut Geometry;
        if message == WM_NCDESTROY {
            unsafe {
                RemoveWindowSubclass(hwnd, Some(window_event), id);
                RemovePropW(hwnd, PROPERTY.as_ptr());
                drop(Box::from_raw(data));
                return DefSubclassProc(hwnd, message, wparam, lparam);
            }
        }
        // This capsule has no non-client frame. Bypass winit's shadow-only 1px inset.
        if message == WM_NCCALCSIZE {
            return 0;
        }
        if message == APPLY {
            apply(hwnd, data);
            return 0;
        }
        if message == WM_ENTERSIZEMOVE {
            unsafe { (*data).moving = true };
        } else if message == WM_EXITSIZEMOVE {
            unsafe { (*data).moving = false };
        }
        // Winit must adopt the new DPI before we reconcile the actual client size.
        let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
        if message == WM_DWMCOMPOSITIONCHANGED {
            enable_alpha(hwnd);
            unsafe {
                (*data)
                    .ctx
                    .request_repaint_of(crate::floating::viewport_id())
            };
        }
        if matches!(message, WM_DPICHANGED | WM_SIZE | WM_EXITSIZEMOVE) {
            post(hwnd, data);
        }
        result
    }
}

#[cfg(windows)]
pub use windows::sync;
