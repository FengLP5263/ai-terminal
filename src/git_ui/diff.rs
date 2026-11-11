//! Virtualized, read-only unified diff using the application's existing colors and font.
use super::{chrome, display_text, header_font, i18n};
use crate::git::{PatchKind, PatchResult};
use egui::{Color32, Id, RichText};

/// Returns true when the user closes the file and returns to the live terminal.
pub fn show(ui: &mut egui::Ui, result: &PatchResult) -> bool {
    let locale = i18n::locale(ui.ctx());
    let font = header_font(ui);
    ui.style_mut().override_font_id = Some(font.clone());
    let close;
    let (row, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), chrome::CONTROL),
        egui::Sense::hover(),
    );
    {
        let mut header = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("git-diff-header")
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let ui = &mut header;
        close = chrome::icon_button(
            ui,
            Id::new("git-diff-close"),
            chrome::ActionIcon::Close,
            locale.text("返回终端", "Return to terminal"),
        )
        .clicked();
        ui.colored_label(chrome::SECONDARY, locale.text("只读", "Read only"));
        let (mut short, description) = match &result.source {
            crate::git::PatchSource::Commit(id) => {
                (id.chars().take(7).collect::<String>(), id.clone())
            }
            crate::git::PatchSource::WorkingTree => (
                locale.text("本地更改", "Local changes").into(),
                locale
                    .text(
                        "当前文件相对 HEAD 的净变化（包含已暂存与未暂存）",
                        "Current file against HEAD (staged and unstaged changes combined)",
                    )
                    .into(),
            ),
        };
        if result.loading {
            short.push_str(" …");
        }
        ui.colored_label(chrome::COMPLETED, short)
            .on_hover_text(description);
        let width = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, chrome::CONTROL),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_width(width);
                crate::file_icons::icon(
                    &result.path,
                    result.data.as_ref().is_some_and(|p| p.binary),
                )
                .show(ui, font.size);
                ui.add(
                    egui::Label::new(RichText::new(display_text(&result.path, 2000)).strong())
                        .truncate(),
                )
                .on_hover_ui(|ui| {
                    ui.label(&result.path);
                    if let Some(patch) = &result.data {
                        if !patch.binary {
                            ui.weak(
                                locale.text(
                                    "UTF-8 · 按提交原文显示",
                                    "UTF-8 · Original committed text",
                                ),
                            );
                        }
                        for line in patch
                            .lines
                            .iter()
                            .filter(|l| l.kind == PatchKind::Note)
                            .take(6)
                        {
                            ui.weak(display_text(&line.text, 300));
                        }
                    }
                });
            },
        );
    }
    ui.separator();
    if result.loading && result.data.is_none() {
        ui.weak(locale.text("正在读取文件差异…", "Reading file diff…"));
    }
    if let Some(error) = &result.error {
        ui.colored_label(
            chrome::WAITING,
            if matches!(error, crate::git::GitError::InvalidUtf8) {
                locale.text(
                    "文件内容不是有效 UTF-8，无法准确显示。",
                    "File content is not valid UTF-8 and cannot be displayed accurately.",
                )
            } else {
                locale.text("无法读取文件差异", "Could not read file diff")
            },
        )
        .on_hover_text(format!("{error:?}"));
    }
    if let Some(patch) = &result.data {
        if patch.first_parent {
            ui.weak(locale.text("相对第一父提交", "Against first parent"));
        }
        if patch.binary {
            ui.weak(locale.text(
                "二进制文件已更改，无法显示文本差异。",
                "Binary file changed; text diff is unavailable.",
            ));
        }
        if patch.partial {
            ui.weak(locale.text(
                "显示上限：20,000 行、每行 4,000 个字符；超出部分已省略。",
                "Display limits: 20,000 lines and 4,000 characters per line; excess content is omitted.",
            ));
        }
        if patch.presentation.rows.is_empty() && !patch.binary {
            ui.weak(locale.text("文件内容为空。", "File is empty."));
        }
        body(
            ui,
            patch,
            ("git-file-diff", &result.source, &result.path),
            false,
        );
    }
    close
}

