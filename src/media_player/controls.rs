//! Native transport controls shared by audio and video; no decoding or timers.
use crate::{chrome, i18n};
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
pub(super) fn height(width: f32) -> f32 {
    if width >= 500.0 {
        72.0
    } else if width >= 280.0 {
        108.0
    } else {
        140.0
    }
}
fn button(ui: &mut egui::Ui, rect: Rect, id: &str, glyph: &str, label: &str) -> Response {
    let response = ui.interact(rect, Id::new(id), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 18.0, Color32::from_gray(62));
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            18.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::new(22.0, FontFamily::Name("codicons".into())),
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
) -> Response {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().slider_width = rect.width();
            ui.visuals_mut().selection.bg_fill = Color32::from_gray(205);
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
pub(super) fn show(ui: &mut egui::Ui, state: State, preview: &mut Option<f64>) -> Actions {
    let width = ui.available_width().max(1.0);
    let (area, _) = ui.allocate_exact_size(Vec2::new(width, height(width)), Sense::hover());
    ui.painter().rect_filled(
        area,
        if width >= 500.0 { 36.0 } else { 20.0 },
        Color32::from_gray(40),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        show_at(ui, area, state, preview)
    })
    .inner
}
fn show_at(ui: &mut egui::Ui, area: Rect, state: State, preview: &mut Option<f64>) -> Actions {
    let locale = i18n::locale(ui.ctx());
    let width = area.width();
    let inner = area.shrink(12.0);
    let wide = width >= 500.0;
    let tiny = width < 280.0;
    let row_y = if wide {
        area.center().y
    } else {
        inner.top() + 18.0
    };
    let play = Rect::from_center_size(egui::pos2(inner.left() + 18.0, row_y), Vec2::splat(36.0));
    let mute_x = inner.right() - if tiny { 18.0 } else { 88.0 };
    let mute = Rect::from_center_size(egui::pos2(mute_x, row_y), Vec2::splat(36.0));
    let time = if state.ready() {
        format!(
            "{} / {}",
            super::time_label(preview.unwrap_or(state.position)),
            super::time_label(state.duration)
        )
    } else {
        locale.text("载入中…", "Loading…").to_owned()
    };
    let font = FontId::proportional(14.0);
    let time_width = ui
        .painter()
        .layout_no_wrap(time.clone(), font.clone(), chrome::TEXT)
        .size()
        .x;
    let time_right = if wide {
        (play.right() + 12.0 + time_width).min(mute.left() - 96.0)
    } else {
        mute.left() - 8.0
    };
    let time_rect = Rect::from_min_max(
        egui::pos2(play.right() + 8.0, row_y - 18.0),
        egui::pos2(time_right.max(play.right() + 9.0), row_y + 18.0),
    );
    let seek_rect = if wide {
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
            egui::pos2(inner.left() + 6.0, inner.top() + 80.0),
            Vec2::new((inner.width() - 56.0).max(1.0), 24.0),
        )
    } else {
        Rect::from_min_max(
            egui::pos2(mute.right() + 6.0, row_y - 16.0),
            egui::pos2(inner.right(), row_y + 16.0),
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
            )
        })
        .inner
        .clicked();
    ui.put(
        time_rect,
        egui::Label::new(egui::RichText::new(time).font(font).color(chrome::TEXT)).truncate(),
    );
    let seek = seek(ui, seek_rect, state, preview).1;
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
    )
    .clicked();
    let mut volume = if state.volume.is_finite() {
        state.volume.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let volume_label = format!("{} {:.0}%", locale.text("音量", "Volume"), volume * 100.0);
    let changed_volume = slider(ui, volume_rect, true, &mut volume, 1.0, &volume_label)
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
                    action = show(ui, self.state, &mut self.preview);
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
