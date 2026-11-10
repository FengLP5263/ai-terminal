//! An owned non-activating viewport, sharing production text shaping and color emoji.
use crate::{
    floating::{HoverHint, hint_viewport_id},
    ui_text,
};
use egui::{Color32, Pos2, Vec2};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::*,
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

fn window(ctx: &egui::Context, title: &str, key: &str) -> Option<HWND> {
    fn owned(hwnd: HWND) -> bool {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        pid == std::process::id()
    }
    let id = egui::Id::new(key);
    if let Some(hwnd) = ctx
        .data_mut(|d| d.get_temp::<usize>(id))
        .map(|p| p as HWND)
        .filter(|&h| {
            if !owned(h) {
                return false;
            }
            let mut buf = [0u16; 128];
            let n = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
            String::from_utf16_lossy(&buf[..n as usize]) == title
        })
    {
        return Some(hwnd);
    }
    struct Find<'a> {
        title: &'a str,
        result: Option<HWND>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: isize) -> i32 {
        if !owned(hwnd) {
            return 1;
        }
        let find = unsafe { &mut *(data as *mut Find<'_>) };
        let mut buf = [0u16; 128];
        let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if String::from_utf16_lossy(&buf[..n as usize]) == find.title {
            find.result = Some(hwnd);
            return 0;
        }
        1
    }
    let mut find = Find {
        title,
        result: None,
    };
    unsafe {
        EnumWindows(Some(visit), &mut find as *mut _ as isize);
    }
    if let Some(hwnd) = find.result {
        ctx.data_mut(|d| d.insert_temp(id, hwnd as usize));
    }
    find.result
}
fn anchor(owner: HWND, zoom: f32) -> Option<([i32; 4], [i32; 4], f32)> {
    let mut bar = RECT::default();
    let mut work = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetWindowRect(owner, &mut bar) == 0
            || GetMonitorInfoW(
                MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
                &mut work,
            ) == 0
        {
            return None;
        }
        let scale = GetDpiForWindow(owner) as f32 / 96.0 * zoom;
        Some((
            [bar.left, bar.top, bar.right, bar.bottom],
            [
                work.rcWork.left,
                work.rcWork.top,
                work.rcWork.right,
                work.rcWork.bottom,
            ],
            scale.max(0.1),
        ))
    }
}
pub fn show(ctx: &egui::Context, hint: Option<HoverHint>, zoom: f32) {
    let Some(hint) = hint else {
        return;
    };
    let Some(owner) = window(ctx, "ai-terminal status", "floating-native-hwnd") else {
        return;
    };
    let Some((bar, work, scale)) = anchor(owner, zoom) else {
        return;
    };
    let ratio = zoom / ctx.zoom_factor();
    let text: String = hint
        .text
        .chars()
        .take(2048)
        .map(|c| if c == '\0' { ' ' } else { c })
        .collect();
    let max_width = 420.0f32
        .min((work[2] - work[0]) as f32 / scale - 16.0)
        .max(32.0);
    let rows = (((work[3] - work[1]) as f32 / scale - 12.0) / 16.0).clamp(1.0, 64.0) as usize;
    let galley = ui_text::layout_rows(
        ctx,
        &text,
        12.0 * ratio,
        Color32::from_rgb(238, 238, 238),
        max_width * ratio,
        rows,
    );
    let initial = galley.size() + Vec2::new(16.0, 12.0) * ratio;
    let physical = origin_for(
        bar,
        bar[0] + (hint.anchor.x * scale).round() as i32,
        [
            (initial.x * scale / ratio).ceil() as i32,
            (initial.y * scale / ratio).ceil() as i32,
        ],
        work,
        (6.0 * scale).round() as i32,
    );
    let exists = ctx.input(|i| i.raw.viewports.contains_key(&hint_viewport_id()));
    let mut builder = egui::ViewportBuilder::default()
        .with_title("ai-terminal hint")
        .with_icon(crate::appearance::window_icon())
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_inner_size(initial)
        .with_taskbar(false)
        .with_active(false)
        .with_mouse_passthrough(true)
        .with_always_on_top();
    if !exists {
        builder = builder.with_inner_size(initial).with_position([
            physical[0] as f32 / (scale / ratio),
            physical[1] as f32 / (scale / ratio),
        ]);
    }
    let owner = owner as usize;
    ctx.show_viewport_deferred(hint_viewport_id(), builder, move |ui, _| {
        let owner = owner as HWND;
        ui.scope(|ui| {
            let Some((bar, work, scale)) = anchor(owner, zoom) else {
                return;
            };
            let ratio = zoom / ui.ctx().zoom_factor();
            let width = 420.0f32
                .min((work[2] - work[0]) as f32 / scale - 16.0)
                .max(32.0);
            let galley = ui_text::layout_rows(
                ui.ctx(),
                &text,
                12.0 * ratio,
                Color32::from_rgb(238, 238, 238),
                width * ratio,
                rows,
            );
            let size = galley.size() + Vec2::new(16.0, 12.0) * ratio;
            let pos = Pos2::new(8.0 * ratio, 6.0 * ratio);
            // A non-interactive native hint owns its full surface. Do not
            // inherit a previous pass's panel clip while global zoom changes.
            let rect = egui::Rect::from_min_size(Pos2::ZERO, size);
            let painter = ui.painter().with_clip_rect(rect);
            painter.rect_filled(rect, 0, Color32::from_rgb(35, 35, 35));
            painter.galley(pos, galley.clone(), Color32::from_rgb(238, 238, 238));
            ui_text::paint_emoji(
                &painter,
                pos,
                &galley,
                12.0 * ratio,
                Color32::from_rgb(238, 238, 238),
            );
            if let Some(hwnd) = window(ui.ctx(), "ai-terminal hint", "floating-hint-hwnd") {
                let dimensions = [
                    (size.x * scale / ratio).ceil() as i32,
                    (size.y * scale / ratio).ceil() as i32,
                ];
                let place = origin_for(
                    bar,
                    bar[0] + (hint.anchor.x * scale).round() as i32,
                    dimensions,
                    work,
                    (6.0 * scale).round() as i32,
                );
                unsafe {
                    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    let required =
                        (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT) as isize;
                    if style & required != required {
                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | required);
                    }
                    if GetWindow(hwnd, GW_OWNER) != owner {
                        SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner as isize);
                    }
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect) != 0
                        && [
                            rect.left,
                            rect.top,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                        ] != [place[0], place[1], dimensions[0], dimensions[1]]
                    {
                        SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            place[0],
                            place[1],
                            dimensions[0],
                            dimensions[1],
                            SWP_NOACTIVATE,
                        );
                        ui.ctx().request_repaint();
                    }
                }
            }
        });
    });
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
