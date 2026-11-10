use super::{Actions, GitView, chrome, date, display_text, header_font, i18n};
use crate::git::{Commit, CommitDetails, Snapshot};
use egui::{Color32, FontId, Id, RichText, Sense, Stroke};

pub(super) fn show(
    ui: &mut egui::Ui,
    view: &mut GitView,
    snapshot: &Snapshot,
    actions: &mut Actions,
) {
    if view
        .selected
        .as_ref()
        .is_some_and(|id| !snapshot.commits.iter().any(|c| &c.id == id))
    {
        view.selected = None;
        view.expanded_details = None;
    }
    let row_height = ui.text_style_height(&egui::TextStyle::Body).max(18.0) + 10.0;
    let spacing = ui.spacing().item_spacing.y;
    let count = snapshot.commits.len();
    for (index, commit) in snapshot.commits.iter().enumerate() {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), row_height), Sense::hover());
        let response = ui.interact(
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - 28.0, rect.bottom())),
            Id::new(("git-commit-row", &commit.id)),
            Sense::click(),
        );
        let expanded = view.selected.as_deref() == Some(commit.id.as_str());
        let center = egui::pos2(rect.left() + 8.0, rect.center().y);
        if expanded || response.hovered() {
            ui.painter().rect_filled(rect, 3.0, Color32::from_gray(46));
        }
        let font = header_font(ui);
        let mut subject = RichText::new(display_text(&commit.subject, 400));
        if expanded {
            subject = subject.strong();
        }
        let galley = egui::WidgetText::from(subject).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (rect.width() - 55.0).max(1.0),
            font.clone(),
        );
        ui.painter().galley(
            egui::pos2(
                rect.left() + super::HISTORY_INDENT,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            chrome::TEXT,
        );
        let mut copy_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(rect.right() - 26.0, rect.top()),
                    rect.right_bottom(),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        copy_id(&mut copy_ui, commit, "row", &mut view.copied);
        if response.clicked() {
            view.selected = if expanded {
                None
            } else {
                Some(commit.id.clone())
            };
            view.expanded_details = None;
            if !expanded {
                actions.inspect = Some(commit.id.clone());
            }
        }
        // Tooltips remain interactive, so the pointer can reach the copy button.
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_ui(|ui| {
                ui.style_mut().override_font_id = Some(font);
                ui.set_max_width(420.0);
                let data = view
                    .inspection
                    .as_ref()
                    .filter(|i| i.id == commit.id)
                    .and_then(|i| i.data.as_deref())
                    .or_else(|| {
                        view.expanded_details
                            .as_deref()
                            .filter(|d| d.id == commit.id)
                    });
                tooltip(ui, commit, data, &mut view.copied);
                if data.is_none()
                    && (view.selected.is_none() || view.expanded_details.is_some())
                    && !view.inspection.as_ref().is_some_and(|i| i.id == commit.id)
                {
                    actions.inspect = Some(commit.id.clone());
                }
            });
        let expanded = view.selected.as_deref() == Some(commit.id.as_str());
        if expanded {
            ui.scope(|ui| {
                ui.spacing_mut().indent = super::HISTORY_INDENT;
                ui.visuals_mut().indent_has_left_vline = false;
                ui.indent(("commit-files", &commit.id), |ui| {
                    let data = view
                        .expanded_details
                        .as_deref()
                        .filter(|d| d.id == commit.id)
                        .or_else(|| {
                            view.inspection
                                .as_ref()
                                .filter(|i| i.id == commit.id)
                                .and_then(|i| i.data.as_deref())
                        });
                    if let Some(data) = data {
                        files(ui, data, actions, view.open_file.as_ref());
                    } else if let Some(error) = view
                        .inspection
                        .as_ref()
                        .filter(|i| i.id == commit.id)
                        .and_then(|i| i.error.as_ref())
                    {
                        ui.colored_label(
                            chrome::WAITING,
                            i18n::tr(ui.ctx(), "读取提交文件失败", "Could not read commit files"),
                        )
                        .on_hover_text(format!("{error:?}"));
                        if ui.button(i18n::tr(ui.ctx(), "重试", "Retry")).clicked() {
                            actions.inspect = Some(commit.id.clone());
                        }
                    } else {
                        ui.weak(i18n::tr(
                            ui.ctx(),
                            "正在读取提交文件…",
                            "Reading commit files…",
                        ));
                        if !view.inspection.as_ref().is_some_and(|i| i.id == commit.id) {
                            actions.inspect = Some(commit.id.clone());
                        }
                    }
                });
            });
        }
        let bottom = ui.min_rect().bottom();
        let start = if index == 0 {
            center.y
        } else {
            rect.top() - spacing
        };
        let end = if index + 1 == count {
            if expanded { bottom } else { center.y }
        } else {
            bottom + spacing
        };
        ui.painter().line_segment(
            [egui::pos2(center.x, start), egui::pos2(center.x, end)],
            Stroke::new(1.5, chrome::COMPLETED),
        );
        if expanded {
            ui.painter().circle_filled(center, 6.0, chrome::COMPLETED);
            ui.painter()
                .circle_filled(center, 3.5, Color32::from_gray(46));
        } else {
            ui.painter().circle_filled(center, 4.0, chrome::COMPLETED);
        }
    }
}

