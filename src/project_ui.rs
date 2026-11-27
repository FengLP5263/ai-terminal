//! Shared project/Git workbench and a virtualized, lazy file tree.
use crate::{
    chrome, file_icons,
    git::GitClient,
    git_ui::{Actions, GitView},
    i18n,
    project_files::{Entry, EntryKind, FileClient, FileError},
};
use egui::{Color32, FontFamily, FontId, RichText};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

const SEARCH_INPUT_ID: &str = "project-file-search";

pub fn search_has_focus(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.has_focus(egui::Id::new(SEARCH_INPUT_ID)))
}

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
pub fn project_name(root: &Path) -> std::borrow::Cow<'_, str> {
    root.file_name()
        .unwrap_or(root.as_os_str())
        .to_string_lossy()
}
pub fn project_button(
    ui: &mut egui::Ui,
    root: &Path,
    width: f32,
    selected: bool,
) -> egui::Response {
    let font = font(ui);
    let name = project_name(root);
    let response = ui.add_sized(
        [width, chrome::CONTROL],
        egui::Button::new("").selected(selected).frame(false),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name.as_ref())
    });
    let painter = ui
        .painter()
        .with_clip_rect(response.rect.intersect(ui.clip_rect()));
    let color = ui
        .style()
        .interact_selectable(&response, selected)
        .text_color();
    let left = response.rect.left() + 6.0;
    painter.text(
        egui::pos2(left, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        PROJECT_FILES_ICON,
        glyph_font(font.size),
        color,
    );
    let left = left + font.size + 6.0;
    let galley = crate::ui_text::layout_font_rows(
        ui.ctx(),
        &name,
        font.clone(),
        color,
        (response.rect.right() - left - 6.0).max(1.0),
        1,
    );
    let pos = egui::pos2(left, response.rect.center().y - galley.size().y / 2.0);
    painter.galley(pos, galley.clone(), color);
    crate::ui_text::paint_emoji(&painter, pos, &galley, font.size, color);
    response.on_hover_ui(|ui| {
        crate::ui_text::multiline_label(
            ui,
            &format!(
                "{}\n{}",
                i18n::tr(ui.ctx(), "浏览项目文件", "Browse project files"),
                root.display()
            ),
        );
    })
}
fn tab(ui: &mut egui::Ui, glyph: &str, label: &str, selected: bool, width: f32) -> egui::Response {
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
    ui.add_sized(
        [width, chrome::CONTROL],
        egui::Button::selectable(selected, text).truncate(),
    )
    .on_hover_text(label)
}
pub fn error_message(error: &FileError, locale: i18n::Locale) -> String {
    match error {
        FileError::Io(message)=>format!("{}: {message}",locale.text("无法读取","Could not read")),
        FileError::Link=>locale.text("链接不会自动跟随，请在对应目录的会话中查看。","Links are not followed automatically. Browse the target directory in its own session.").into(),
        FileError::OutsideProject=>locale.text("此路径不在当前项目中。","This path is outside the current project.").into(),
        FileError::TooLarge=>locale.text("图片超过 32 MiB，无法预览。","Images larger than 32 MiB cannot be previewed.").into(),
        FileError::ImageLimit=>locale.text("图片超出预览的尺寸或资源上限。","Image exceeds preview dimensions or resource limits.").into(),
        FileError::Image(message)=>format!("{}: {message}",locale.text("无法解码图片","Cannot decode image")),
        FileError::Unsupported=>locale.text("此文档格式暂不支持预览。","Preview is unavailable for this document format.").into(),
        FileError::Changed=>locale.text("文件已更改，请重新加载后查看。","File has changed. Reload to continue viewing.").into(),
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
    search_text: String,
    search_scroll_reset: bool,
    preview: crate::project_preview::View,
}
impl FileView {
    pub fn preview_mut(&mut self) -> &mut crate::project_preview::View {
        &mut self.preview
    }
    pub fn collapse(&mut self, path: &Path, client: &mut FileClient) {
        self.expanded.retain(|p| !p.starts_with(path));
        client.collapse(path);
        self.revision = None;
    }
    pub fn expand(&mut self, path: &Path, client: &mut FileClient) {
        if self.root != client.root {
            self.root = client.root.clone();
            self.expanded.clear();
            self.search_text.clear();
        }
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
            self.search_text.clear();
        }
        if client.root.is_none() {
            ui.weak(locale.text(
                "当前会话没有工作目录。",
                "This session has no working directory.",
            ));
            return None;
        }
        self.search_field(ui, client, locale);
        if let Some(search) = client.search_state() {
            let reset_scroll =
                search.result.is_some() && std::mem::take(&mut self.search_scroll_reset);
            return search_results(ui, search, client, locale, reset_scroll);
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
                                "{}{}{}",
                                client.root.as_ref().unwrap().join(&entry.path).display(),
                                if entry.kind == EntryKind::Link {
                                    locale.text(
                                        "\n链接（不自动跟随）",
                                        "\nLink (not followed automatically)",
                                    )
                                } else {
                                    ""
                                },
                                if entry.ignored {
                                    locale.text("\n已被 Git 忽略", "\nIgnored by Git")
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
                            match entry.kind {
                                EntryKind::Directory | EntryKind::Link => {
                                    let glyph = if entry.kind == EntryKind::Link {
                                        "\u{eaee}"
                                    } else if self.expanded.contains(&entry.path) {
                                        "\u{eaf7}"
                                    } else {
                                        "\u{ea83}"
                                    };
                                    ui.painter().text(
                                        egui::pos2(left, center),
                                        egui::Align2::LEFT_CENTER,
                                        glyph,
                                        glyph_font(font.size),
                                        if entry.ignored || entry.kind == EntryKind::Link {
                                            chrome::SECONDARY
                                        } else {
                                            chrome::WORKING
                                        },
                                    );
                                }
                                _ => {
                                    let mut icon = file_icons::icon(&entry.name, false);
                                    if entry.ignored {
                                        icon.color = chrome::SECONDARY;
                                    }
                                    icon.paint(
                                        ui,
                                        egui::Rect::from_center_size(
                                            egui::pos2(left + font.size / 2.0, center),
                                            egui::Vec2::splat(font.size),
                                        ),
                                    );
                                }
                            }
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
                                if entry.ignored {
                                    chrome::SECONDARY
                                } else {
                                    chrome::TEXT
                                },
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

    fn search_field(&mut self, ui: &mut egui::Ui, client: &mut FileClient, locale: i18n::Locale) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("\u{ea6d}")
                    .font(glyph_font(font(ui).size))
                    .color(chrome::SECONDARY),
            );
            let clear_width = if self.search_text.is_empty() {
                0.0
            } else {
                24.0 + ui.spacing().item_spacing.x
            };
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.search_text)
                    .id(egui::Id::new(SEARCH_INPUT_ID))
                    .hint_text(locale.text("搜索文件名或路径", "Find files by name or path"))
                    .desired_width((ui.available_width() - clear_width).max(0.0))
                    .char_limit(1024)
                    .margin(egui::vec2(7.0, 5.0)),
            );
            let clear = !self.search_text.is_empty()
                && ui
                    .add_sized(
                        [24.0, response.rect.height()],
                        egui::Button::new(
                            RichText::new("\u{ea76}").font(glyph_font(font(ui).size)),
                        )
                        .frame(false),
                    )
                    .on_hover_text(locale.text("清除搜索", "Clear search"))
                    .clicked();
            let escape = response.has_focus()
                && ui
                    .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            if response.has_focus() {
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        response.id,
                        egui::EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                            ..Default::default()
                        },
                    )
                });
            }
            if clear || escape {
                if escape && self.search_text.is_empty() {
                    response.surrender_focus();
                } else {
                    self.search_text.clear();
                    if clear {
                        response.request_focus();
                    }
                }
            }
            // Synchronize on change only. A new request cancels the previous query.
            if response.changed() || clear || escape {
                client.set_search(&self.search_text);
                self.search_scroll_reset = true;
            }
        });
    }
}

