//! A deferred native viewport: only session metadata crosses the window boundary.
use crate::{
    appearance, chrome,
    i18n::{self, Locale},
    model::{Agent, SessionColor, SessionGroup, session_group},
    sidebar::{self, Icons},
    status::Activity,
};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender},
};
use std::time::{Duration, Instant};
pub fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("session-status-bar")
}
pub fn hint_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("session-status-hint")
}
pub fn group_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("session-group-popup")
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
    pub groups: Vec<SessionGroup>,
    pub active: Option<String>,
    pub locale: Locale,
}
impl Snapshot {
    fn representatives(&self) -> Vec<(&Item, Option<&SessionGroup>)> {
        let mut seen = std::collections::HashSet::new();
        self.items
            .iter()
            .filter_map(|item| {
                let group = session_group(&self.groups, &item.id);
                if group.is_some_and(|g| !seen.insert(&g.id)) {
                    return None;
                }
                Some((item, group))
            })
            .collect()
    }
    pub fn entry_count(&self) -> usize {
        self.representatives().len()
    }
    fn hover_group(&self, group: &SessionGroup) -> HoverGroup {
        HoverGroup {
            id: group.id.clone(),
            name: group.name.clone(),
            items: self
                .items
                .iter()
                .filter(|i| group.members.contains(&i.id))
                .cloned()
                .collect(),
            active: self.active.clone(),
            locale: self.locale,
        }
    }
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
    hint: Arc<Mutex<HoverState>>,
    scale: Arc<Mutex<f32>>,
    opacity: Arc<Mutex<f32>>,
    border: Arc<Mutex<bool>>,
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
            hint: Arc::new(Mutex::new(HoverState::default())),
            scale: Arc::new(Mutex::new(1.0)),
            opacity: Arc::new(Mutex::new(1.0)),
            border: Arc::new(Mutex::new(true)),
        }
    }
    pub fn update(&self, snapshot: Snapshot, ctx: &egui::Context) {
        let mut current = self.snapshot.lock().unwrap();
        if *current != snapshot {
            let mut hover = self.hint.lock().unwrap();
            if let Some(hint) = &mut hover.hint
                && let Some(group) = &hint.group
            {
                if let Some(updated) = snapshot.groups.iter().find(|g| g.id == group.id) {
                    hint.group = Some(snapshot.hover_group(updated));
                    if ctx.input(|i| i.raw.viewports.contains_key(&group_viewport_id())) {
                        ctx.request_repaint_of(group_viewport_id());
                    }
                } else {
                    hover.clear();
                }
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
            *current = snapshot;
            ctx.request_repaint_of(viewport_id());
        }
    }
    pub fn actions(&self) -> Vec<Action> {
        self.receiver.try_iter().take(32).collect()
    }
    pub fn dismiss(&self) {
        self.hint.lock().unwrap().clear();
    }
    pub fn set_scale(&self, ctx: &egui::Context, scale: f32) {
        let scale = if scale.is_finite() {
            scale.clamp(
                crate::model::UiSettings::MIN_ZOOM,
                crate::model::UiSettings::MAX_ZOOM,
            )
        } else {
            1.0
        };
        let mut current = self.scale.lock().unwrap();
        if *current != scale {
            *current = scale;
            if ctx.input(|i| i.raw.viewports.contains_key(&viewport_id())) {
                ctx.request_repaint_of(viewport_id());
            }
        }
    }
    pub fn set_opacity(&self, ctx: &egui::Context, opacity: f32) {
        let opacity = crate::model::UiSettings::normalize_floating_opacity(opacity);
        let mut current = self.opacity.lock().unwrap();
        if *current != opacity {
            *current = opacity;
            if ctx.input(|i| i.raw.viewports.contains_key(&viewport_id())) {
                ctx.request_repaint_of(viewport_id());
            }
        }
    }
    pub fn set_border(&self, ctx: &egui::Context, border: bool) {
        let mut current = self.border.lock().unwrap();
        if *current != border {
            *current = border;
            if ctx.input(|i| i.raw.viewports.contains_key(&viewport_id())) {
                ctx.request_repaint_of(viewport_id());
            }
        }
    }
    pub fn show(&self, ctx: &egui::Context, position: Option<[f32; 2]>) {
        let scale = *self.scale.lock().unwrap();
        let hint = {
            let mut hover = self.hint.lock().unwrap();
            hover.expire(Instant::now());
            if let Some(deadline) = hover.close_after {
                ctx.request_repaint_after_for(
                    deadline.saturating_duration_since(Instant::now()),
                    egui::ViewportId::ROOT,
                );
            }
            hover.hint.clone()
        };
        #[cfg(windows)]
        if hint.as_ref().is_some_and(|h| h.group.is_some()) {
            crate::floating_group::show(
                ctx,
                hint.unwrap(),
                scale,
                self.hint.clone(),
                self.sender.clone(),
                self.icons.clone(),
            );
        } else {
            crate::floating_hint::show(ctx, hint, scale);
        }
        let hover = self.hint.clone();
        let snapshot = self.snapshot.clone();
        let sender = self.sender.clone();
        let icons = self.icons.clone();
        let count = snapshot.lock().unwrap().entry_count();
        let placement = placement(ctx, count, position, false, scale);
        let width = placement.width;
        let layout = (count, scale, ctx.zoom_factor());
        let previous = ctx.data_mut(|data| {
            let key = egui::Id::new("floating-window-layout");
            let previous = data.get_temp::<(usize, f32, f32)>(key);
            data.insert_temp(key, layout);
            previous
        });
        let exists = ctx.input(|i| i.raw.viewports.contains_key(&viewport_id()));
        // Saved position is an observation, not a command. Only creation and an
        // intentional layout change can position the native window.
        if !exists
            || previous
                .is_none_or(|(old_count, old_scale, _)| old_count != count || old_scale != scale)
        {
            *self.placement_request.lock().unwrap() = Some(placement.pixels);
        }
        if previous != Some(layout) {
            ctx.request_repaint_of(viewport_id());
            if ctx.input(|i| i.raw.viewports.contains_key(&hint_viewport_id())) {
                ctx.request_repaint_of(hint_viewport_id());
            }
            if ctx.input(|i| i.raw.viewports.contains_key(&group_viewport_id())) {
                ctx.request_repaint_of(group_viewport_id());
            }
        }
        let placement_request = self.placement_request.clone();
        let mut builder = egui::ViewportBuilder::default()
            .with_title("ai-terminal status")
            .with_icon(appearance::window_icon())
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_inner_size([width, 48.0 * scale / ctx.zoom_factor()])
            .with_taskbar(false)
            .with_active(false)
            .with_always_on_top();
        if !exists {
            builder = builder.with_position([
                placement.pixels[0] / placement.scale,
                placement.pixels[1] / placement.scale,
            ]);
        }
        let scale_state = self.scale.clone();
        let opacity_state = self.opacity.clone();
        let border_state = self.border.clone();
        ctx.show_viewport_deferred(viewport_id(), builder, move |ui, _| {
            let scale = *scale_state.lock().unwrap();
            let requested = placement_request.lock().unwrap().take();
            #[cfg(windows)]
            let observed = crate::floating_native::sync(
                ui.ctx(),
                [(count.min(10) as f32 * 48.0 + 8.0).max(96.0), 48.0],
                scale,
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
            let opacity = *opacity_state.lock().unwrap();
            let border = *border_state.lock().unwrap();
            let (action, hint) = scaled_body(ui, &snapshot, &icons, scale, opacity, border);
            let mut current_hint = hover.lock().unwrap();
            if current_hint.bar(hint, Instant::now()) {
                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
            }
            drop(current_hint);
            if let Some(action) = action {
                hover.lock().unwrap().clear();
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
pub(crate) fn dispatch(sender: &SyncSender<Action>, ctx: &egui::Context, action: Action) {
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
    body_with_hint(ui, snapshot, icons, 1.0, 1.0, true).0
}
/// Project base geometry into shared-context points, without scaling a paint layer.
pub fn content_transform(ctx: &egui::Context, scale: f32) -> egui::emath::TSTransform {
    egui::emath::TSTransform::from_scaling(scale / ctx.zoom_factor())
}
pub fn body_at_scale(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    icons: &Icons,
    scale: f32,
) -> Option<Action> {
    scaled_body(ui, snapshot, icons, scale, 1.0, true).0
}
fn scaled_body(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    icons: &Icons,
    scale: f32,
    opacity: f32,
    border: bool,
) -> (Option<Action>, Option<HoverHint>) {
    body_with_hint(
        ui,
        snapshot,
        icons,
        scale / ui.ctx().zoom_factor(),
        opacity,
        border,
    )
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HoverHint {
    pub text: String,
    pub anchor: Pos2,
    pub group: Option<HoverGroup>,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HoverGroup {
    pub id: String,
    pub name: String,
    pub items: Vec<Item>,
    pub active: Option<String>,
    pub locale: Locale,
}
/// One delayed close per leave event permits crossing the gap without idle polling.
#[derive(Default)]
pub(crate) struct HoverState {
    pub hint: Option<HoverHint>,
    bar_inside: bool,
    popup_inside: bool,
    pub close_after: Option<Instant>,
}
impl HoverState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    fn bar(&mut self, hint: Option<HoverHint>, now: Instant) -> bool {
        let previous = self.hint.clone();
        let was_inside = self.bar_inside;
        self.bar_inside = hint.is_some();
        if let Some(hint) = hint {
            if self
                .hint
                .as_ref()
                .and_then(|h| h.group.as_ref())
                .map(|g| &g.id)
                != hint.group.as_ref().map(|g| &g.id)
            {
                self.popup_inside = false;
            }
            self.hint = Some(hint);
            self.close_after = None;
        } else if self.hint.as_ref().is_some_and(|h| h.group.is_some()) {
            if was_inside && !self.popup_inside {
                self.close_after = Some(now + Duration::from_millis(300));
            }
        } else {
            self.clear();
        }
        was_inside != self.bar_inside || previous != self.hint
    }
    pub fn popup(&mut self, inside: bool, now: Instant) -> bool {
        if self.popup_inside == inside {
            return false;
        }
        self.popup_inside = inside;
        if inside {
            self.close_after = None;
        } else if !self.bar_inside {
            self.close_after = Some(now + Duration::from_millis(300));
        }
        true
    }
    fn expire(&mut self, now: Instant) {
        if self.close_after.is_some_and(|d| now >= d) && !self.bar_inside && !self.popup_inside {
            self.clear();
        }
    }
}

fn attention(items: &[Item]) -> Option<&Item> {
    items.iter().max_by_key(|i| match (i.activity, i.unread) {
        (Activity::Failed, true) => 5,
        (Activity::Waiting, _) => 4,
        (Activity::Working, _) => 3,
        (_, true) => 2,
        _ => 0,
    })
}
fn body_with_hint(
    ui: &mut egui::Ui,
    snapshot: &Snapshot,
    icons: &Icons,
    scale: f32,
    opacity: f32,
    border: bool,
) -> (Option<Action>, Option<HoverHint>) {
    let mut hint = None;
    let mut action = None;
    ui.scope(|ui| {
        // Layout and shape at the final size. Scaling a pre-rasterized text/image
        // layer would resample icons whenever the main and floating zoom differ.
        let style = ui.style_mut();
        for font in style.text_styles.values_mut() {
            font.size *= scale;
        }
        style.spacing.interact_size *= scale;
        style.spacing.item_spacing *= scale;
        style.spacing.button_padding *= scale;
        let rail = ui.max_rect();
        ui.painter().rect_filled(
            rail,
            (12.0 * scale).round() as u8,
            Color32::from_gray(8).gamma_multiply(opacity),
        );
        if border {
            ui.painter().rect_stroke(
                rail.shrink(0.5 * scale),
                (11.5 * scale).round() as u8,
                Stroke::new(scale, Color32::from_gray(58)),
                egui::StrokeKind::Inside,
            );
        }
        // The bare edges move the window; avatars remain dedicated to scrolling and clicks.
        let edges = [
            Rect::from_min_max(rail.min, Pos2::new(rail.right(), rail.top() + 4.0 * scale)),
            Rect::from_min_max(
                Pos2::new(rail.left(), rail.bottom() - 4.0 * scale),
                rail.max,
            ),
            Rect::from_min_max(
                rail.min,
                Pos2::new(rail.left() + 6.0 * scale, rail.bottom()),
            ),
            Rect::from_min_max(Pos2::new(rail.right() - 6.0 * scale, rail.top()), rail.max),
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
            rail.min + Vec2::new(6.0, 4.0) * scale,
            rail.max - Vec2::new(6.0, 4.0) * scale,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 4.0 * scale;
                let width = ui.available_width().max(1.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(width, 40.0 * scale),
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
                                    ui.spacing_mut().item_spacing.x = 4.0 * scale;
                                    if snapshot.items.is_empty() {
                                        ui.label(snapshot.locale.text("暂无会话", "No sessions"));
                                    }
                                    for (item, group) in snapshot.representatives() {
                                        let group = group.map(|g| snapshot.hover_group(g));
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::new(44.0, 40.0) * scale,
                                            Sense::hover(),
                                        );
                                        let response = ui.interact(
                                            rect,
                                            egui::Id::new(("floating-session", &item.id)),
                                            Sense::click_and_drag(),
                                        );
                                        dragged |= response.dragged();
                                        let selected = group.as_ref().map_or_else(
                                            || snapshot.active.as_deref() == Some(&item.id),
                                            |g| {
                                                g.items.iter().any(|i| {
                                                    Some(&i.id) == snapshot.active.as_ref()
                                                })
                                            },
                                        );
                                        let avatar = Rect::from_center_size(
                                            rect.center(),
                                            Vec2::splat(chrome::AVATAR * scale),
                                        );
                                        icons.paint(ui.painter(), avatar, item.agent);
                                        if selected {
                                            ui.painter().circle_stroke(
                                                avatar.center(),
                                                avatar.width() / 2.0 + 1.5 * scale,
                                                Stroke::new(
                                                    scale,
                                                    chrome::session_border(item.color),
                                                ),
                                            );
                                        }
                                        let state = group
                                            .as_ref()
                                            .and_then(|g| attention(&g.items))
                                            .unwrap_or(item);
                                        sidebar::paint_status(
                                            ui,
                                            avatar,
                                            state.activity,
                                            state.unread,
                                            Color32::from_gray(8),
                                        );
                                        if let Some(group) = &group {
                                            sidebar::paint_group_mark(
                                                ui.painter(),
                                                avatar,
                                                &group.name,
                                                scale,
                                            );
                                        }
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
                                                anchor: rect.center() / scale,
                                                group,
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
                            let max = (scroll.content_size.x - scroll.inner_rect.width()).max(0.0);
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
    let result = placement(ctx, count, saved, false, 1.0);
    (result.pixels, result.width)
}

/// Center on the monitor containing the physical position, preserving its height.
pub fn centered_geometry(
    ctx: &egui::Context,
    count: usize,
    saved: Option<[f32; 2]>,
) -> ([f32; 2], f32) {
    centered_geometry_at_scale(ctx, count, saved, 1.0)
}
pub fn centered_geometry_at_scale(
    ctx: &egui::Context,
    count: usize,
    saved: Option<[f32; 2]>,
    scale: f32,
) -> ([f32; 2], f32) {
    let result = placement(ctx, count, saved, true, scale);
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
    floating_scale: f32,
) -> Placement {
    let desired_width = (count.min(10) as f32 * 48.0 + 8.0).max(96.0);
    let root_scale = ctx.pixels_per_point();
    let ratio = floating_scale / ctx.zoom_factor();
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
                dpi_x as f32 / 96.0 * floating_scale
            } else {
                root_scale * ratio
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
                width: width * ratio,
                scale: scale / ratio,
            };
        }
    }
    let monitor_width = ctx
        .input(|i| i.viewport().monitor_size.map(|s| s.x))
        .unwrap_or(1000.0);
    let width = (desired_width * ratio).min((monitor_width - 16.0 * ratio).max(1.0));
    let default = root_rect
        .map(|r| {
            [
                (r.center().x - width / 2.0) * root_scale,
                (r.top() + 12.0 * ratio) * root_scale,
            ]
        })
        .unwrap_or([40.0 * ratio * root_scale, 12.0 * ratio * root_scale]);
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
    fn restoring_hidden_floating_preferences_does_not_create_an_empty_window() {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        ctx.set_embed_viewports(false);
        let bar = FloatingBar::new(Icons::new(&ctx));
        let output = ctx.run_ui(egui::RawInput::default(), |_| {
            bar.set_scale(&ctx, 1.25);
            bar.set_opacity(&ctx, 0.5);
            bar.set_border(&ctx, false);
        });
        assert!(!output.viewport_output.contains_key(&viewport_id()));
    }
    #[test]
    fn floating_border_toggle_changes_only_the_outline_and_keeps_avatar_hitboxes() {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let icons = Icons::new(&ctx);
        let snapshot = grouped();
        let mut hitbox = None;
        for border in [true, false, true] {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(104.0, 48.0))),
                    ..Default::default()
                },
                |ui| {
                    body_with_hint(ui, &snapshot, &icons, 1.0, 0.5, border);
                },
            );
            let outlines=output.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.stroke.color==Color32::from_gray(58) && r.stroke.width>0.0)).count();
            assert_eq!(outlines, usize::from(border));
            let current = ctx
                .read_response(egui::Id::new(("floating-session", "a")))
                .unwrap()
                .rect;
            if let Some(before) = hitbox {
                assert_eq!(before, current);
            } else {
                hitbox = Some(current);
            }
        }
    }
    #[test]
    fn initial_grouped_bar_does_not_create_an_empty_member_viewport() {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        ctx.set_embed_viewports(false);
        let bar = FloatingBar::new(Icons::new(&ctx));
        let output = ctx.run_ui(egui::RawInput::default(), |_| {
            bar.update(grouped(), &ctx);
            bar.show(&ctx, Some([100.0, 100.0]));
        });
        assert!(output.viewport_output.contains_key(&viewport_id()));
        assert!(!output.viewport_output.contains_key(&group_viewport_id()));
        assert!(!output.viewport_output.contains_key(&hint_viewport_id()));
    }
    #[test]
    fn grouped_avatar_has_the_same_monochrome_name_mark_at_each_scale() {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let icons = Icons::new(&ctx);
        let snapshot = grouped();
        for scale in [1.0, 1.42, 2.0] {
            let mut output = None;
            for _ in 0..2 {
                output = Some(ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(200.0, 48.0) * scale,
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        body_with_hint(ui, &snapshot, &icons, scale, 1.0, true);
                    },
                ));
            }
            let output = output.unwrap();
            let mark = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.job.text == "开" => Some(t),
                    _ => None,
                })
                .expect("group name initial instead of a count bubble");
            assert_eq!(mark.galley.job.sections[0].format.color, Color32::BLACK);
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Rect(r) if r.fill == Color32::WHITE
                    && r.rect.contains(mark.pos)
                    && (r.rect.width() - 12.0 * scale).abs() < 0.1)));
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Text(t) if t.galley.job.text == "2")));
        }
    }
    fn grouped() -> Snapshot {
        Snapshot {
            items: ["a", "b", "c"]
                .into_iter()
                .map(|id| Item {
                    id: id.into(),
                    name: id.into(),
                    agent: Agent::Shell,
                    color: None,
                    activity: Activity::Idle,
                    unread: false,
                })
                .collect(),
            groups: vec![SessionGroup {
                id: "team".into(),
                name: "开发队 ❤️".into(),
                members: vec!["c".into(), "a".into()],
            }],
            active: Some("c".into()),
            ..Default::default()
        }
    }
    #[test]
    fn group_first_avatar_and_members_follow_session_order_and_attention_is_preserved() {
        let mut snapshot = grouped();
        assert_eq!(snapshot.entry_count(), 2);
        assert_eq!(
            snapshot
                .representatives()
                .iter()
                .map(|(i, _)| i.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        let group = snapshot.hover_group(&snapshot.groups[0]);
        assert_eq!(
            group
                .items
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "c"]
        );
        snapshot.items[2].activity = Activity::Waiting;
        assert_eq!(
            attention(&snapshot.hover_group(&snapshot.groups[0]).items)
                .unwrap()
                .id,
            "c"
        );
        crate::model::reorder(&mut snapshot.items, 2, 0, false);
        assert_eq!(snapshot.representatives()[0].0.id, "c");
        snapshot.items.remove(0);
        assert_eq!(snapshot.representatives()[0].0.id, "a");
        snapshot.groups.clear();
        assert_eq!(snapshot.entry_count(), 2);
    }
    #[test]
    fn group_hover_crosses_window_gap_and_closes_once_after_both_surfaces_are_left() {
        let snapshot = grouped();
        let hint = HoverHint {
            text: String::new(),
            anchor: Pos2::ZERO,
            group: Some(snapshot.hover_group(&snapshot.groups[0])),
        };
        let mut state = HoverState::default();
        let now = Instant::now();
        assert!(state.bar(Some(hint.clone()), now));
        state.bar(None, now);
        state.expire(now + Duration::from_millis(100));
        assert!(state.hint.is_some());
        state.popup(true, now + Duration::from_millis(100));
        state.expire(now + Duration::from_secs(5));
        assert!(state.hint.is_some() && state.close_after.is_none());
        state.popup(false, now + Duration::from_secs(5));
        state.expire(now + Duration::from_millis(5400));
        assert!(state.hint.is_none() && state.close_after.is_none());
        state.bar(Some(hint), now);
        state.bar(None, now);
        state.bar(
            Some(HoverHint {
                text: "single".into(),
                anchor: Pos2::ZERO,
                group: None,
            }),
            now,
        );
        state.bar(None, now);
        assert!(state.hint.is_none());
    }
    #[test]
    fn short_capsules_request_full_native_hint_without_covering_avatar_hit_targets() {
        for width in [96.0, 152.0, 488.0] {
            let ctx = egui::Context::default();
            appearance::setup(&ctx);
            let icons = Icons::new(&ctx);
            let snapshot = Snapshot {
                groups: Vec::new(),
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
                        hint = body_with_hint(ui, &snapshot, &icons, 1.0, 1.0, true).1;
                    },
                );
            }
            let hint = hint.expect("native hint metadata");
            assert_eq!(hint.text, "PowerShell：较长的会话名称 ✨ · ZCode · 进行中");
            assert!(hint.anchor.x > 0.0 && hint.anchor.x < width);
        }
    }
}
