//! Per-window native frame policy; never changes Windows personalization.
use raw_window_handle::HasWindowHandle;

/// Request dark non-client rendering explicitly, and reassert it after native
/// theme notifications have passed through the window backend.
/// Unsupported frames keep the window backend's theme instead.
pub fn apply_dark(window: &impl HasWindowHandle) -> bool {
    #[cfg(windows)]
    {
        use raw_window_handle::RawWindowHandle;
        let Ok(handle) = window.window_handle() else {
            return false;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return false;
        };
        let hwnd = handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
        set_dark_attribute(hwnd)
            && unsafe {
                windows_sys::Win32::UI::Shell::SetWindowSubclass(hwnd, Some(theme_changed), 1, 0)
                    != 0
            }
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        false
    }
}

#[cfg(windows)]
fn set_dark_attribute(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::{Graphics::Dwm::DwmSetWindowAttribute, UI::WindowsAndMessaging::*};
    let enabled = 1_i32;
    // 20 is documented; 19 is the older Windows 10 attribute. No global setting changes.
    for attribute in [20, 19] {
        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                (&enabled as *const i32).cast(),
                std::mem::size_of_val(&enabled) as u32,
            )
        };
        if result >= 0 {
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
            return true;
        }
    }
    false
}

#[cfg(windows)]
unsafe extern "system" fn theme_changed(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    id: usize,
    _: usize,
) -> isize {
    use windows_sys::Win32::UI::{Shell::*, WindowsAndMessaging::*};
    // The OS owns the callback lifetime; no App pointer or allocation is retained.
    if message == WM_NCDESTROY {
        unsafe {
            RemoveWindowSubclass(hwnd, Some(theme_changed), id);
        }
    }
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if matches!(message, WM_THEMECHANGED | WM_SETTINGCHANGE) {
        set_dark_attribute(hwnd);
    }
    result
}
