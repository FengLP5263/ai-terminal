//! Native transport controls shared by audio and video; no decoding or timers.
use crate::{chrome, i18n, project_media::MediaKind};
use egui::{Align2, Color32, FontFamily, FontId, Id, Rect, Response, Sense, Vec2};

#[derive(Clone, Copy)]
pub(super) struct State {
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
}
impl State {
    fn ready(self) -> bool {
        self.duration.is_finite() && self.duration > 0.0
    }
}
#[derive(Default)]
pub(super) struct Actions {
    pub toggle_play: bool,
    pub toggle_mute: bool,
    pub seek: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Default)]
pub(super) struct VideoOverlay {
    last_activity: Option<f64>,
    pointer_inside: bool,
    keyboard: bool,
}

impl VideoOverlay {
    fn update(&mut self, time: f64, paused: bool, active: bool) -> f32 {
        if paused {
            self.last_activity = None;
            return 1.0;
        }
        if active {
            self.last_activity = Some(time);
        }
        let idle = time - *self.last_activity.get_or_insert(time);
        let remaining = (1.0 - (idle - 2.5) / 0.28).clamp(0.0, 1.0) as f32;
        remaining * remaining * (3.0 - 2.0 * remaining)
    }

    pub(super) fn opacity(
        &mut self,
        ui: &egui::Ui,
        player: Rect,
        controls: Rect,
        paused: bool,
        seeking: bool,
    ) -> f32 {
        let inside = ui.rect_contains_pointer(player);
        let (time, moved, pointer_down, tab) = ui.input(|input| {
            (
                input.time,
                input.pointer.delta() != Vec2::ZERO,
                input.pointer.any_down() || input.pointer.any_released(),
                input.key_pressed(egui::Key::Tab),
            )
        });
        let pointer_active = inside && (moved || pointer_down || !self.pointer_inside);
        self.pointer_inside = inside;
        if pointer_active {
            self.keyboard = false;
        }
        if tab {
            self.keyboard = true;
        }
        let keyboard_focus = self.keyboard
            && ui
                .ctx()
                .memory(|memory| memory.focused())
                .and_then(|id| ui.ctx().read_response(id))
                .is_some_and(|response| controls.contains_rect(response.rect));
        self.update(
            time,
            paused,
            pointer_active || tab || keyboard_focus || seeking,
        )
    }
}

pub(super) fn height(width: f32) -> f32 {
    if width >= 500.0 {
        72.0
    } else if width >= 280.0 {
        108.0
    } else {
        140.0
    }
}

fn video_height(width: f32) -> f32 {
    if width < 240.0 { 86.0 } else { 62.0 }
}

pub(super) fn video_layout(canvas: Rect, width: f32, original: Vec2) -> (Rect, Rect, Rect) {
    let scale = (width / original.x).min((canvas.height() - 32.0) / original.y);
    let picture_size = original * scale;
    let controls_width = picture_size.x.max(180.0);
    let controls_height = video_height(controls_width);
    // Preserve a visible picture and usable controls even for very short or narrow video.
    let player_size = Vec2::new(
        controls_width,
        picture_size
            .y
            .max(controls_height + (picture_size.y * 0.5).min(56.0)),
    );
    let player = Rect::from_center_size(canvas.center(), player_size);
    let picture = Rect::from_min_size(
        egui::pos2(player.center().x - picture_size.x / 2.0, player.top()),
        picture_size,
    );
    let controls = Rect::from_min_size(
        egui::pos2(player.left(), player.bottom() - controls_height),
        Vec2::new(controls_width, controls_height),
    );
    (player, picture, controls)
}

