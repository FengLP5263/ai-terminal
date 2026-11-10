//! Shared project/Git workbench and a virtualized, lazy file tree.
use crate::{
    chrome, file_icons,
    git::GitClient,
    git_ui::{Actions, GitView},
    i18n,
    project_files::{Entry, EntryKind, FileClient, FileError, FileState},
};
use egui::{Color32, FontFamily, FontId, RichText};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub const PROJECT_FILES_ICON: &str = "\u{ea83}";

fn font(ui: &egui::Ui) -> FontId {
    let mut font = egui::TextStyle::Body.resolve(ui.style());
    font.family = FontFamily::Monospace;
    font
}
fn glyph_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("codicons".into()))
}
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, label: &str, selected: bool) -> egui::Response {
    ui.add_sized(
        [chrome::CONTROL, chrome::CONTROL],
        egui::Button::new(RichText::new(glyph).font(glyph_font(font(ui).size)))
            .selected(selected)
            .frame(false),
    )
    .on_hover_text(label)
}
fn tab(ui: &mut egui::Ui, glyph: &str, label: &str, selected: bool) -> egui::Response {
    let font = font(ui);
    let mut text = egui::text::LayoutJob::default();
    text.append(
        glyph,
        0.0,
        egui::TextFormat {
            font_id: glyph_font(font.size),
            color: chrome::TEXT,
            ..Default::default()
        },
    );
    text.append(
        &format!(" {label}"),
        0.0,
        egui::TextFormat {
            font_id: font,
            color: chrome::TEXT,
            ..Default::default()
        },
    );
    ui.selectable_label(selected, text)
}
pub fn error_message(error: &FileError, locale: i18n::Locale) -> String {
    match error {
        FileError::Io(message)=>format!("{}: {message}",locale.text("无法读取","Could not read")),
        FileError::Link=>locale.text("链接不会自动跟随，请在对应目录的会话中查看。","Links are not followed automatically. Browse the target directory in its own session.").into(),
        FileError::OutsideProject=>locale.text("此路径不在当前项目中。","This path is outside the current project.").into(),
        FileError::TooLarge=>locale.text("文件超过 2 MiB，暂不支持预览。","Files larger than 2 MiB cannot be previewed.").into(),
        FileError::Binary=>locale.text("此文件不支持文本预览。","Text preview is unavailable for this file.").into(),
        FileError::Encoding=>locale.text("无法准确解码此文件。支持 UTF-8 和带 BOM 的 UTF-16。","Cannot decode this file accurately. UTF-8 and UTF-16 with BOM are supported.").into(),
        FileError::Cancelled=>String::new(),
    }
}
#[derive(Clone)]
enum Row {
    Entry(Entry, usize),
    Note(PathBuf, usize, String),
}
#[derive(Default)]
pub struct FileView {
    root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    revision: Option<u64>,
    locale: Option<&'static str>,
    rows: Vec<Row>,
}
impl FileView {
    pub fn expand(&mut self, path: &Path, client: &mut FileClient) {
        self.expanded.insert(path.into());
        client.ensure_directory(path);
        self.revision = None;
    }
    pub fn collapse_all(&mut self, client: &mut FileClient) {
        self.expanded.clear();
        client.collapse(Path::new(""));
        self.revision = None;
    }
    fn flatten(&mut self, path: &Path, depth: usize, client: &FileClient, locale: i18n::Locale) {
        if self.rows.len() >= 20_000 {
            return;
        }
        if depth > 32 {
            self.rows.push(Row::Note(
                path.into(),
                depth,
                locale
                    .text(
                        "目录层级过深，暂不展开。",
                        "Folder nesting is too deep to expand.",
                    )
                    .into(),
            ));
            return;
        }
        if let Some(state) = client.directories.get(path) {
            if let Some(data) = &state.data {
                if data.entries.is_empty() && !data.partial {
                    self.rows.push(Row::Note(
                        path.into(),
                        depth,
                        locale.text("空文件夹", "Empty folder").into(),
                    ));
                }
                for entry in &data.entries {
                    if self.rows.len() >= 20_000 {
                        break;
                    }
                    self.rows.push(Row::Entry(entry.clone(), depth));
                    if entry.kind == EntryKind::Directory && self.expanded.contains(&entry.path) {
                        self.flatten(&entry.path, depth + 1, client, locale);
                    }
                }
                if data.partial {
                    self.rows.push(Row::Note(
                        path.into(),
                        depth,
                        locale
                            .text(
                                "目录过大，仅显示部分项目。",
                                "Large directory; some entries are omitted.",
                            )
                            .into(),
                    ));
                }
            } else if let Some(error) = &state.error {
                self.rows
                    .push(Row::Note(path.into(), depth, error_message(error, locale)));
            } else if state.loading {
                self.rows.push(Row::Note(
                    path.into(),
                    depth,
                    locale.text("读取中…", "Reading…").into(),
                ));
            }
        }
    }
    pub fn contents(&mut self, ui: &mut egui::Ui, client: &mut FileClient) -> Option<PathBuf> {
        let locale = i18n::locale(ui.ctx());
        if self.root != client.root {
            self.root = client.root.clone();
            self.expanded.clear();
            self.revision = None;
        }
        if client.root.is_none() {
            ui.weak(locale.text(
                "当前会话没有工作目录。",
                "This session has no working directory.",
            ));
            return None;
        }
        client.ensure_directory(Path::new(""));
        let locale_key = locale.text("zh", "en");
        if self.revision != Some(client.revision) || self.locale != Some(locale_key) {
            self.locale = Some(locale_key);
            self.rows.clear();
            self.flatten(Path::new(""), 0, client, locale);
            self.revision = Some(client.revision);
        }
        let font = font(ui);
        ui.style_mut().override_font_id = Some(font.clone());
        let height = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
        let selected = client.file.as_ref().map(|f| f.path.clone());
        let mut toggle = None;
        let mut open = None;
        egui::ScrollArea::vertical()
            .id_salt(("project-tree", &self.root))
            .auto_shrink([false, false])
            .show_rows(ui, height, self.rows.len(), |ui, range| {
                for index in range {
                    match &self.rows[index] {
                        Row::Note(path, depth, note) => {
                            let response = ui.add_sized(
                                [ui.available_width(), height],
                                egui::Label::new(RichText::new(note).color(chrome::SECONDARY))
                                    .truncate(),
                            );
                            response.on_hover_text(format!("{}\n{}", path.display(), note));
                            let _ = depth;
                        }
                        Row::Entry(entry, depth) => {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), height),
                                egui::Sense::click(),
                            );
                            let response = response.on_hover_text(format!(
                                "{}{}",
                                client.root.as_ref().unwrap().join(&entry.path).display(),
                                if entry.kind == EntryKind::Link {
                                    locale.text(
                                        "\n链接（不自动跟随）",
                                        "\nLink (not followed automatically)",
                                    )
                                } else {
                                    ""
                                }
                            ));
                            let active = selected.as_deref() == Some(entry.path.as_path());
                            if active || response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    3,
                                    if active {
                                        chrome::SELECTED
                                    } else {
                                        chrome::HOVER
                                    },
                                );
                            }
                            if active {
                                ui.painter().vline(
                                    rect.left(),
                                    rect.y_range(),
                                    egui::Stroke::new(2.0, chrome::WORKING),
                                );
                            }
                            let mut left = rect.left() + *depth as f32 * 16.0 + 4.0;
                            let center = rect.center().y;
                            if entry.kind == EntryKind::Directory {
                                ui.painter().text(
                                    egui::pos2(left, center),
                                    egui::Align2::LEFT_CENTER,
                                    if self.expanded.contains(&entry.path) {
                                        "\u{eab4}"
                                    } else {
                                        "\u{eab6}"
                                    },
                                    glyph_font(font.size),
                                    chrome::SECONDARY,
                                );
                            }
                            left += font.size + 4.0;
                            let (glyph, color) = match entry.kind {
                                EntryKind::Directory => (
                                    if self.expanded.contains(&entry.path) {
                                        "\u{eaf7}"
                                    } else {
                                        "\u{ea83}"
                                    },
                                    chrome::WORKING,
                                ),
                                EntryKind::Link => ("\u{eaee}", chrome::SECONDARY),
                                _ => file_icons::icon(&entry.name, false),
                            };
                            ui.painter().text(
                                egui::pos2(left, center),
                                egui::Align2::LEFT_CENTER,
                                glyph,
                                glyph_font(font.size),
                                color,
                            );
                            left += font.size + 7.0;
                            let bounds =
                                egui::Rect::from_min_max(egui::pos2(left, rect.top()), rect.max);
                            let mut text_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .id_salt(("tree-file", &entry.path))
                                    .max_rect(bounds)
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            );
                            text_ui.set_clip_rect(bounds.intersect(ui.clip_rect()));
                            crate::ui_text::label_with_font(
                                &mut text_ui,
                                &entry.name,
                                font.clone(),
                                chrome::TEXT,
                                bounds.width().max(0.0),
                            );
                            if response.clicked() {
                                if entry.kind == EntryKind::Directory {
                                    toggle = Some(entry.path.clone());
                                } else {
                                    open = Some(entry.path.clone());
                                }
                            }
                        }
                    }
                }
            });
        if let Some(path) = toggle {
            if self.expanded.remove(&path) {
                self.expanded.retain(|p| !p.starts_with(&path));
                client.collapse(&path);
            } else {
                self.expand(&path, client);
            }
            self.revision = None;
        }
        if self.rows.len() >= 20_000 {
            ui.weak(locale.text(
                "展开内容过多，请收起部分文件夹。",
                "Too many expanded entries; collapse some folders.",
            ));
        }
        if client.limit_reached {
            ui.weak(locale.text(
                "请收起一些文件夹后继续展开。",
                "Collapse some folders before expanding more.",
            ));
        }
        open
    }
}

