//! Shared sizing and interaction for compact application dialogs.
use crate::{i18n, ui_text};

pub fn content_width(ctx: &egui::Context) -> f32 {
    (ctx.content_rect().width() - 68.0).clamp(200.0, 400.0)
}

pub fn frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::window(&ctx.style_of(egui::Theme::Dark))
        .inner_margin(egui::Margin::symmetric(18, 14))
}

/// Returns true only when a nonempty name has been submitted.
pub fn rename(ctx: &egui::Context, open: &mut bool, text: &mut String) -> bool {
    let width = content_width(ctx);
    let mut done = false;
    egui::Window::new(i18n::tr(ctx, "重命名", "Rename"))
        .id(egui::Id::new("session-rename-window"))
        .open(open)
        .collapsible(false)
        .resizable(false)
        .frame(frame(ctx))
        .default_width(width + 38.0)
        .min_width(width + 38.0)
        .max_width(width + 38.0)
        .default_height(205.0)
        .max_height((ctx.content_rect().height() - 24.0).max(80.0))
        .vscroll(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(i18n::tr(ctx, "会话名称", "Session name"));
            let input = ui_text::edit_sized(ui, text, egui::Id::new("session-rename-input"), width)
                .response;
            done = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.add_space(4.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, 34.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    done |= ui
                        .add_enabled(
                            !text.trim().is_empty(),
                            egui::Button::new(i18n::tr(ctx, "保存", "Save"))
                                .min_size(egui::vec2(80.0, 34.0)),
                        )
                        .clicked();
                },
            );
        });
    done && !text.trim().is_empty()
}