/// Shared source rendering for diffs and ordinary project documents; only visible rows are laid out.
pub fn body(
    ui: &mut egui::Ui,
    patch: &crate::git::FilePatch,
    salt: impl std::hash::Hash + std::fmt::Debug,
    single_numbers: bool,
) {
    let font = header_font(ui);
    ui.style_mut().override_font_id = Some(font.clone());
    let row_height = ui.text_style_height(&egui::TextStyle::Body).max(font.size) + 5.0;
    let char_width = ui
        .painter()
        .layout_no_wrap("0".into(), font.clone(), chrome::TEXT)
        .size()
        .x;
    let gutter = char_width * if single_numbers { 8.0 } else { 16.0 };
    let width = (gutter + patch.max_columns as f32 * char_width + 20.0).max(ui.available_width());
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both()
            .id_salt(salt)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, patch.presentation.rows.len(), |ui, rows| {
                for index in rows {
                    let row = &patch.presentation.rows[index];
                    let line = &patch.lines[row.source];
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(width, row_height), egui::Sense::hover());
                    let (color, marker) = match line.kind {
                        PatchKind::Added => (chrome::COMPLETED, "+"),
                        PatchKind::Deleted => (chrome::FAILED, "−"),
                        _ => (chrome::TEXT, ""),
                    };
                    if matches!(line.kind, PatchKind::Added | PatchKind::Deleted) {
                        ui.painter()
                            .rect_filled(rect, 0.0, color.gamma_multiply(0.13));
                    }
                    let numbers = if single_numbers {
                        format!("{:>6}", line.new.map(|n| n.to_string()).unwrap_or_default())
                    } else {
                        format!(
                            "{:>6} {:>6}",
                            line.old.map(|n| n.to_string()).unwrap_or_default(),
                            line.new.map(|n| n.to_string()).unwrap_or_default()
                        )
                    };
                    let origin = egui::pos2(rect.left(), rect.center().y - row_height * 0.5 + 2.5);
                    ui.painter().text(
                        origin,
                        egui::Align2::LEFT_TOP,
                        numbers,
                        font.clone(),
                        chrome::SECONDARY,
                    );
                    if !marker.is_empty() {
                        ui.painter().text(
                            origin + egui::vec2(char_width * 14.0, 0.0),
                            egui::Align2::LEFT_TOP,
                            marker,
                            font.clone(),
                            color,
                        );
                    }
                    let text_origin = origin + egui::vec2(gutter, 0.0);
                    for column in (0..row.indent).step_by(patch.presentation.indent_width.max(1)) {
                        let x = text_origin.x + column as f32 * char_width;
                        ui.painter().line_segment(
                            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                            egui::Stroke::new(
                                1.0 / ui.ctx().pixels_per_point(),
                                Color32::from_gray(53),
                            ),
                        );
                    }
                    let mut job = egui::text::LayoutJob::default();
                    let mut start = 0;
                    for token in &row.tokens {
                        let end = (token.end as usize).min(line.text.len());
                        if let Some(text) = line.text.get(start..end) {
                            job.append(
                                text,
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: Color32::from_rgb(
                                        token.color[0],
                                        token.color[1],
                                        token.color[2],
                                    ),
                                    ..Default::default()
                                },
                            );
                            start = end;
                        }
                    }
                    if start < line.text.len() || job.text.is_empty() {
                        job.append(
                            &line.text[start..],
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: chrome::TEXT,
                                ..Default::default()
                            },
                        );
                    }
                    let galley = crate::ui_text::layout_job(ui.ctx(), job);
                    ui.painter()
                        .galley(text_origin, galley.clone(), chrome::TEXT);
                    crate::ui_text::paint_emoji(
                        ui.painter(),
                        text_origin,
                        &galley,
                        font.size,
                        chrome::TEXT,
                    );
                }
            });
    });
}
