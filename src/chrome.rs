//! Compact desktop workbench tokens and shared vector controls.
use egui::{Color32, Rect, Response, Sense, Stroke, Vec2};
pub const CONTROL: f32 = 32.0;
pub const ACTION_ICON: f32 = 16.0;
pub const AVATAR: f32 = 32.0;
pub const AVATAR_ICON: f32 = 20.0;
pub const ROW: f32 = 64.0;
pub const RAIL_ROW: f32 = 44.0;
pub const TEXT: Color32 = Color32::from_gray(225);
pub const SECONDARY: Color32 = Color32::from_gray(154);
pub const SELECTED: Color32 = Color32::from_gray(40);
pub const HOVER: Color32 = Color32::from_gray(28);
pub const BORDER: Color32 = Color32::from_gray(62);
pub const NEW_TERMINAL_ID: &str = "new-terminal-sidebar";
#[derive(Clone, Copy)]
pub enum ActionIcon {
    Add,
    Settings,
    Close,
    Collapse,
    Expand,
    Edit,
}

pub fn paint_action(painter: &egui::Painter, rect: Rect, icon: ActionIcon, color: Color32) {
    let center = rect.center();
    let scale = rect.width() / 16.0;
    let point = |x: f32, y: f32| center + egui::vec2(x, y) * scale;
    let stroke = Stroke::new(1.5 * scale, color);
    match icon {
        ActionIcon::Settings => {
            let teeth = (0..32)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 32.0;
                    let radius = if i % 4 < 2 { 6.8 } else { 5.0 };
                    center + egui::vec2(angle.cos(), angle.sin()) * radius * scale
                })
                .collect();
            painter.add(egui::Shape::closed_line(teeth, stroke));
            painter.circle_stroke(center, 2.4 * scale, stroke);
        }
        ActionIcon::Close => {
            painter.line_segment([point(-4.0, -4.0), point(4.0, 4.0)], stroke);
            painter.line_segment([point(-4.0, 4.0), point(4.0, -4.0)], stroke);
        }
        ActionIcon::Add => {
            painter.line_segment([point(-5.5, 0.0), point(5.5, 0.0)], stroke);
            painter.line_segment([point(0.0, -5.5), point(0.0, 5.5)], stroke);
        }
        ActionIcon::Collapse | ActionIcon::Expand => {
            painter.rect_stroke(
                Rect::from_min_max(point(-7.0, -6.0), point(7.0, 6.0)),
                1,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([point(-2.0, -5.5), point(-2.0, 5.5)], stroke);
            let points = if matches!(icon, ActionIcon::Collapse) {
                vec![point(3.5, -2.5), point(1.0, 0.0), point(3.5, 2.5)]
            } else {
                vec![point(1.0, -2.5), point(3.5, 0.0), point(1.0, 2.5)]
            };
            painter.add(egui::Shape::line(points, stroke));
        }
        ActionIcon::Edit => {
            painter.add(egui::Shape::closed_line(
                vec![
                    point(-5.5, 2.5),
                    point(2.5, -5.5),
                    point(5.5, -2.5),
                    point(-2.5, 5.5),
                ],
                stroke,
            ));
            painter.line_segment([point(-5.5, 2.5), point(-6.0, 6.0)], stroke);
            painter.line_segment([point(-6.0, 6.0), point(-2.5, 5.5)], stroke);
            painter.line_segment([point(1.0, -4.0), point(4.0, -1.0)], stroke);
        }
    }
}

pub fn icon_at(
    ui: &mut egui::Ui,
    rect: Rect,
    id: egui::Id,
    icon: ActionIcon,
    label: &str,
) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let fill = if response.is_pointer_button_down_on() {
        Color32::from_gray(48)
    } else if response.hovered() {
        Color32::from_gray(35)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6, fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            6,
            Stroke::new(1.0, Color32::from_rgb(136, 180, 246)),
            egui::StrokeKind::Inside,
        );
    }
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(ACTION_ICON));
    paint_action(
        ui.painter(),
        glyph,
        icon,
        if ui.is_enabled() {
            Color32::from_gray(210)
        } else {
            Color32::from_gray(110)
        },
    );
    response.on_hover_text(label)
}

pub fn icon_button(ui: &mut egui::Ui, id: egui::Id, icon: ActionIcon, label: &str) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(CONTROL), Sense::hover());
    icon_at(ui, rect, id, icon, label)
}

pub fn paint_shell(painter: &egui::Painter, rect: Rect, agent: crate::model::Agent) {
    use crate::model::Agent;
    let center = rect.center();
    let scale = rect.width() / 20.0;
    let p = |x: f32, y: f32| center + egui::vec2(x, y) * scale;
    let color = match agent {
        Agent::Shell => Color32::from_rgb(156, 190, 232),
        Agent::Bash => Color32::from_rgb(133, 200, 145),
        _ => Color32::from_gray(210),
    };
    let stroke = Stroke::new(1.5 * scale, color);
    if agent == Agent::Bash {
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [p(6.0, -6.0), p(-8.0, -11.0), p(-8.0, 0.0), p(0.0, 0.0)],
            false,
            Color32::TRANSPARENT,
            stroke,
        ));
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [p(0.0, 0.0), p(9.0, 0.0), p(8.0, 11.0), p(-6.0, 6.0)],
            false,
            Color32::TRANSPARENT,
            stroke,
        ));
        painter.line_segment([p(0.0, -9.0), p(0.0, 9.0)], stroke);
    } else {
        if agent == Agent::Shell {
            painter.add(egui::Shape::closed_line(
                vec![p(-6.5, -7.5), p(9.0, -7.5), p(6.5, 7.5), p(-9.0, 7.5)],
                stroke,
            ));
        } else {
            painter.rect_stroke(
                Rect::from_min_max(p(-9.0, -7.5), p(9.0, 7.5)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        painter.add(egui::Shape::line(
            vec![p(-4.0, -3.5), p(-0.5, 0.0), p(-4.0, 3.5)],
            stroke,
        ));
        painter.line_segment([p(1.5, 3.5), p(5.5, 3.5)], stroke);
    }
}

/// Opaque row surfaces also provide the exact background for lifecycle badges.
pub fn session_fill(
    color: Option<crate::model::SessionColor>,
    selected: bool,
    hovered: bool,
) -> Color32 {
    if let Some(color) = color {
        tint(
            color,
            if selected {
                0.38
            } else if hovered {
                0.28
            } else {
                0.18
            },
        )
    } else if selected {
        SELECTED
    } else if hovered {
        HOVER
    } else {
        crate::appearance::SIDEBAR_BG
    }
}
pub fn session_border(color: Option<crate::model::SessionColor>) -> Color32 {
    color.map(|color| tint(color, 0.65)).unwrap_or(BORDER)
}
fn tint(color: crate::model::SessionColor, amount: f32) -> Color32 {
    let rgb = color
        .rgb()
        .map(|v| (18.0 + (v as f32 - 18.0) * amount).round() as u8);
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}