#[derive(Default)]
pub struct PanelActions {
    pub git: Actions,
    pub file: Option<PathBuf>,
}
#[allow(clippy::too_many_arguments)]
pub fn panel(
    ui: &mut egui::Ui,
    client: &mut FileClient,
    view: &mut FileView,
    git: Option<&GitClient>,
    git_view: &mut GitView,
    git_open: &mut bool,
    files_open: &mut bool,
) -> PanelActions {
    if !*git_open && !*files_open {
        return PanelActions::default();
    }
    let mut result = PanelActions::default();
    let maximum = (ui.available_width() * 0.55).max(80.0);
    egui::Panel::right("repository-inspector")
        .resizable(true)
        .default_size(340.0)
        .min_size(240.0_f32.min(maximum))
        .max_size(maximum)
        .frame(
            egui::Frame::new()
                .fill(Color32::from_gray(22))
                .inner_margin(12),
        )
        .show(ui, |ui| {
            ui.style_mut().override_font_id = Some(font(ui));
            let locale = i18n::locale(ui.ctx());
            ui.horizontal(|ui| {
                if tab(
                    ui,
                    PROJECT_FILES_ICON,
                    locale.text("文件", "Files"),
                    *files_open,
                )
                .clicked()
                {
                    *files_open = true;
                    *git_open = false;
                }
                if tab(ui, "\u{ec6f}", "Git", *git_open).clicked() {
                    *git_open = true;
                    *files_open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(
                        ui,
                        "\u{ea76}",
                        locale.text("收起面板", "Close panel"),
                        false,
                    )
                    .clicked()
                    {
                        *git_open = false;
                        *files_open = false;
                    }
                    if icon_button(ui, "\u{eb37}", locale.text("刷新", "Refresh"), false).clicked()
                    {
                        if *files_open {
                            client.refresh();
                        } else {
                            result.git.refresh = true;
                        }
                    }
                });
            });
            if !*files_open && !*git_open {
                return;
            }
            ui.horizontal(|ui| {
                let root = if *files_open {
                    client.root.as_deref()
                } else {
                    git.and_then(|g| g.snapshot.as_ref())
                        .map(|s| s.root.as_path())
                        .or(client.root.as_deref())
                };
                if let Some(root) = root {
                    let width = (ui.available_width()
                        - if *files_open {
                            chrome::CONTROL + 8.0
                        } else {
                            0.0
                        })
                    .max(1.0);
                    ui.add_sized(
                        [width, chrome::CONTROL],
                        egui::Label::new(
                            RichText::new(
                                root.file_name()
                                    .unwrap_or(root.as_os_str())
                                    .to_string_lossy(),
                            )
                            .strong(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(root.display().to_string());
                }
                if *files_open
                    && icon_button(
                        ui,
                        "\u{eac5}",
                        locale.text("全部收起", "Collapse all folders"),
                        false,
                    )
                    .clicked()
                {
                    view.collapse_all(client);
                }
            });
            ui.separator();
            if *files_open {
                result.file = view.contents(ui, client);
            } else {
                if git.is_some_and(|g| g.snapshot.is_some() && !g.watching) {
                    ui.weak(locale.text(
                        "目录监控不可用，可手动刷新。",
                        "Directory monitoring is unavailable; refresh manually.",
                    ));
                }
                let actions = git_view.contents_without_header(ui, git);
                result.git.refresh |= actions.refresh;
                result.git.more = actions.more;
                result.git.inspect = actions.inspect;
                result.git.file = actions.file;
            }
        });
    result
}

/// Returns true only when the user closes the document; rendering never writes a project file.
pub fn document(ui: &mut egui::Ui, state: &FileState, root: Option<&Path>) -> bool {
    let locale = i18n::locale(ui.ctx());
    let font = font(ui);
    ui.style_mut().override_font_id = Some(font.clone());
    let mut close = false;
    ui.horizontal(|ui| {
        let (glyph, color) = file_icons::icon(&state.path.to_string_lossy(), false);
        ui.label(
            RichText::new(glyph)
                .font(glyph_font(font.size))
                .color(color),
        );
        let width = (ui.available_width() - chrome::CONTROL - 100.0).max(1.0);
        ui.add_sized(
            [width, chrome::CONTROL],
            egui::Label::new(
                RichText::new(state.path.to_string_lossy().replace('\\', "/")).strong(),
            )
            .truncate(),
        )
        .on_hover_text(
            root.map(|r| r.join(&state.path))
                .unwrap_or_else(|| state.path.clone())
                .display()
                .to_string(),
        );
        if state.loading {
            ui.label("…");
        }
        ui.weak(locale.text("只读", "Read only"));
        close = icon_button(
            ui,
            "\u{ea76}",
            locale.text("返回终端", "Return to terminal"),
            false,
        )
        .clicked();
    });
    ui.separator();
    if let Some(error) = &state.error {
        ui.colored_label(chrome::WAITING, error_message(error, locale));
    }
    if let Some(data) = &state.data {
        ui.weak(format!(
            "{} · {} {}",
            data.encoding,
            data.content.lines.len(),
            locale.text("行", "lines")
        ));
        if data.content.partial {
            ui.weak(locale.text("显示上限：20,000 行、每行 4,000 个字符；超出部分已省略。","Display limits: 20,000 lines and 4,000 characters per line; excess content is omitted."));
        }
        if data.content.lines.is_empty() {
            ui.weak(locale.text("文件内容为空。", "File is empty."));
        }
        crate::git_ui::diff::body(
            ui,
            &data.content,
            ("project-document", root, &state.path),
            true,
        );
    }
    close
}
