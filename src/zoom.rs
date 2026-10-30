use crate::model::UiSettings;

/// Window-wide UI scale. Consume before layout so the terminal cannot treat it as scroll.
pub fn input(raw: &mut egui::RawInput, current: f32) -> Option<f32> {
    let mut next = current;
    raw.events.retain(|event| match event {
        egui::Event::MouseWheel {
            unit,
            delta,
            modifiers,
            ..
        } if modifiers.ctrl && !modifiers.alt && !modifiers.shift && delta.y != 0.0 => {
            let steps = match unit {
                egui::MouseWheelUnit::Line => delta.y / 3.0,
                egui::MouseWheelUnit::Point => delta.y / 120.0,
                egui::MouseWheelUnit::Page => delta.y,
            };
            next *= 1.1_f32.powf(steps);
            false
        }
        egui::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } if modifiers.ctrl
            && !modifiers.alt
            && matches!(
                key,
                egui::Key::Plus | egui::Key::Equals | egui::Key::Minus | egui::Key::Num0
            ) =>
        {
            if *pressed {
                next = match key {
                    egui::Key::Num0 => 1.0,
                    egui::Key::Minus => next / 1.1,
                    _ => next * 1.1,
                };
            }
            false
        }
        _ => true,
    });
    next = next.clamp(UiSettings::MIN_ZOOM, UiSettings::MAX_ZOOM);
    ((next - current).abs() > 0.0001).then_some(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wheel(ctrl: bool) -> egui::Event {
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 3.0),
            modifiers: egui::Modifiers {
                ctrl,
                ..Default::default()
            },
            phase: egui::TouchPhase::Move,
        }
    }
    #[test]
    fn zoom_is_global_consumes_ctrl_wheel_and_preserves_plain_scroll() {
        for pointer in [egui::pos2(20.0, 100.0), egui::pos2(700.0, 300.0)] {
            let mut raw = egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    wheel(false),
                    wheel(true),
                ],
                ..Default::default()
            };
            assert!((input(&mut raw, 1.0).unwrap() - 1.1).abs() < 0.0001);
            assert_eq!(
                raw.events
                    .iter()
                    .filter(|e| matches!(e, egui::Event::MouseWheel { .. }))
                    .count(),
                1
            );
        }
        let mut raw = egui::RawInput {
            events: vec![wheel(true)],
            ..Default::default()
        };
        assert_eq!(input(&mut raw, UiSettings::MAX_ZOOM), None);
        assert!(raw.events.is_empty());
    }
}
