//! Shared compact file rows for working-tree changes and expanded commits.
use super::{chrome, display_text, header_font, i18n};
use crate::file_icons::icon;
use egui::{Color32, RichText, Sense};

pub(super) struct FileRow<'a> {
    pub path: &'a str,
    pub status: &'a str,
    pub added: u64,
    pub deleted: u64,
    pub binary: bool,
    pub omitted: bool,
    pub selected: bool,
}

pub(super) fn show(ui: &mut egui::Ui, file: FileRow<'_>, id: egui::Id) -> egui::Response {
    let font = header_font(ui);
    let height = ui.text_style_height(&egui::TextStyle::Body).max(18.0) + 4.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    if file.selected {
        let selected = rect.expand2(egui::vec2(4.0, 0.0));
        ui.painter().rect_filled(selected, 2.0, chrome::SELECTED);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                selected.left_top() + egui::vec2(0.0, 2.0),
                egui::vec2(2.0, (rect.height() - 4.0).max(0.0)),
            ),
            1.0,
            chrome::WORKING,
        );
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 2.0, Color32::from_gray(40));
    }
    let symbol = icon(file.path, file.binary);
    symbol.paint(
        ui,
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + font.size / 2.0, rect.center().y),
            egui::Vec2::splat(font.size),
        ),
    );
    let status = if file.status == "??" {
        "U"
    } else {
        file.status.trim()
    };
    let status_color = if file.status.contains('D') {
        chrome::FAILED
    } else if file.status.contains(['A', '?']) {
        chrome::COMPLETED
    } else {
        chrome::WAITING
    };
    let status_galley = ui
        .painter()
        .layout_no_wrap(status.into(), font.clone(), status_color);
    let left = rect.left() + font.size + 8.0;
    let width = (rect.right() - left - status_galley.size().x - 10.0).max(1.0);
    let path = egui::WidgetText::from(RichText::new(display_text(file.path, 1000))).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width,
        font,
    );
    let status_x = (left + path.size().x + 10.0).min(rect.right() - status_galley.size().x);
    ui.painter().galley(
        egui::pos2(left, rect.center().y - path.size().y * 0.5),
        path,
        chrome::TEXT,
    );
    ui.painter().galley(
        egui::pos2(status_x, rect.center().y - status_galley.size().y * 0.5),
        status_galley,
        status_color,
    );
    response.on_hover_ui(|ui| {
        ui.style_mut().override_font_id = Some(header_font(ui));
        ui.set_max_width(420.0);
        ui.label(display_text(file.path, 4000));
        let locale = i18n::locale(ui.ctx());
        ui.weak(format!(
            "{}: {}",
            locale.text("状态", "Status"),
            status_description(file.status, locale)
        ));
        if file.binary {
            ui.weak(locale.text("二进制", "Binary"));
        } else if file.omitted {
            ui.weak(locale.text("未统计行数", "Line counts unavailable"));
        } else {
            ui.horizontal(|ui| {
                ui.colored_label(chrome::COMPLETED, format!("+{}", file.added));
                ui.colored_label(chrome::FAILED, format!("−{}", file.deleted));
            });
        }
    })
}

fn status_description(status: &str, locale: i18n::Locale) -> String {
    if status == "??" {
        return locale
            .text("未跟踪（尚未加入 Git）", "Untracked (not added to Git)")
            .into();
    }
    if status.contains('U') || matches!(status, "AA" | "DD") {
        return locale.text("合并冲突", "Merge conflict").into();
    }
    let name = |code| match code {
        'M' => locale.text("已修改", "Modified"),
        'A' => locale.text("新增", "Added"),
        'D' => locale.text("已删除", "Deleted"),
        'R' => locale.text("已重命名", "Renamed"),
        'C' => locale.text("已复制", "Copied"),
        'T' => locale.text("类型变化", "Type changed"),
        _ => locale.text("未知状态", "Unknown status"),
    };
    let codes: Vec<_> = status.chars().collect();
    if codes.len() == 2 {
        [
            (codes[0], locale.text("已暂存", "Staged")),
            (codes[1], locale.text("未暂存", "Unstaged")),
        ]
        .into_iter()
        .filter(|(code, _)| *code != ' ')
        .map(|(code, group)| format!("{group}: {}", name(code)))
        .collect::<Vec<_>>()
        .join(" · ")
    } else {
        codes
            .first()
            .map_or_else(String::new, |code| name(*code).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn statuses_explain_untracked_staging_and_conflicts_in_both_languages() {
        assert_eq!(
            status_description("??", i18n::Locale::ZhCn),
            "未跟踪（尚未加入 Git）"
        );
        assert_eq!(
            status_description("??", i18n::Locale::En),
            "Untracked (not added to Git)"
        );
        assert_eq!(
            status_description("MM", i18n::Locale::En),
            "Staged: Modified · Unstaged: Modified"
        );
        assert_eq!(
            status_description(" M", i18n::Locale::ZhCn),
            "未暂存: 已修改"
        );
        assert_eq!(status_description("A ", i18n::Locale::En), "Staged: Added");
        assert_eq!(status_description("UU", i18n::Locale::ZhCn), "合并冲突");
        assert_eq!(status_description("D", i18n::Locale::En), "Deleted");
    }
}
