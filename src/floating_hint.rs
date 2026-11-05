//! An owned, non-activating native tooltip can extend beyond the capsule HWND.
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::*,
    UI::{Controls::*, WindowsAndMessaging::*},
};

pub(crate) struct Popup {
    window: HWND,
    owner: HWND,
    text: Vec<u16>,
    font: HFONT,
    scale: f32,
    last: Option<(String, [i32; 4], i32, f32)>,
}
impl Popup {
    pub fn new(owner: HWND) -> Option<Self> {
        let init = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES,
        };
        // SAFETY: all calls and resources are confined to the owning viewport UI thread.
        unsafe {
            if InitCommonControlsEx(&init) == 0 {
                return None;
            }
            let window = CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_LAYERED
                    | WS_EX_TRANSPARENT,
                TOOLTIPS_CLASSW,
                std::ptr::null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX | TTS_NOANIMATE | TTS_NOFADE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                owner,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            if window.is_null() {
                return None;
            }
            let mut popup = Self {
                window,
                owner,
                text: vec![0],
                font: std::ptr::null_mut(),
                scale: 0.0,
                last: None,
            };
            SetLayeredWindowAttributes(window, 0, 255, LWA_ALPHA);
            // Classic tooltip colors are predictable across system light/dark themes.
            let empty = [0u16];
            SetWindowTheme(window, empty.as_ptr(), empty.as_ptr());
            SendMessageW(window, TTM_SETTIPBKCOLOR, 0x232323, 0);
            SendMessageW(window, TTM_SETTIPTEXTCOLOR, 0xeeeeee, 0);
            let margin = RECT {
                left: 7,
                top: 5,
                right: 7,
                bottom: 5,
            };
            SendMessageW(window, TTM_SETMARGIN, 0, &margin as *const _ as isize);
            let info = popup.info();
            if SendMessageW(window, TTM_ADDTOOLW, 0, &info as *const _ as isize) == 0 {
                return None;
            }
            Some(popup)
        }
    }
    fn info(&mut self) -> TTTOOLINFOW {
        TTTOOLINFOW {
            // V2 excludes lpReserved and works with both classic and themed controls.
            cbSize: std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32,
            uFlags: TTF_TRACK | TTF_ABSOLUTE,
            hwnd: self.owner,
            uId: 1,
            lpszText: self.text.as_mut_ptr(),
            ..Default::default()
        }
    }
    pub fn hide(&mut self) {
        if self.last.take().is_some() {
            let info = self.info();
            unsafe {
                SendMessageW(
                    self.window,
                    TTM_TRACKACTIVATE,
                    0,
                    &info as *const _ as isize,
                );
            }
        }
    }
    pub fn show(&mut self, hint: &crate::floating::HoverHint, scale: f32) {
        let mut bar = RECT::default();
        let mut origin = POINT::default();
        let mut work = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        unsafe {
            if GetClientRect(self.owner, &mut bar) == 0
                || ClientToScreen(self.owner, &mut origin) == 0
                || GetMonitorInfoW(
                    MonitorFromWindow(self.owner, MONITOR_DEFAULTTONEAREST),
                    &mut work,
                ) == 0
            {
                self.hide();
                return;
            }
        }
        let bar = [
            origin.x,
            origin.y,
            origin.x + bar.right,
            origin.y + bar.bottom,
        ];
        let anchor = origin.x + (hint.anchor.x * scale).round() as i32;
        let signature = (hint.text.clone(), bar, anchor, scale);
        if self.last.as_ref() == Some(&signature) {
            return;
        }
        unsafe {
            if self.scale != scale {
                let face: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
                let font = CreateFontW(
                    -(12.0 * scale).round() as i32,
                    0,
                    0,
                    0,
                    FW_NORMAL as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    DEFAULT_PITCH as u32,
                    face.as_ptr(),
                );
                if !font.is_null() {
                    SendMessageW(self.window, WM_SETFONT, font as usize, 0);
                    if !self.font.is_null() {
                        DeleteObject(self.font);
                    }
                    self.font = font;
                }
                self.scale = scale;
            }
            let text: Vec<u16> = hint
                .text
                .chars()
                .take(2048)
                .map(|c| if c == '\0' { ' ' } else { c })
                .chain(std::iter::once('\0'))
                .collect::<String>()
                .encode_utf16()
                .collect();
            let old = std::mem::replace(&mut self.text, text);
            let info = self.info();
            let max_width = (420.0 * scale).round() as i32;
            SendMessageW(
                self.window,
                TTM_SETMAXTIPWIDTH,
                0,
                max_width
                    .min(work.rcWork.right - work.rcWork.left - 16)
                    .max(1) as isize,
            );
            SendMessageW(
                self.window,
                TTM_UPDATETIPTEXTW,
                0,
                &info as *const _ as isize,
            );
            drop(old);
            // Activate only the owned tooltip; neither the capsule nor main window receives focus.
            SendMessageW(
                self.window,
                TTM_TRACKACTIVATE,
                1,
                &info as *const _ as isize,
            );
            let mut bounds = RECT::default();
            if GetWindowRect(self.window, &mut bounds) != 0 {
                let pos = origin_for(
                    bar,
                    anchor,
                    [bounds.right - bounds.left, bounds.bottom - bounds.top],
                    [
                        work.rcWork.left,
                        work.rcWork.top,
                        work.rcWork.right,
                        work.rcWork.bottom,
                    ],
                    (6.0 * scale).round() as i32,
                );
                let packed = (pos[0] as u16 as u32) | ((pos[1] as u16 as u32) << 16);
                SendMessageW(self.window, TTM_TRACKPOSITION, 0, packed as isize);
                SetWindowPos(
                    self.window,
                    HWND_TOPMOST,
                    pos[0],
                    pos[1],
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        self.last = Some(signature);
    }
}
impl Drop for Popup {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.window);
            if !self.font.is_null() {
                DeleteObject(self.font);
            }
        }
    }
}

fn origin_for(bar: [i32; 4], anchor: i32, size: [i32; 2], work: [i32; 4], gap: i32) -> [i32; 2] {
    let x = (anchor - size[0] / 2).clamp(work[0], (work[2] - size[0]).max(work[0]));
    let below = bar[3] + gap;
    let y = if below + size[1] <= work[3] {
        below
    } else {
        bar[1] - gap - size[1]
    };
    [x, y.clamp(work[1], (work[3] - size[1]).max(work[1]))]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tooltip_is_not_constrained_to_bar_and_stays_on_negative_coordinate_monitor() {
        let bar = [-1100, 0, -952, 72];
        let work = [-1920, 0, 0, 1040];
        assert_eq!(origin_for(bar, -1060, [480, 70], work, 8), [-1300, 80]);
        assert_eq!(
            origin_for([-148, 970, 0, 1040], -20, [480, 70], work, 8),
            [-480, 892]
        );
        assert_eq!(
            origin_for([-1920, 0, -1772, 72], -1900, [480, 70], work, 8),
            [-1920, 80]
        );
    }
}
