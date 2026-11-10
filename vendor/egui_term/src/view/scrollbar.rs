//! Local scrollback control. Geometry never depends on history appearing/disappearing.
use crate::backend::RenderableContent;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;
use egui::{pos2, vec2, Color32, Event, Painter, PointerButton, Rect};

pub(super) const WIDTH: f32 = 12.0;
#[derive(Clone, Default)]
pub(super) struct Drag {
    /// Offset within the thumb, preserving the point grabbed by the pointer.
    grab: Option<f32>,
}
pub(super) struct Geometry {
    track: Rect,
    thumb: Rect,
    history: usize,
}
impl Geometry {
    pub fn new(rect: Rect, content: &RenderableContent) -> Option<Self> {
        let history = content.history_size;
        if history == 0 || content.terminal_mode.contains(TermMode::ALT_SCREEN) {
            return None;
        }
        let track = rect.shrink2(vec2(0.0, 2.0));
        if track.height() <= 0.0 {
            return None;
        }
        let rows = content.terminal_size.screen_lines().max(1) as f32;
        let height = (track.height() * rows / (history as f32 + rows))
            .max(24.0)
            .min(track.height());
        let travel = track.height() - height;
        let top = track.top()
            + travel * (1.0 - content.display_offset.min(history) as f32 / history as f32);
        Some(Self {
            track,
            thumb: Rect::from_min_size(pos2(track.left(), top), vec2(track.width(), height)),
            history,
        })
    }
    fn target(&self, y: f32, grab: f32) -> usize {
        let travel = self.track.height() - self.thumb.height();
        if travel <= 0.0 {
            return 0;
        }
        let fraction = ((y - grab - self.track.top()) / travel).clamp(0.0, 1.0);
        ((1.0 - fraction) * self.history as f32).round() as usize
    }
    pub fn paint(&self, painter: &Painter, hot: bool) {
        let width = 3.0;
        let scale = painter.ctx().pixels_per_point();
        let left = ((self.track.center().x - width * 0.5) * scale).round() / scale;
        let rect = Rect::from_min_size(
            pos2(left, self.thumb.top()),
            vec2(width, self.thumb.height()),
        );
        painter.rect_filled(rect, 2.0, Color32::from_gray(if hot { 160 } else { 95 }));
    }
}
impl Drag {
    pub fn active(&self) -> bool {
        self.grab.is_some()
    }
    pub fn clear(&mut self) {
        self.grab = None;
    }
    /// Own the entire pointer gesture, including release outside the track.
    pub fn event(
        &mut self,
        event: &Event,
        rect: Rect,
        geometry: Option<&Geometry>,
    ) -> (bool, Option<usize>) {
        match event {
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                ..
            } if rect.contains(*pos) => {
                let target = geometry.map(|g| {
                    let grab = if g.thumb.contains(*pos) {
                        pos.y - g.thumb.top()
                    } else {
                        g.thumb.height() * 0.5
                    };
                    self.grab = Some(grab);
                    g.target(pos.y, grab)
                });
                (true, target)
            }
            Event::PointerMoved(pos) if self.active() => (
                true,
                geometry
                    .zip(self.grab)
                    .map(|(g, grab)| g.target(pos.y, grab)),
            ),
            Event::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                ..
            } if self.active() => {
                self.clear();
                (true, None)
            }
            Event::PointerGone => {
                let active = self.active();
                self.clear();
                (active, None)
            }
            Event::PointerMoved(pos) | Event::PointerButton { pos, .. } if rect.contains(*pos) => {
                (true, None)
            }
            _ => (false, None),
        }
    }
}
