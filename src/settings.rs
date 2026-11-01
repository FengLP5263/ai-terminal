//! The settings entry changes app preferences without restarting terminals.
use crate::{
    chrome,
    i18n::{self, Language},
    model::UiSettings,
};
pub fn show(ctx: &egui::Context, open: &mut bool, preferences: &mut UiSettings) -> bool {
    if !*open {
        return false;
    }
    let before = *preferences;
    let locale = i18n::locale(ctx);
    let mut done = false;
    egui::Window::new(locale.text("设置", "Settings"))
        .id(egui::Id::new("app-settings-window"))
        .open(open)
        .collapsible(false)
        .resizable(false)
        .default_width(390.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add_space(4.0);
            ui.label(locale.text("界面语言", "Interface language"));
            egui::ComboBox::from_id_salt("language-preference")
                .width(300.0)
                .selected_text(preferences.language.label(locale))
                .show_ui(ui, |ui| {
                    for language in [Language::System, Language::ZhCn, Language::En] {
                        ui.selectable_value(
                            &mut preferences.language,
                            language,
                            language.label(locale),
                        );
                    }
                });
            ui.label(
                egui::RichText::new(locale.text(
                    "自动检测系统语言，也可以手动选择。切换立即生效。",
                    "Detect the system language or choose one manually. Changes apply immediately.",
                ))
                .size(13.0)
                .color(chrome::SECONDARY),
            );
            ui.separator();
            ui.checkbox(
                &mut preferences.floating_enabled,
                locale.text("开启置顶状态浮窗", "Show floating status bar"),
            );
            ui.label(
                egui::RichText::new(locale.text(
                    "在其他应用上方查看所有会话的状态。点击会话，即可回到对应终端。",
                    "Monitor sessions above other apps. Click a session to return to its terminal.",
                ))
                .size(13.0)
                .color(chrome::SECONDARY),
            );
            ui.add_space(4.0);
            done = ui.button(locale.text("完成", "Done")).clicked();
        });
    if done {
        *open = false;
    }
    before != *preferences
}
