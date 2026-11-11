//! Shared sizing and interaction for compact application dialogs.
use crate::{i18n, ui_text};

pub const TITLE_SIZE: f32 = 18.0;
pub const SECTION_SIZE: f32 = 16.0;
pub const BODY_SIZE: f32 = 14.0;
pub const SMALL_SIZE: f32 = 12.0;
pub const FIELD_HEIGHT: f32 = 36.0;
pub const GAP: f32 = 8.0;
pub const SECTION_GAP: f32 = 16.0;
const ACTION_WIDTH: f32 = 88.0;

pub fn content_width(ctx: &egui::Context) -> f32 {
    (ctx.content_rect().width() - 68.0).clamp(200.0, 360.0)
}

pub fn frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::window(&ctx.style_of(egui::Theme::Dark))
        .inner_margin(egui::Margin::symmetric(20, 12))
        .corner_radius(16)
        .fill(egui::Color32::from_gray(24))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(52)))
}

/// Scope the workbench form style to a dialog, keeping terminal typography intact.
pub fn body(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.override_font_id = None;
    for (role, size) in [
        (egui::TextStyle::Body, BODY_SIZE),
        (egui::TextStyle::Button, BODY_SIZE),
        (egui::TextStyle::Small, SMALL_SIZE),
        (egui::TextStyle::Heading, TITLE_SIZE),
    ] {
        style
            .text_styles
            .insert(role, egui::FontId::proportional(size));
    }
    style.spacing.item_spacing = egui::vec2(GAP, GAP);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = FIELD_HEIGHT;
    style.visuals.override_text_color = Some(crate::chrome::TEXT);
    style.visuals.weak_text_color = Some(egui::Color32::from_gray(178));
    for (widget, fill) in [
        (&mut style.visuals.widgets.inactive, 40),
        (&mut style.visuals.widgets.hovered, 48),
        (&mut style.visuals.widgets.active, 56),
    ] {
        widget.corner_radius = egui::CornerRadius::same(8);
        widget.bg_fill = egui::Color32::from_gray(fill);
        widget.weak_bg_fill = egui::Color32::from_gray(fill);
        widget.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(68));
    }
}

/// The same native form surface and trailing actions for session/exit confirmations.
pub fn confirm(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    message: &str,
    action: &str,
) -> Option<bool> {
    let width = content_width(ctx);
    let mut open = true;
    let mut result = None;
    egui::Window::new(egui::RichText::new(title).size(TITLE_SIZE).strong())
        .id(egui::Id::new(id))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .frame(frame(ctx))
        .auto_sized()
        .default_width(width + 50.0)
        .max_width(width + 50.0)
        .max_height((ctx.content_rect().height() - 32.0).max(80.0))
        .vscroll(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            body(ui);
            ui.set_width(width);
            ui.label(message);
            ui.add_space(GAP);
            ui.allocate_ui_with_layout(
                egui::vec2(width, FIELD_HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if ui
                        .add_sized([ACTION_WIDTH, FIELD_HEIGHT], egui::Button::new(action))
                        .clicked()
                    {
                        result = Some(true);
                    }
                    if ui
                        .add_sized(
                            [ACTION_WIDTH, FIELD_HEIGHT],
                            egui::Button::new(i18n::tr(ctx, "取消", "Cancel")),
                        )
                        .clicked()
                    {
                        result = Some(false);
                    }
                },
            );
        });
    if !open {
        result = Some(false);
    }
    result
}

/// Returns true only when a nonempty name has been submitted.
pub fn rename(ctx: &egui::Context, open: &mut bool, text: &mut String) -> bool {
    let width = content_width(ctx);
    let mut done = false;
    egui::Window::new(
        egui::RichText::new(i18n::tr(ctx, "重命名", "Rename"))
            .size(TITLE_SIZE)
            .strong(),
    )
    .id(egui::Id::new("session-rename-window"))
    .open(open)
    .collapsible(false)
    .resizable(false)
    .frame(frame(ctx))
    .auto_sized()
    .default_width(width + 50.0)
    .min_width(width + 50.0)
    .max_width(width + 50.0)
    .max_height((ctx.content_rect().height() - 24.0).max(80.0))
    .vscroll(true)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .show(ctx, |ui| {
        body(ui);
        ui.set_width(width);
        ui.label(i18n::tr(ctx, "会话名称", "Session name"));
        let input =
            ui_text::edit_sized(ui, text, egui::Id::new("session-rename-input"), width).response;
        done = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(4.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, FIELD_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                done |= ui
                    .add_enabled(
                        !text.trim().is_empty(),
                        egui::Button::new(i18n::tr(ctx, "保存", "Save"))
                            .min_size(egui::vec2(ACTION_WIDTH, FIELD_HEIGHT)),
                    )
                    .clicked();
            },
        );
    });
    done && !text.trim().is_empty()
}
