//! Keep a moved window reachable when displays have different work areas and DPI.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Rect {
    fn width(self) -> i32 {
        self.right - self.left
    }
    fn height(self) -> i32 {
        self.bottom - self.top
    }
}

/// Fit the outer rectangle into the available work area without enlarging it.
fn fit_window(window: Rect, work: Rect) -> Option<Rect> {
    if window.width() <= 0 || window.height() <= 0 || work.width() <= 0 || work.height() <= 0 {
        return None;
    }
    let width = window.width().min(work.width());
    let height = window.height().min(work.height());
    let left = window.left.clamp(work.left, work.right - width);
    let top = window.top.clamp(work.top, work.bottom - height);
    let fitted = Rect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    (fitted != window).then_some(fitted)
}

#[derive(Default)]
pub struct MonitorTransition {
    settled: Option<isize>,
    last_position: Option<(i32, i32)>,
    drag_moved: bool,
    pending: Option<(isize, std::time::Instant, bool)>,
}

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    None,
    AwaitRelease,
    Changed(isize),
}

impl MonitorTransition {
    fn observe(
        &mut self,
        position: (i32, i32),
        area_monitor: isize,
        pressed: bool,
        pointer_monitor: Option<isize>,
    ) -> Decision {
        let moved = self
            .last_position
            .replace(position)
            .is_some_and(|old| old != position);
        if self.settled.is_none() {
            self.settled = Some(area_monitor);
            return Decision::None;
        }
        if pressed {
            self.drag_moved |= moved;
            return if self.drag_moved {
                Decision::AwaitRelease
            } else {
                Decision::None
            };
        }
        let target = if self.drag_moved {
            pointer_monitor.unwrap_or(area_monitor)
        } else {
            area_monitor
        };
        self.drag_moved = false;
        if self.settled == Some(target) {
            return Decision::None;
        }
        self.settled = Some(target);
        Decision::Changed(target)
    }

    /// Called for root-window frames only. No timers are active when stationary.
    #[cfg(windows)]
    pub fn update(&mut self, ctx: &egui::Context) {
        use windows_sys::Win32::{
            Foundation::{POINT, RECT},
            Graphics::Gdi::{
                GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
                MonitorFromRect,
            },
            UI::{
                Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
                WindowsAndMessaging::GetCursorPos,
            },
        };
        let Some((outer, inner, maximized)) = ctx.input(|input| {
            let v = input.viewport();
            if v.minimized.unwrap_or(false) {
                return None;
            }
            Some((v.outer_rect?, v.inner_rect?, v.maximized.unwrap_or(false)))
        }) else {
            return;
        };
        let ppp = ctx.pixels_per_point();
        if !ppp.is_finite() || ppp <= 0.0 {
            return;
        }
        let physical = |v: f32| (v * ppp).round() as i32;
        let bounds = Rect {
            left: physical(outer.left()),
            top: physical(outer.top()),
            right: physical(outer.right()),
            bottom: physical(outer.bottom()),
        };
        let native = RECT {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        };
        // Track native title-bar movement, which is not delivered as an egui drag.
        let area = unsafe { MonitorFromRect(&native, MONITOR_DEFAULTTONEAREST) };
        if area.is_null() {
            return;
        }
        let pressed = unsafe { GetAsyncKeyState(VK_LBUTTON.into()) } < 0;
        let pointer_monitor = if self.drag_moved && !pressed {
            let mut cursor = POINT::default();
            (unsafe { GetCursorPos(&mut cursor) } != 0)
                .then(|| unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) } as isize)
                .filter(|monitor| *monitor != 0)
        } else {
            None
        };
        // A DPI change can deliver the native window geometry one frame before
        // egui has switched its scale. Let the scale and bounds settle before
        // translating physical work-area coordinates into viewport commands.
        let (monitor, repositioned) = if let Some((target, since, repositioned)) = self.pending {
            if pressed {
                self.pending = None;
                self.observe((bounds.left, bounds.top), area as isize, true, None);
                ctx.request_repaint_after(std::time::Duration::from_millis(80));
                return;
            }
            let delay = std::time::Duration::from_millis(150);
            if since.elapsed() < delay {
                ctx.request_repaint_after(delay - since.elapsed());
                return;
            }
            self.pending = None;
            (target as _, repositioned)
        } else {
            match self.observe(
                (bounds.left, bounds.top),
                area as isize,
                pressed,
                pointer_monitor,
            ) {
                Decision::None => return,
                Decision::AwaitRelease => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(80));
                    return;
                }
                Decision::Changed(target) => {
                    self.pending = Some((target, std::time::Instant::now(), false));
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                    return;
                }
            }
        };
        if maximized {
            return;
        } // Windows owns the bounds of a maximized window.
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return;
        }
        let work = Rect {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        };
        if let Some(fitted) = fit_window(bounds, work) {
            if area != monitor && !repositioned {
                // First let Windows switch DPI for the destination monitor. If we
                // resize while still using the old DPI, Windows scales twice.
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    fitted.left as f32 / ppp,
                    fitted.top as f32 / ppp,
                )));
                self.pending = Some((monitor as isize, std::time::Instant::now(), true));
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
                return;
            }
            let border_width = (physical(outer.width()) - physical(inner.width())).max(0);
            let border_height = (physical(outer.height()) - physical(inner.height())).max(0);
            if fitted.width() != bounds.width() || fitted.height() != bounds.height() {
                let width = (fitted.width() - border_width).max(1) as f32 / ppp;
                let height = (fitted.height() - border_height).max(1) as f32 / ppp;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                fitted.left as f32 / ppp,
                fitted.top as f32 / ppp,
            )));
        }
    }

    #[cfg(not(windows))]
    pub fn update(&mut self, _: &egui::Context) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(left: i32, top: i32, width: i32, height: i32) -> Rect {
        Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }
    #[test]
    fn title_drag_waits_for_release_and_uses_pointer_monitor_even_when_most_of_window_is_elsewhere()
    {
        let mut state = MonitorTransition::default();
        assert_eq!(state.observe((3000, 20), 2, false, None), Decision::None);
        assert_eq!(
            state.observe((1800, 20), 2, true, None),
            Decision::AwaitRelease
        );
        assert_eq!(
            state.observe((300, 20), 2, true, None),
            Decision::AwaitRelease
        );
        assert_eq!(
            state.observe((300, 20), 2, false, Some(1)),
            Decision::Changed(1)
        );
    }
    #[test]
    fn a_local_drag_does_not_force_the_window_into_the_work_area() {
        let mut state = MonitorTransition::default();
        assert_eq!(state.observe((30, 20), 1, false, None), Decision::None);
        assert_eq!(
            state.observe((-100, 20), 1, true, None),
            Decision::AwaitRelease
        );
        assert_eq!(state.observe((-100, 20), 1, false, Some(1)), Decision::None);
    }
    #[test]
    fn oversized_window_is_fitted_to_smaller_work_area() {
        let work = rect(0, 0, 2560, 1368);
        assert_eq!(fit_window(rect(280, 40, 3000, 1700), work), Some(work));
    }
    #[test]
    fn fitting_preserves_window_size_and_supports_negative_monitor_coordinates() {
        let work = rect(-1920, -200, 1920, 1040);
        assert_eq!(
            fit_window(rect(-1300, -190, 1400, 800), work),
            Some(rect(-1400, -190, 1400, 800))
        );
        assert_eq!(fit_window(rect(-1800, -100, 1200, 700), work), None);
    }
    #[test]
    fn invalid_work_areas_are_ignored() {
        assert_eq!(fit_window(rect(0, 0, 100, 100), rect(0, 0, 0, 0)), None);
    }
}
