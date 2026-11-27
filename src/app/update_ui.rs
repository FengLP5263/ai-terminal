use ai_terminal::{appearance, chrome, i18n::tr, updater::Updater};
use egui::{FontFamily, FontId, RichText};
use std::time::Duration;

pub fn button(ui: &mut egui::Ui, updater: &mut Updater) {
    let Some(offer) = &updater.offer else { return };
    let ctx = ui.ctx().clone();
    let label = tr(&ctx, "有新版本可用", "Update available");
    let response = ui.add_sized(
        [chrome::CONTROL, chrome::CONTROL],
        egui::Button::new("").frame(false).selected(updater.open),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let center = response.rect.center();
    if response.hovered() || updater.open {
        ui.painter().rect_filled(response.rect, 6, chrome::HOVER);
    }
    ui.painter().circle_filled(center, 9.0, chrome::COMPLETED);
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "\u{eaa1}",
        FontId::new(13.0, FontFamily::Name("codicons".into())),
        appearance::TERMINAL_BG,
    );
    if response.clicked() {
        updater.open = !updater.open;
    }
    if !updater.open {
        response.clone().on_hover_text(label);
    }
    let progress = updater.progress();
    let width = (ctx.content_rect().width() - 48.0).clamp(210.0, 332.0);
    let confirmed = egui::Popup::from_response(&response)
        .open_bool(&mut updater.open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .align(egui::RectAlign::BOTTOM_END)
        .gap(6.0)
        .width(width)
        .frame(egui::Frame::popup(ui.style()).inner_margin(16).corner_radius(9)
            .fill(ui.visuals().window_fill()).stroke(egui::Stroke::new(1.0, chrome::BORDER)))
        .show(|ui| {
            ui.set_width(width);
            let mut body = egui::TextStyle::Body.resolve(ui.style());
            body.family = FontFamily::Monospace;
            ui.style_mut().override_font_id = Some(body);
            let title = match progress {
                Some(100) => tr(&ctx, "正在校验更新", "Verifying update"),
                Some(_) => tr(&ctx, "正在下载更新", "Downloading update"),
                None => tr(&ctx, "发现新版本", "Update available"),
            };
            ui.label(RichText::new(title).monospace().size(16.0).strong());
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{} {}", tr(&ctx, "当前", "Current"), updater.current))
                    .color(chrome::SECONDARY));
                ui.label(RichText::new("→").color(chrome::SECONDARY));
                ui.label(RichText::new(format!("{} {}", tr(&ctx, "新版", "New"), offer.version)).strong());
            });
            if let Some(progress) = progress {
                ui.add(egui::ProgressBar::new(progress as f32 / 100.0)
                    .fill(chrome::WORKING).desired_height(6.0));
                ctx.request_repaint_after(Duration::from_millis(120));
                return false;
            }
            let notes = offer.notes(ai_terminal::i18n::locale(&ctx));
            if !notes.is_empty() {
                ui.add_space(4.0);
                ui.label(notes);
            }
            ui.separator();
            ui.label(RichText::new(tr(&ctx,
                "更新后自动重启。运行中的终端会结束，会话和对话历史会保留。",
                "AI Terminal restarts and ends running terminal processes. Saved sessions and Agent conversation history remain available.",
            )).monospace().size(13.0).color(chrome::SECONDARY));
            if let Some(error) = &updater.error {
                ui.colored_label(chrome::FAILED, tr(&ctx, "更新失败，可以重试。", "Update failed. You can retry."))
                    .on_hover_text(error);
            }
            ui.add_space(4.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let confirm = ui.add(egui::Button::new(RichText::new(tr(&ctx, "更新并重启", "Update and restart")).strong())
                    .fill(chrome::WORKING.gamma_multiply(0.27))).clicked();
                if ui.add(egui::Button::new(tr(&ctx, "稍后", "Later")).frame(false)).clicked() {
                    ui.close();
                }
                confirm
            }).inner
        })
        .is_some_and(|popup| popup.inner);
    if confirmed {
        updater.start(&ctx);
    }
}
