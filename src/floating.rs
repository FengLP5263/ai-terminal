//! A deferred native viewport: only session metadata crosses the window boundary.
use crate::{
    appearance, chrome,
    i18n::{self, Locale},
    model::{Agent, SessionColor},
    sidebar::{self, Icons},
    status::Activity,
};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender},
};
pub fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("session-status-bar")
}
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub agent: Agent,
    pub color: Option<SessionColor>,
    pub activity: Activity,
    pub unread: bool,
}
#[derive(Clone, Default, PartialEq)]
pub struct Snapshot {
    pub items: Vec<Item>,
    pub active: Option<String>,
    pub locale: Locale,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Activate(String),
    Disable,
    Position([f32; 2]),
}
pub struct FloatingBar {
    snapshot: Arc<Mutex<Snapshot>>,
    sender: SyncSender<Action>,
    receiver: Receiver<Action>,
    icons: Icons,
    placement_request: Arc<Mutex<Option<[f32; 2]>>>,
}
impl FloatingBar {
    pub fn new(icons: Icons) -> Self {
        let (sender, receiver) = mpsc::sync_channel(32);
        Self {
            snapshot: Arc::new(Mutex::new(Snapshot::default())),
            sender,
            receiver,
            icons,
            placement_request: Arc::new(Mutex::new(None)),
        }
    }
    pub fn update(&self, snapshot: Snapshot, ctx: &egui::Context) {
        let mut current = self.snapshot.lock().unwrap();
        if *current != snapshot {
            *current = snapshot;
            ctx.request_repaint_of(viewport_id());
        }
    }
    pub fn actions(&self) -> Vec<Action> {
        self.receiver.try_iter().take(32).collect()
    }
    pub fn show(&self, ctx: &egui::Context, position: Option<[f32; 2]>) {
        let snapshot = self.snapshot.clone();
        let sender = self.sender.clone();
        let icons = self.icons.clone();
        let count = snapshot.lock().unwrap().items.len();
        let placement = placement(ctx, count, position, false);
        let width = placement.width;
        let zoom = ctx.zoom_factor();
        let layout = (count, zoom);
        let previous = ctx.data_mut(|data| {
            let key = egui::Id::new("floating-window-layout");
            let previous = data.get_temp::<(usize, f32)>(key);
            data.insert_temp(key, layout);
            previous
        });
        let exists = ctx.input(|i| i.raw.viewports.contains_key(&viewport_id()));
        // Saved position is an observation, not a command. Only creation and an
        // intentional layout change can position the native window.
        if !exists || previous != Some(layout) {
            *self.placement_request.lock().unwrap() = Some(placement.pixels);
            ctx.request_repaint_of(viewport_id());
        }
        let placement_request = self.placement_request.clone();
        let mut builder = egui::ViewportBuilder::default()
            .with_title("ai-terminal status")
            .with_icon(appearance::window_icon())
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_inner_size([width, 48.0])
            .with_taskbar(false)
            .with_active(false)
            .with_always_on_top();
        if !exists {
            builder = builder.with_position([
                placement.pixels[0] / placement.scale,
                placement.pixels[1] / placement.scale,
            ]);
        }
        ctx.show_viewport_deferred(viewport_id(), builder, move |ui, _| {
            let requested = placement_request.lock().unwrap().take();
            #[cfg(windows)]
            let observed = crate::floating_native::sync(
                ui.ctx(),
                [(count.min(10) as f32 * 48.0 + 8.0).max(96.0), 48.0],
                ui.ctx().zoom_factor(),
                requested,
            );
            #[cfg(not(windows))]
            let observed = {
                if let Some(pos) = requested {
                    let scale = ui.ctx().pixels_per_point();
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                            pos[0] / scale,
                            pos[1] / scale,
                        )));
                }
                ui.input(|i| i.viewport().outer_rect)
                    .map(|r| (r.min * ui.ctx().pixels_per_point()).to_array())
            };
            let snapshot = snapshot.lock().unwrap().clone();
            let (action, hint) = body_with_hint(ui, &snapshot, &icons);
            #[cfg(windows)]
            crate::floating_native::hint(ui.ctx(), hint.as_ref());
            #[cfg(not(windows))]
            let _ = hint;
            if let Some(action) = action {
                dispatch(&sender, ui.ctx(), action);
            }
            let close = ui.input(|i| i.viewport().close_requested());
            if close {
                dispatch(&sender, ui.ctx(), Action::Disable);
            }
            if let Some(pos) = observed {
                let key = egui::Id::new("floating-last-position");
                let previous = ui.ctx().data_mut(|d| {
                    let old = d.get_temp::<[f32; 2]>(key);
                    d.insert_temp(key, pos);
                    old
                });
                if previous.is_some_and(|old| old != pos) {
                    dispatch(&sender, ui.ctx(), Action::Position(pos));
                }
            }
        });
    }
}
fn dispatch(sender: &SyncSender<Action>, ctx: &egui::Context, action: Action) {
    if sender.try_send(action).is_ok() {
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}
pub fn status_color(activity: Activity) -> Color32 {
    match activity {
        Activity::Working => chrome::WORKING,
        Activity::Completed => chrome::COMPLETED,
        Activity::Waiting => chrome::WAITING,
        Activity::Failed => chrome::FAILED,
        _ => chrome::SECONDARY,
    }
}
pub fn body(ui: &mut egui::Ui, snapshot: &Snapshot, icons: &Icons) -> Option<Action> {
    body_with_hint(ui, snapshot, icons).0
}
#[derive(Clone, Debug)]
pub(crate) struct HoverHint {
    pub text: String,
    pub anchor: Pos2,
}
fn body_with_hint(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    icons: &Icons,
) -> (Option<Action>, Option<HoverHint>) {
    let mut hint = None;
    let mut action = None;
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
        .show(ui, |ui| {
            let rail = ui.max_rect();
            ui.painter().rect_filled(rail, 12, Color32::from_gray(8));
            // The bare edges move the window; avatars remain dedicated to scrolling and clicks.
            let edges = [
                Rect::from_min_max(rail.min, Pos2::new(rail.right(), rail.top() + 4.0)),
                Rect::from_min_max(Pos2::new(rail.left(), rail.bottom() - 4.0), rail.max),
                Rect::from_min_max(rail.min, Pos2::new(rail.left() + 6.0, rail.bottom())),
                Rect::from_min_max(Pos2::new(rail.right() - 6.0, rail.top()), rail.max),
            ];
            for (index, rect) in edges.into_iter().enumerate() {
                if ui
                    .interact(
                        rect,
                        egui::Id::new(("floating-window-edge", index)),
                        Sense::drag(),
                    )
                    .drag_started()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            }
            let row = Rect::from_min_max(
                rail.min + Vec2::new(6.0, 4.0),
                rail.max - Vec2::new(6.0, 4.0),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(row)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let width = ui.available_width().max(1.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, 40.0),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            let mut dragged = false;
                            let scroll = egui::ScrollArea::horizontal()
                                .id_salt("floating-sessions")
                                .scroll_source(egui::scroll_area::ScrollSource::NONE)
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                )
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    ui.horizontal_top(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        if snapshot.items.is_empty() {
                                            ui.label(
                                                snapshot.locale.text("暂无会话", "No sessions"),
                                            );
                                        }
                                        for item in &snapshot.items {
                                            let (rect, _) = ui.allocate_exact_size(
                                                Vec2::new(44.0, 40.0),
                                                Sense::hover(),
                                            );
                                            let response = ui.interact(
                                                rect,
                                                egui::Id::new(("floating-session", &item.id)),
                                                Sense::click_and_drag(),
                                            );
                                            dragged |= response.dragged();
                                            let selected =
                                                snapshot.active.as_deref() == Some(&item.id);
                                            let avatar = Rect::from_center_size(
                                                rect.center(),
                                                Vec2::splat(chrome::AVATAR),
                                            );
                                            icons.paint(ui.painter(), avatar, item.agent);
                                            if selected {
                                                ui.painter().circle_stroke(
                                                    avatar.center(),
                                                    avatar.width() / 2.0 + 1.5,
                                                    Stroke::new(
                                                        1.0,
                                                        chrome::session_border(item.color),
                                                    ),
                                                );
                                            }
                                            sidebar::paint_status(
                                                ui,
                                                avatar,
                                                item.activity,
                                                item.unread,
                                                Color32::from_gray(8),
                                            );
                                            let status = if item.activity == Activity::Unknown
                                                && item.agent.is_shell()
                                            {
                                                snapshot.locale.text("未启动", "Not started")
                                            } else {
                                                i18n::activity(snapshot.locale, item.activity)
                                            };
                                            let hint_text = format!(
                                                "{} · {} · {}",
                                                item.name,
                                                item.agent.label(),
                                                status
                                            );
                                            if response.hovered()
                                                && !response.dragged()
                                                && !response.clicked()
                                            {
                                                hint = Some(HoverHint {
                                                    text: hint_text,
                                                    anchor: rect.center(),
                                                });
                                            }
                                            if response.clicked() {
                                                action = Some(Action::Activate(item.id.clone()));
                                            }
                                        }
                                    });
                                });
                            if dragged {
                                let delta = ui.input(|i| i.pointer.delta().x);
                                let max =
                                    (scroll.content_size.x - scroll.inner_rect.width()).max(0.0);
                                let mut state = scroll.state;
                                state.offset.x = (state.offset.x - delta).clamp(0.0, max);
                                state.store(ui.ctx(), scroll.id);
                                ui.ctx().request_repaint();
                            }
                        },
                    );
                },
            );
        });
    (action, hint)
}

