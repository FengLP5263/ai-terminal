//! The settings entry changes app preferences without restarting terminals.
use crate::{
    appearance, chrome, dialogs,
    font_choice::FontChoice,
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
    let width = dialogs::content_width(ctx);
    egui::Window::new(locale.text("设置", "Settings"))
        .id(egui::Id::new("app-settings-window"))
        .open(open)
        .collapsible(false)
        .resizable(false)
        .frame(dialogs::frame(ctx))
        .default_width(width + 38.0)
        .min_width(width + 38.0)
        .max_width(width + 38.0)
        .default_height(450.0)
        .max_height((ctx.content_rect().height() - 24.0).max(80.0))
        .vscroll(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(locale.text("界面语言", "Interface language"));
            egui::ComboBox::from_id_salt("language-preference")
                .width(width)
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
            ui.add(
                egui::Label::new(egui::RichText::new(locale.text(
                    "跟随系统语言或手动选择，立即生效。",
                    "Follow system language or choose one. Applies immediately.",
                ))
                .size(13.0)
                .color(chrome::SECONDARY)).wrap(),
            );
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(locale.text("终端字体", "Terminal font"));
            egui::ComboBox::from_id_salt("terminal-font-preference")
                .width(width)
                .selected_text(preferences.terminal_font.family())
                .show_ui(ui, |ui| {
                    for choice in FontChoice::ALL {
                        let available = appearance::font_available(choice);
                        ui.add_enabled_ui(available, |ui| {
                            let label = if available {
                                choice.family().to_owned()
                            } else {
                                format!("{} {}", choice.family(), locale.text("（未安装）", "(Not installed)"))
                            };
                            ui.selectable_value(&mut preferences.terminal_font, choice, label);
                        });
                    }
                });
            ui.add(egui::Label::new(egui::RichText::new(locale.text(
                "立即应用到所有终端，保留运行中的会话。中文和表情沿用系统字体。",
                "Applies instantly to all terminals; sessions keep running. Chinese and emoji keep system fonts.",
            )).size(13.0).color(chrome::SECONDARY)).wrap());
            if !appearance::font_available(preferences.terminal_font) {
                ui.label(locale.text("所选字体未安装，当前使用默认字体。", "The selected font is not installed. Using the default font."));
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.checkbox(
                &mut preferences.floating_enabled,
                locale.text("开启置顶状态浮窗", "Show floating status bar"),
            );
            ui.add(
                egui::Label::new(egui::RichText::new(locale.text(
                    "悬浮显示会话状态，点击即可返回终端。",
                    "See session status above other apps. Click to return.",
                ))
                .size(13.0)
                .color(chrome::SECONDARY)).wrap(),
            );
            ui.add_space(4.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, 34.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    done = ui
                        .add_sized([80.0, 34.0], egui::Button::new(locale.text("完成", "Done")))
                        .clicked();
                },
            );
        });
    if done {
        *open = false;
    }
    before != *preferences
}
