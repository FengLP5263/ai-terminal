//! Immediate, auto-saved preferences in a scoped native workbench form.
use crate::{
    appearance, dialogs,
    font_choice::FontChoice,
    i18n::{self, Language},
    model::UiSettings,
};
use egui::{Align, Color32, Layout, Response, RichText};

fn section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::from_gray(28))
        .corner_radius(12)
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(44)))
        .inner_margin(16)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = dialogs::GAP;
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(18.0).strong());
            ui.add_space(4.0);
            contents(ui);
        });
}
fn row(ui: &mut egui::Ui, label: &str, control: impl FnOnce(&mut egui::Ui, f32) -> Response) {
    let width = ui.available_width();
    if width >= 420.0 {
        let controls = (width * 0.60).floor();
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, dialogs::FIELD_HEIGHT),
            egui::Sense::hover(),
        );
        let label_rect = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(width - controls - dialogs::GAP, rect.height()),
        );
        let mut label_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("label", label))
                .max_rect(label_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let label_response = label_ui.label(label);
        let control_rect =
            egui::Rect::from_min_max(egui::pos2(rect.right() - controls, rect.top()), rect.max);
        let mut control_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("control", label))
                .max_rect(control_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        control(&mut control_ui, controls).labelled_by(label_response.id);
    } else {
        let label = ui.label(label);
        ui.allocate_ui_with_layout(
            egui::vec2(width, dialogs::FIELD_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                control(ui, width).labelled_by(label.id);
            },
        );
    }
}
pub fn show(ctx: &egui::Context, open: &mut bool, preferences: &mut UiSettings) -> bool {
    if !*open {
        return false;
    }
    let before = *preferences;
    let locale = i18n::locale(ctx);
    let width = (ctx.content_rect().width() - 80.0).clamp(200.0, 520.0);
    egui::Window::new(
        RichText::new(locale.text("设置", "Settings"))
            .size(22.0)
            .strong(),
    )
    .id(egui::Id::new("app-settings-window"))
    .open(open)
    .collapsible(false)
    .resizable(false)
    .frame(dialogs::frame(ctx))
    .auto_sized()
    .default_width(width + 50.0)
    .min_width(width + 50.0)
    .max_width(width + 50.0)
    .max_height((ctx.content_rect().height() - 32.0).max(80.0))
    .vscroll(true)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .show(ctx, |ui| {
        dialogs::body(ui);
        ui.set_width(width);
        ui.spacing_mut().item_spacing.y = dialogs::SECTION_GAP;
        section(ui, locale.text("外观", "Appearance"), |ui| {
            row(
                ui,
                locale.text("界面语言", "Interface language"),
                |ui, width| {
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
                        })
                        .response
                },
            );
            row(
                ui,
                locale.text("终端字体", "Terminal font"),
                |ui, width| {
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
                                        format!(
                                            "{} {}",
                                            choice.family(),
                                            locale.text("（未安装）", "(Not installed)")
                                        )
                                    };
                                    ui.selectable_value(
                                        &mut preferences.terminal_font,
                                        choice,
                                        label,
                                    );
                                });
                            }
                        })
                        .response
                },
            );
            if !appearance::font_available(preferences.terminal_font) {
                ui.label(
                    RichText::new(locale.text(
                        "所选字体未安装，当前使用默认字体。",
                        "The selected font is not installed. Using the default font.",
                    ))
                    .size(14.0),
                );
            }
        });
        section(ui, locale.text("浮窗", "Floating bar"), |ui| {
            row(
                ui,
                locale.text("置顶状态浮窗", "Always-on-top status bar"),
                |ui, _| {
                    ui.checkbox(
                        &mut preferences.floating_enabled,
                        locale.text("启用", "Enabled"),
                    )
                },
            );
            row(
                ui,
                locale.text("浮窗缩放", "Floating bar scale"),
                |ui, width| {
                    ui.spacing_mut().slider_width = (width - 80.0).max(60.0);
                    ui.add_enabled(
                        preferences.floating_enabled,
                        egui::Slider::new(
                            &mut preferences.floating_scale,
                            UiSettings::MIN_ZOOM..=UiSettings::MAX_ZOOM,
                        )
                        .step_by(0.01)
                        .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 0.18 })
                        .custom_formatter(|value, _| format!("{:.0}%", value * 100.0))
                        .custom_parser(|text| {
                            text.trim()
                                .trim_end_matches('%')
                                .trim()
                                .parse::<f64>()
                                .ok()
                                .filter(|n| n.is_finite())
                                .map(|n| n / 100.0)
                        }),
                    )
                },
            );
            ui.label(
                RichText::new(locale.text(
                    "与主窗口独立 · Ctrl + 滚轮调整",
                    "Independent of the main window · Ctrl+wheel",
                ))
                .size(14.0)
                .color(Color32::from_gray(178)),
            );
        });
    });
    before != *preferences
}