/// Position in physical screen pixels; width remains in egui points.
pub fn geometry(ctx: &egui::Context, count: usize, saved: Option<[f32; 2]>) -> ([f32; 2], f32) {
    let result = placement(ctx, count, saved, false);
    (result.pixels, result.width)
}

/// Center on the monitor containing the physical position, preserving its height.
pub fn centered_geometry(
    ctx: &egui::Context,
    count: usize,
    saved: Option<[f32; 2]>,
) -> ([f32; 2], f32) {
    let result = placement(ctx, count, saved, true);
    (result.pixels, result.width)
}

struct Placement {
    pixels: [f32; 2],
    width: f32,
    scale: f32,
}

fn placement(
    ctx: &egui::Context,
    count: usize,
    saved: Option<[f32; 2]>,
    centered: bool,
) -> Placement {
    let desired_width = (count.min(10) as f32 * 48.0 + 8.0).max(96.0);
    let root_scale = ctx.pixels_per_point();
    let root_rect = ctx.input(|i| i.viewport().outer_rect);
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::POINT,
            Graphics::Gdi::{
                GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
            },
            UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        };
        // Use the existing native window center to choose a monitor when it straddles screens.
        let zoom = ctx.zoom_factor();
        let child_width = ctx.input(|i| {
            i.raw
                .viewports
                .get(&viewport_id())
                .and_then(|v| Some(v.outer_rect?.width() * v.native_pixels_per_point? * zoom))
        });
        let anchor = saved
            .map(|p| Pos2::new(p[0] + child_width.unwrap_or(0.0) / 2.0, p[1]))
            .or_else(|| root_rect.map(|r| r.center() * root_scale))
            .unwrap_or(Pos2::new(500.0, 300.0));
        let monitor = unsafe {
            MonitorFromPoint(
                POINT {
                    x: anchor.x as i32,
                    y: anchor.y as i32,
                },
                MONITOR_DEFAULTTONEAREST,
            )
        };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
            let mut dpi_x = 0;
            let mut dpi_y = 0;
            let valid_dpi =
                unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }
                    == 0
                    && dpi_x > 0;
            let scale = if valid_dpi {
                dpi_x as f32 / 96.0 * ctx.zoom_factor()
            } else {
                root_scale
            };
            let left = info.rcWork.left as f32;
            let right = info.rcWork.right as f32;
            let top = info.rcWork.top as f32;
            let bottom = info.rcWork.bottom as f32;
            let width = desired_width.min(((right - left) / scale - 16.0).max(1.0));
            let mut pixels =
                saved.unwrap_or([(left + right - width * scale) / 2.0, top + 12.0 * scale]);
            if centered {
                pixels[0] = (left + right - width * scale) / 2.0;
            }
            pixels[0] = pixels[0].clamp(
                left + 8.0 * scale,
                (right - (width + 8.0) * scale).max(left + 8.0 * scale),
            );
            pixels[1] = pixels[1].clamp(
                top + 8.0 * scale,
                (bottom - 56.0 * scale).max(top + 8.0 * scale),
            );
            return Placement {
                pixels,
                width,
                scale,
            };
        }
    }
    let monitor_width = ctx
        .input(|i| i.viewport().monitor_size.map(|s| s.x))
        .unwrap_or(1000.0);
    let width = desired_width.min((monitor_width - 16.0).max(1.0));
    let default = root_rect
        .map(|r| {
            [
                (r.center().x - width / 2.0) * root_scale,
                (r.top() + 12.0) * root_scale,
            ]
        })
        .unwrap_or([40.0 * root_scale, 12.0 * root_scale]);
    let mut pixels = saved.unwrap_or(default);
    if centered {
        pixels[0] = default[0];
    }
    Placement {
        pixels,
        width,
        scale: root_scale,
    }
}