fn copy_id(ui: &mut egui::Ui, commit: &Commit, location: &str, copied: &mut Option<String>) {
    let locale = i18n::locale(ui.ctx());
    let id = Id::new(("git-copy-commit", &commit.id, location));
    let response = ui
        .push_id(id, |ui| {
            ui.add(
                egui::Button::new(RichText::new("\u{ebcc}").font(FontId::new(
                    header_font(ui).size,
                    egui::FontFamily::Name("codicons".into()),
                )))
                .frame(false),
            )
        })
        .inner;
    if response
        .on_hover_text(locale.text("复制完整提交 ID", "Copy full commit ID"))
        .clicked()
    {
        ui.ctx().copy_text(commit.id.clone());
        *copied = Some(commit.id.clone());
    }
}
fn tooltip(
    ui: &mut egui::Ui,
    commit: &Commit,
    data: Option<&CommitDetails>,
    copied: &mut Option<String>,
) {
    let locale = i18n::locale(ui.ctx());
    ui.label(RichText::new(display_text(&commit.author, 150)).color(chrome::COMPLETED));
    let seconds = commit.timestamp.rem_euclid(86400);
    ui.weak(format!(
        "{} {:02}:{:02}:{:02} UTC",
        date(commit.timestamp),
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    ));
    egui::ScrollArea::vertical()
        .id_salt(("commit-message", &commit.id))
        .max_height(220.0)
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(message_text(
                    data.map_or(commit.subject.as_str(), |d| d.message.as_str()),
                ))
                .wrap(),
            );
        });
    if let Some(data) = data {
        totals(ui, data);
    }
    if commit.parents.len() > 1 {
        ui.weak(locale.text(
            "合并提交 · 更改相对第一父提交",
            "Merge commit · Changes against first parent",
        ));
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("\u{eafc}")
                .font(FontId::new(
                    header_font(ui).size,
                    egui::FontFamily::Name("codicons".into()),
                ))
                .color(chrome::COMPLETED),
        );
        let short_id: String = commit.id.chars().take(7).collect();
        ui.label(RichText::new(short_id).color(chrome::COMPLETED));
        copy_id(ui, commit, "tooltip", copied);
        if copied.as_deref() == Some(commit.id.as_str()) {
            ui.weak(locale.text("已复制", "Copied"));
        }
    });
}
fn message_text(message: &str) -> String {
    let mut chars = message.chars();
    let mut text: String = chars
        .by_ref()
        .take(8000)
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();
    if chars.next().is_some() {
        text.push('…');
    }
    text
}
fn totals(ui: &mut egui::Ui, data: &CommitDetails) {
    let locale = i18n::locale(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.weak(format!(
            "{} {}",
            data.total_files,
            locale.text("个文件", "files")
        ));
        ui.colored_label(chrome::COMPLETED, format!("+{}", data.added));
        ui.colored_label(chrome::FAILED, format!("−{}", data.deleted));
    });
}
fn files(
    ui: &mut egui::Ui,
    data: &CommitDetails,
    actions: &mut Actions,
    open_file: Option<&(crate::git::PatchSource, String)>,
) {
    let locale = i18n::locale(ui.ctx());
    totals(ui, data);
    if data.parents.len() > 1 {
        ui.weak(locale.text("相对第一父提交", "Against first parent"));
    }
    if data.files.is_empty() {
        ui.weak(locale.text("此提交没有文件更改", "No file changes in this commit"));
    }
    for file in &data.files {
        let response = super::files::show(
            ui,
            super::files::FileRow {
                path: &file.path,
                status: &file.status.to_string(),
                added: file.added,
                deleted: file.deleted,
                binary: file.binary,
                omitted: false,
                selected: open_file.is_some_and(|(source, path)| {
                    matches!(source, crate::git::PatchSource::Commit(id) if id == &data.id)
                        && path == &file.path
                }),
            },
            egui::Id::new(("git-commit-file", &data.id, &file.path)),
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            actions.file = Some((
                crate::git::PatchSource::Commit(data.id.clone()),
                file.path.clone(),
            ));
        }
    }
    if data.partial {
        ui.weak(locale.text(
            "仅展示前 500 个文件；行数为全部文件的合计。",
            "Showing the first 500 files; line totals cover all files.",
        ));
    }
}