fn paint_background(ui: &egui::Ui, area: Rect, kind: MediaKind) {
    if kind == MediaKind::Audio {
        let radius = if area.width() >= 500.0 { 36.0 } else { 20.0 };
        ui.painter()
            .rect_filled(area, radius, Color32::from_gray(40));
    } else {
        let mut shade = egui::Mesh::default();
        for (y, alpha) in [
            (area.top() - 20.0, 0),
            (area.top(), 155),
            (area.bottom(), 220),
        ] {
            let color = Color32::from_black_alpha(alpha);
            shade.colored_vertex(egui::pos2(area.left(), y), color);
            shade.colored_vertex(egui::pos2(area.right(), y), color);
        }
        for row in [0, 2] {
            shade.add_triangle(row, row + 1, row + 3);
            shade.add_triangle(row, row + 3, row + 2);
        }
        ui.painter().add(egui::Shape::mesh(shade));
    }
}
fn button(
    ui: &mut egui::Ui,
    rect: Rect,
    id: &str,
    glyph: &str,
    label: &str,
    video: bool,
) -> Response {
    let response = ui.interact(rect, Id::new(id), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.has_focus() {
        let (radius, fill) = if video {
            (8.0, Color32::from_white_alpha(34))
        } else {
            (18.0, Color32::from_gray(62))
        };
        ui.painter().rect_filled(rect, radius, fill);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            if video { 8.0 } else { 18.0 },
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::new(
            if video { 18.0 } else { 22.0 },
            FontFamily::Name("codicons".into()),
        ),
        if ui.is_enabled() {
            chrome::TEXT
        } else {
            chrome::SECONDARY
        },
    );
    response.on_hover_text(label)
}
fn slider(
    ui: &mut egui::Ui,
    rect: Rect,
    enabled: bool,
    value: &mut f64,
    end: f64,
    label: &str,
    video: bool,
) -> Response {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().slider_width = rect.width();
            ui.visuals_mut().selection.bg_fill = Color32::from_gray(205);
            if video {
                ui.spacing_mut().interact_size.y = rect.height();
                ui.spacing_mut().slider_rail_height = 3.0;
                let visuals = ui.visuals_mut();
                visuals.selection.bg_fill = chrome::TEXT;
                visuals.widgets.inactive.bg_fill = Color32::from_white_alpha(88);
                visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
                for widget in [
                    &mut visuals.widgets.inactive,
                    &mut visuals.widgets.hovered,
                    &mut visuals.widgets.active,
                ] {
                    widget.expansion = -5.0;
                    widget.fg_stroke = egui::Stroke::NONE;
                }
                visuals.widgets.hovered.bg_fill = chrome::TEXT;
                visuals.widgets.active.bg_fill = Color32::WHITE;
            }
            let response = ui.add_enabled(
                enabled,
                egui::Slider::new(value, 0.0..=end)
                    .show_value(false)
                    .handle_shape(egui::style::HandleShape::Circle)
                    .trailing_fill(true),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Slider, enabled, label)
            });
            response.on_hover_text(label)
        },
    )
    .inner
}
pub(super) fn seek(
    ui: &mut egui::Ui,
    rect: Rect,
    state: State,
    preview: &mut Option<f64>,
    video: bool,
) -> (Response, Option<f64>) {
    let ready = state.ready();
    let mut position = preview.unwrap_or(state.position).max(0.0);
    if !position.is_finite() {
        position = 0.0;
    }
    let locale = i18n::locale(ui.ctx());
    let response = slider(
        ui,
        rect,
        ready,
        &mut position,
        if ready { state.duration } else { 1.0 },
        locale.text("播放进度", "Playback position"),
        video,
    );
    if ready && response.changed() {
        *preview = Some(position);
    }
    let commit = ready && (response.drag_stopped() || (response.changed() && !response.dragged()));
    if commit {
        *preview = None;
    }
    (response, commit.then_some(position))
}
pub(super) fn show(
    ui: &mut egui::Ui,
    state: State,
    preview: &mut Option<f64>,
    kind: MediaKind,
) -> Actions {
    let width = ui.available_width().max(1.0);
    let video = kind == MediaKind::Video;
    let height = if video {
        video_height(width)
    } else {
        height(width)
    };
    let (area, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    paint_background(ui, area, kind);
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        show_at(ui, area, state, preview, video)
    })
    .inner
}
fn show_at(
    ui: &mut egui::Ui,
    area: Rect,
    state: State,
    preview: &mut Option<f64>,
    video: bool,
) -> Actions {
    let locale = i18n::locale(ui.ctx());
    let width = area.width();
    let inner = area.shrink(12.0);
    let wide = width >= 500.0;
    let tiny = width < if video { 240.0 } else { 280.0 };
    let row_y = if video {
        area.top() + 40.0
    } else if wide {
        area.center().y
    } else {
        inner.top() + 18.0
    };
    let button_radius = if video { 16.0 } else { 18.0 };
    let button_size = Vec2::splat(button_radius * 2.0);
    let play = Rect::from_center_size(egui::pos2(inner.left() + button_radius, row_y), button_size);
    let mute_x = inner.right() - if tiny { button_radius } else { 88.0 };
    let mute = Rect::from_center_size(egui::pos2(mute_x, row_y), button_size);
    let time = if state.ready() {
        format!(
            "{} / {}",
            super::time_label(preview.unwrap_or(state.position)),
            super::time_label(state.duration)
        )
    } else {
        locale.text("载入中…", "Loading…").to_owned()
    };
    let font = FontId::proportional(if video { 13.0 } else { 14.0 });
    let time_width = ui
        .painter()
        .layout_no_wrap(time.clone(), font.clone(), chrome::TEXT)
        .size()
        .x;
    let time_right = if wide && !video {
        (play.right() + 12.0 + time_width).min(mute.left() - 96.0)
    } else {
        mute.left() - 8.0
    };
    let time_rect = Rect::from_min_max(
        egui::pos2(play.right() + 8.0, row_y - 18.0),
        egui::pos2(time_right.max(play.right() + 9.0), row_y + 18.0),
    );
    let seek_rect = if video {
        Rect::from_min_size(
            egui::pos2(inner.left(), area.top() + 1.0),
            Vec2::new(inner.width(), 22.0),
        )
    } else if wide {
        Rect::from_min_max(
            egui::pos2(time_rect.right() + 12.0, row_y - 16.0),
            egui::pos2(mute.left() - 12.0, row_y + 16.0),
        )
    } else {
        Rect::from_min_size(
            egui::pos2(inner.left() + 6.0, inner.top() + 44.0),
            Vec2::new((inner.width() - 12.0).max(1.0), 24.0),
        )
    };
    let volume_rect = if tiny {
        Rect::from_min_size(
            egui::pos2(
                inner.left() + 6.0,
                if video {
                    area.top() + 60.0
                } else {
                    inner.top() + 80.0
                },
            ),
            Vec2::new((inner.width() - 56.0).max(1.0), 24.0),
        )
    } else {
        Rect::from_min_max(
            egui::pos2(mute.right() + 6.0, row_y - if video { 11.0 } else { 16.0 }),
            egui::pos2(inner.right(), row_y + if video { 11.0 } else { 16.0 }),
        )
    };
    let toggle_play = ui
        .add_enabled_ui(state.ready(), |ui| {
            button(
                ui,
                play,
                "media-play-toggle",
                if state.paused { "\u{eb2c}" } else { "\u{ead1}" },
                if state.paused {
                    locale.text("播放", "Play")
                } else {
                    locale.text("暂停", "Pause")
                },
                video,
            )
        })
        .inner
        .clicked();
    let label =
        egui::Label::new(egui::RichText::new(&time).font(font).color(chrome::TEXT)).truncate();
    ui.put(
        time_rect,
        if video {
            label.halign(egui::Align::Min)
        } else {
            label
        },
    )
    .on_hover_text(time);
    let seek = seek(ui, seek_rect, state, preview, video).1;
    let muted = state.volume <= 0.0;
    let toggle_mute = button(
        ui,
        mute,
        "media-mute-toggle",
        if muted { "\u{eb24}" } else { "\u{eb75}" },
        if muted {
            locale.text("取消静音", "Unmute")
        } else {
            locale.text("静音", "Mute")
        },
        video,
    )
    .clicked();
    let mut volume = if state.volume.is_finite() {
        state.volume.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let volume_label = format!("{} {:.0}%", locale.text("音量", "Volume"), volume * 100.0);
    let changed_volume = slider(
        ui,
        volume_rect,
        true,
        &mut volume,
        1.0,
        &volume_label,
        video,
    )
    .changed()
    .then_some(volume);
    if tiny {
        ui.painter().text(
            egui::pos2(inner.right(), volume_rect.center().y),
            Align2::RIGHT_CENTER,
            format!("{:.0}%", volume * 100.0),
            FontId::proportional(12.0),
            chrome::SECONDARY,
        );
    }
    Actions {
        toggle_play,
        toggle_mute,
        seek,
        volume: changed_volume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, PointerButton, Pos2};
    struct Harness {
        ctx: egui::Context,
        state: State,
        preview: Option<f64>,
        width: f32,
        tick: u32,
    }
    impl Harness {
        fn new(width: f32) -> Self {
            let ctx = egui::Context::default();
            crate::appearance::setup(&ctx);
            Self {
                ctx,
                state: State {
                    paused: true,
                    position: 0.0,
                    duration: 3600.0,
                    volume: 0.6,
                },
                preview: None,
                width,
                tick: 0,
            }
        }
        fn frame(&mut self, events: Vec<Event>) -> Actions {
            self.tick += 1;
            let mut action = Actions::default();
            let _ = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 500.0))),
                    time: Some(self.tick as f64 * 0.1),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.set_width(self.width);
                    action = show(ui, self.state, &mut self.preview, MediaKind::Audio);
                },
            );
            action
        }
        fn click(&mut self, id: &str) -> Actions {
            let pos = self.ctx.read_response(Id::new(id)).unwrap().rect.center();
            self.frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            self.frame(vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }])
        }
    }
    #[test]
    fn video_controls_fade_during_idle_playback_and_show_on_activity_or_pause() {
        let mut overlay = VideoOverlay::default();
        assert_eq!(overlay.update(10.0, false, false), 1.0);
        assert_eq!(overlay.update(12.49, false, false), 1.0);
        assert!((overlay.update(12.64, false, false) - 0.5).abs() < 0.01);
        assert!(overlay.update(12.7, false, false) < 0.5);
        assert_eq!(overlay.update(12.8, false, false), 0.0);
        assert_eq!(overlay.update(13.0, false, true), 1.0);
        assert!((0.0..0.5).contains(&overlay.update(15.7, false, false)));
        assert_eq!(overlay.update(15.71, false, true), 1.0);
        assert_eq!(overlay.update(20.0, true, false), 1.0);
        assert_eq!(overlay.update(41.0, false, false), 1.0);
    }
    #[test]
    fn video_controls_are_anchored_inside_the_player_for_wide_and_narrow_frames() {
        for (size, original) in [
            ((800.0, 500.0), (640.0, 360.0)),
            ((340.0, 420.0), (360.0, 640.0)),
            ((212.0, 220.0), (640.0, 140.0)),
        ] {
            let canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(size.0, size.1));
            let (player, picture, controls) = video_layout(
                canvas,
                canvas.width() - 32.0,
                Vec2::new(original.0, original.1),
            );
            assert!(canvas.contains_rect(player));
            assert!(player.expand(0.01).contains_rect(picture));
            assert!(player.expand(0.01).contains_rect(controls));
            assert_eq!(player.center(), canvas.center());
            assert!((controls.bottom() - player.bottom()).abs() <= 0.01);
            assert!(controls.top() > picture.top());
            if original == (640.0, 360.0) {
                assert!(picture.expand(0.01).contains_rect(controls));
            }
        }
    }
    #[test]
    fn transport_buttons_work_at_wide_and_narrow_widths_and_wait_for_metadata() {
        for width in [180.0, 320.0, 560.0] {
            let mut h = Harness::new(width);
            h.frame(vec![]);
            let play = h
                .ctx
                .read_response(Id::new("media-play-toggle"))
                .unwrap()
                .rect;
            let mute = h
                .ctx
                .read_response(Id::new("media-mute-toggle"))
                .unwrap()
                .rect;
            assert!(play.right() <= mute.left());
            assert!(play.left() >= 0.0 && mute.right() <= width);
            assert!(h.click("media-play-toggle").toggle_play);
            h.state.paused = false;
            h.frame(vec![]);
            assert!(h.click("media-play-toggle").toggle_play);
            assert!(h.click("media-mute-toggle").toggle_mute);
            h.state.volume = 0.0;
            h.frame(vec![]);
            assert!(h.click("media-mute-toggle").toggle_mute);
            h.state.duration = f64::NAN;
            h.frame(vec![]);
            assert!(!h.click("media-play-toggle").toggle_play);
        }
    }
    #[test]
    fn seek_previews_a_drag_and_commits_only_on_release() {
        let ctx = egui::Context::default();
        let mut preview = None;
        let mut frame = |events| {
            let mut result = None;
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(600.0, 300.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    result = Some(seek(
                        ui,
                        Rect::from_min_size(Pos2::new(20.0, 20.0), egui::vec2(240.0, 32.0)),
                        State {
                            paused: true,
                            position: 0.0,
                            duration: 100.0,
                            volume: 0.0,
                        },
                        &mut preview,
                        true,
                    ));
                },
            );
            let (response, commit) = result.unwrap();
            (response.rect, commit, preview)
        };
        let (rect, _, _) = frame(vec![]);
        let start = egui::pos2(rect.left() + 8.0, rect.center().y);
        let end = egui::pos2(rect.right() - 40.0, rect.center().y);
        frame(vec![
            Event::PointerMoved(start),
            Event::PointerButton {
                pos: start,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let (_, commit, pending) = frame(vec![Event::PointerMoved(end)]);
        assert!(commit.is_none());
        assert!(pending.is_some_and(|p| p > 50.0 && p <= 100.0));
        let (_, commit, pending) = frame(vec![Event::PointerButton {
            pos: end,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert!(commit.is_some_and(|p| p > 50.0 && p <= 100.0));
        assert!(pending.is_none());
    }
}