pub fn restore_main(ctx: &egui::Context) {
    for command in [
        egui::ViewportCommand::Visible(true),
        egui::ViewportCommand::Minimized(false),
        egui::ViewportCommand::Focus,
    ] {
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, command);
    }
}

#[cfg(test)]
mod hint_tests {
    use super::*;
    #[test]
    fn short_capsules_request_full_native_hint_without_covering_avatar_hit_targets() {
        for width in [96.0, 152.0, 488.0] {
            let ctx = egui::Context::default();
            appearance::setup(&ctx);
            let icons = Icons::new(&ctx);
            let snapshot = Snapshot {
                items: vec![Item {
                    id: "one".into(),
                    name: "PowerShell：较长的会话名称 ✨".into(),
                    agent: Agent::ZCode,
                    color: None,
                    activity: Activity::Working,
                    unread: false,
                }],
                locale: Locale::ZhCn,
                ..Default::default()
            };
            let mut hint = None;
            for tick in 0..4 {
                let _ = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 48.0))),
                        time: Some(tick as f64 * 0.1),
                        events: vec![egui::Event::PointerMoved(Pos2::new(28.0, 24.0))],
                        ..Default::default()
                    },
                    |ui| {
                        hint = body_with_hint(ui, &snapshot, &icons).1;
                    },
                );
            }
            let hint = hint.expect("native hint metadata");
            assert_eq!(hint.text, "PowerShell：较长的会话名称 ✨ · ZCode · 进行中");
            assert!(hint.anchor.x > 0.0 && hint.anchor.x < width);
        }
    }
}