fn search_results(
    ui: &mut egui::Ui,
    search: &crate::project_files::SearchState,
    client: &FileClient,
    locale: i18n::Locale,
    reset_scroll: bool,
) -> Option<PathBuf> {
    let results = match &search.result {
        None => {
            ui.weak(locale.text("正在搜索…", "Searching…"));
            return None;
        }
        Some(Err(error)) => {
            ui.weak(error_message(error, locale));
            return None;
        }
        Some(Ok(results)) => results,
    };
    if results.partial {
        ui.weak(locale.text(
            "仅显示部分结果，可输入更具体的路径。",
            "Partial results. Try a more specific path.",
        ));
    } else if results.paths.is_empty() {
        ui.weak(locale.text("未找到匹配的文件", "No matching files"));
    }
    let font = font(ui);
    let path_font = egui::FontId::new((font.size - 2.0).max(1.0), font.family.clone());
    let line_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
    let path_height = ui.fonts_mut(|fonts| fonts.row_height(&path_font));
    let row_height = line_height + path_height + 8.0;
    let root = client.root.as_deref().unwrap_or_else(|| Path::new(""));
    let mut open = None;
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(("project-search-results", root))
        .auto_shrink([false, false]);
    if reset_scroll {
        scroll = scroll.vertical_scroll_offset(0.0);
    }
    scroll.show_rows(ui, row_height, results.paths.len(), |ui, range| {
        for path in &results.paths[range] {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), row_height),
                egui::Sense::click(),
            );
            let absolute = root.join(path).display().to_string();
            let response = response.on_hover_text(&absolute);
            let selected = client.file.as_ref().is_some_and(|file| file.path == *path);
            if selected || response.hovered() {
                ui.painter().rect_filled(
                    rect,
                    3,
                    if selected {
                        chrome::SELECTED
                    } else {
                        chrome::HOVER
                    },
                );
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let icon_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(6.0, 4.0),
                egui::Vec2::splat(font.size),
            );
            file_icons::icon(&name, false).paint(ui, icon_rect);
            let bounds = egui::Rect::from_min_max(
                egui::pos2(icon_rect.right() + 7.0, rect.top() + 3.0),
                rect.max,
            );
            let mut labels = ui.new_child(egui::UiBuilder::new().id_salt(path).max_rect(bounds));
            labels.set_clip_rect(bounds.intersect(ui.clip_rect()));
            labels.spacing_mut().item_spacing.y = 0.0;
            crate::ui_text::label_with_font(
                &mut labels,
                &name,
                font.clone(),
                chrome::TEXT,
                bounds.width().max(0.0),
            );
            if let Some(directory) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
                crate::ui_text::label_with_font(
                    &mut labels,
                    &directory.to_string_lossy(),
                    path_font.clone(),
                    chrome::SECONDARY,
                    bounds.width().max(0.0),
                );
            }
            response.context_menu(|ui| {
                if ui
                    .button(locale.text("复制完整路径", "Copy full path"))
                    .clicked()
                {
                    ui.ctx().copy_text(absolute);
                    ui.close();
                }
            });
            if response.clicked() {
                open = Some(path.clone());
            }
        }
    });
    open
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
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().button_padding.x = 4.0;
                ui.horizontal(|ui| {
                    let controls = 3.0;
                    let available =
                        (ui.available_width() - controls * (chrome::CONTROL + 4.0) - 8.0).max(0.0);
                    let font = font(ui);
                    let files_label = locale.text("文件", "Files");
                    let file_width = font.size
                        + ui.painter()
                            .layout_no_wrap(files_label.into(), font.clone(), chrome::TEXT)
                            .size()
                            .x
                        + 18.0;
                    let git_width = font.size
                        + ui.painter()
                            .layout_no_wrap("Git".into(), font, chrome::TEXT)
                            .size()
                            .x
                        + 18.0;
                    let fraction = (available / (file_width + git_width)).min(1.0);
                    if tab(
                        ui,
                        PROJECT_FILES_ICON,
                        files_label,
                        *files_open,
                        file_width * fraction,
                    )
                    .clicked()
                    {
                        *files_open = true;
                        *git_open = false;
                    }
                    if tab(ui, "\u{ec6f}", "Git", *git_open, git_width * fraction).clicked() {
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
                        if icon_button(ui, "\u{eb37}", locale.text("刷新", "Refresh"), false)
                            .clicked()
                        {
                            if *files_open {
                                client.refresh();
                            } else {
                                result.git.refresh = true;
                            }
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
                });
            });
            if !*files_open && !*git_open {
                return;
            }
            if *files_open {
                ui.separator();
            }
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
impl FileView {
    pub fn release_document(&mut self, ctx: &egui::Context) {
        self.preview.clear(ctx);
    }
    pub fn document(&mut self, ui: &mut egui::Ui, client: &mut FileClient) -> bool {
        let Some(state) = client.file.clone() else {
            self.preview.clear(ui.ctx());
            return false;
        };
        if state.data.is_none() && !state.loading {
            self.preview.clear(ui.ctx());
        }
        let root = client.root.clone();
        let locale = i18n::locale(ui.ctx());
        let font = font(ui);
        ui.style_mut().override_font_id = Some(font.clone());
        let close;
        let (row, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), chrome::CONTROL),
            egui::Sense::hover(),
        );
        {
            let mut header = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("project-document-header")
                    .max_rect(row)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            let ui = &mut header;
            close = chrome::icon_button(
                ui,
                egui::Id::new("project-document-close"),
                chrome::ActionIcon::Close,
                locale.text("返回终端", "Return to terminal"),
            )
            .clicked();
            ui.weak(locale.text("只读", "Read only"));
            if state.loading {
                ui.label("…");
            }
            let width = ui.available_width().max(0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, chrome::CONTROL),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(width);
                    file_icons::icon(&state.path.to_string_lossy(), false).show(ui, font.size);
                    ui.add(
                        egui::Label::new(
                            RichText::new(state.path.to_string_lossy().replace('\\', "/")).strong(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(
                        root.as_ref()
                            .map(|r| r.join(&state.path))
                            .unwrap_or_else(|| state.path.clone())
                            .display()
                            .to_string(),
                    );
                },
            );
        }
        ui.separator();
        if let Some(error) = &state.error {
            ui.colored_label(chrome::WAITING, error_message(error, locale));
            if ui.button(locale.text("重新加载", "Reload")).clicked() {
                client.reload_file();
            }
        }
        if let Some(data) = &state.data {
            self.preview.show(ui, client, data, &state.path);
        }
        if close {
            self.preview.clear(ui.ctx());
        }
        close
    }
}
