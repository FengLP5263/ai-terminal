//! Native read-only repository panel, shared by the app and acceptance probes.
use crate::{
    chrome,
    git::{GitClient, GitError, MAX_COMMITS, Snapshot},
    i18n,
};
use egui::{Color32, RichText};
pub mod diff;
mod files;
mod history;
const HISTORY_INDENT: f32 = 25.0;

#[derive(Default)]
pub struct GitView {
    history: bool,
    selected: Option<String>,
    open_file: Option<(crate::git::PatchSource, String)>,
    inspection: Option<crate::git::Inspection>,
    expanded_details: Option<std::sync::Arc<crate::git::CommitDetails>>,
    copied: Option<String>,
}
#[derive(Default)]
pub struct Actions {
    pub refresh: bool,
    pub more: bool,
    pub close: bool,
    pub inspect: Option<String>,
    pub file: Option<(crate::git::PatchSource, String)>,
}

/// Header text uses the font selected in ai-terminal, at the inherited UI size.
fn header_font(ui: &egui::Ui) -> egui::FontId {
    let mut font = egui::TextStyle::Body.resolve(ui.style());
    font.family = egui::FontFamily::Monospace;
    font
}
fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.into(), header_font(ui), chrome::TEXT)
        .size()
        .x
}
fn branch_label(ui: &egui::Ui, client: Option<&GitClient>) -> String {
    if let Some(snapshot) = client.and_then(|c| c.snapshot.as_deref()) {
        format!(
            "{}{}{}",
            snapshot.branch,
            if snapshot.detached { " (HEAD)" } else { "" },
            if snapshot.total_files > 0 { " *" } else { "" }
        )
    } else {
        i18n::tr(ui.ctx(), "Git 仓库", "Git repository").into()
    }
}
fn branch_icon(ui: &mut egui::Ui) {
    let font = header_font(ui);
    ui.add(
        egui::Label::new(
            RichText::new("\u{ec6f}")
                .font(egui::FontId::new(
                    font.size,
                    egui::FontFamily::Name("codicons".into()),
                ))
                .color(chrome::SECONDARY),
        )
        .selectable(false),
    );
}
fn summary_width(ui: &egui::Ui, client: Option<&GitClient>) -> f32 {
    let counts = client.and_then(|c| c.snapshot.as_ref()).map_or(0.0, |s| {
        text_width(ui, &format!("+{} −{}", s.added, s.deleted)) + 8.0
    });
    header_font(ui).size + text_width(ui, &branch_label(ui, client)) + counts + 20.0
}
pub fn summary(ui: &mut egui::Ui, client: Option<&GitClient>, open: &mut bool) -> egui::Response {
    ui.scope(|ui| {
        let font = header_font(ui);
        ui.style_mut().override_font_id = Some(font.clone());
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
        ui.spacing_mut().button_padding = egui::vec2(2.0, 0.0);
        ui.horizontal(|ui| {
            let locale = i18n::locale(ui.ctx());
            branch_icon(ui);
            let label = branch_label(ui, client);
            let snapshot = client.and_then(|c| c.snapshot.as_ref());
            let counts = snapshot.map(|s| (format!("+{}", s.added), format!("−{}", s.deleted)));
            let count_width = counts
                .as_ref()
                .map_or(0.0, |(a, d)| text_width(ui, a) + text_width(ui, d) + 8.0);
            let show_counts = counts.is_some() && ui.available_width() >= count_width + 40.0;
            let reserve = if show_counts { count_width } else { 0.0 }
                + if client.is_some_and(|c| {
                    c.loading || c.error.is_some() || c.snapshot.as_ref().is_some_and(|s| s.partial)
                }) {
                    font.size + 4.0
                } else {
                    0.0
                };
            let width = text_width(ui, &label).min((ui.available_width() - reserve - 4.0).max(8.0));
            if ui
                .add_sized(
                    [width, ui.text_style_height(&egui::TextStyle::Body)],
                    egui::Button::new(RichText::new(&label).font(font))
                        .truncate()
                        .selected(*open)
                        .frame(false),
                )
                .on_hover_text(format!(
                    "{}\n{}",
                    label,
                    locale.text("查看更改与提交历史", "View changes and commit history")
                ))
                .clicked()
            {
                *open = !*open;
            }
            if show_counts && let Some((added, deleted)) = counts {
                ui.colored_label(chrome::COMPLETED, added);
                ui.colored_label(chrome::FAILED, deleted);
            }
            if let Some(client) = client {
                if client.loading {
                    ui.label("…")
                        .on_hover_text(locale.text("刷新中…", "Refreshing…"));
                } else if client.error.is_some() {
                    ui.colored_label(chrome::WAITING, "!")
                        .on_hover_text(locale.text(
                            "Git 刷新失败，点击面板查看",
                            "Git refresh failed; open the panel for details",
                        ));
                } else if snapshot.is_some_and(|s| s.partial) {
                    ui.label("≈").on_hover_text(locale.text(
                        "部分文件未统计，或超过展示上限",
                        "Some files are excluded or exceed display limits",
                    ));
                }
            }
        })
        .response
    })
    .inner
}

pub struct Header<'a> {
    pub name: &'a str,
    pub directory: &'a std::path::Path,
    pub scale: f32,
    pub connection: Option<(crate::model::Agent, &'a crate::status::Status)>,
}
/// One bounded row; long names/paths yield space instead of growing another line.
pub fn header(
    ui: &mut egui::Ui,
    header: Header<'_>,
    client: Option<&GitClient>,
    open: &mut bool,
) -> bool {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), chrome::CONTROL),
        egui::Sense::hover(),
    );
    let font = header_font(ui);
    let gap = 8.0;
    let mut right = rect.right();
    let mut reset = false;
    let child = |ui: &mut egui::Ui, name: &str, left: f32, width: f32| {
        let bounds = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(width.max(0.0), rect.height()),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(name)
                .max_rect(bounds)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.set_clip_rect(bounds.intersect(ui.clip_rect()));
        child.style_mut().override_font_id = Some(font.clone());
        child.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
        child
    };
    let scale = format!("{:.0}%", header.scale * 100.0);
    let scale_width = (text_width(ui, &scale) + 8.0).min(rect.width() * 0.3);
    right -= scale_width;
    let mut scale_ui = child(ui, "terminal-header-scale", right, scale_width);
    reset |= scale_ui
        .add_sized(
            [scale_width, chrome::CONTROL],
            egui::Button::new(scale).frame(false),
        )
        .on_hover_text(i18n::tr(
            ui.ctx(),
            "Ctrl＋滚轮缩放整个界面 · 点击恢复 100%",
            "Ctrl+wheel zooms the window · Click to reset to 100%",
        ))
        .clicked();
    right -= gap;
    if let Some((agent, status)) = header.connection
        && rect.width() > 540.0
    {
        let label = i18n::activity(i18n::locale(ui.ctx()), status.activity);
        let width = text_width(ui, label) + 20.0;
        right -= width;
        crate::connection_ui::show(
            &mut child(ui, "terminal-header-status", right, width),
            agent,
            status,
        );
        right -= gap;
    }
    let name_width = crate::ui_text::layout_font_rows(
        ui.ctx(),
        header.name,
        font.clone(),
        chrome::TEXT,
        f32::INFINITY,
        1,
    )
    .size()
    .x
    .min(rect.width() * 0.24)
    .min(220.0);
    let git_width = summary_width(ui, client)
        .min((right - rect.left() - name_width - gap * 2.0).max(0.0))
        .min(rect.width() * 0.45);
    right -= git_width;
    summary(
        &mut child(ui, "terminal-header-git", right, git_width),
        client,
        open,
    );
    right -= gap;
    let name_width = name_width.min((right - rect.left()).max(0.0));
    crate::ui_text::label_with_font(
        &mut child(ui, "terminal-header-name", rect.left(), name_width),
        header.name,
        font.clone(),
        chrome::TEXT,
        name_width,
    )
    .on_hover_text(format!("{}\n{}", header.name, header.directory.display()));
    let left = rect.left() + name_width + gap;
    if right - left > 24.0 {
        let mut path_ui = child(ui, "terminal-header-directory", left, right - left);
        crate::ui_text::label_with_font(
            &mut path_ui,
            &header.directory.to_string_lossy(),
            font,
            chrome::SECONDARY,
            right - left,
        )
        .on_hover_text(header.directory.display().to_string());
    }
    reset
}
impl GitView {
    pub fn set_history(&mut self, history: bool) {
        self.history = history;
    }
    pub fn reset(&mut self) {
        self.selected = None;
        self.open_file = None;
        self.inspection = None;
        self.expanded_details = None;
    }
    pub fn panel(
        &mut self,
        ui: &mut egui::Ui,
        client: Option<&GitClient>,
        open: &mut bool,
    ) -> Actions {
        let open_file = client
            .and_then(|c| c.patch.as_ref())
            .map(|p| (&p.source, &p.path));
        if self.open_file.as_ref().map(|(id, path)| (id, path)) != open_file {
            self.open_file = open_file.map(|(id, path)| (id.clone(), path.clone()));
        }
        if !*open {
            return Actions::default();
        }
        self.inspection = client.and_then(|c| c.inspection.clone());
        if let Some(data) = self.inspection.as_ref().and_then(|i| i.data.as_ref())
            && self.selected.as_deref() == Some(data.id.as_str())
        {
            self.expanded_details = Some(data.clone());
        }
        let mut actions = Actions::default();
        let available = ui.available_width();
        let maximum = (available * 0.55).max(80.0);
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
                if client.is_some_and(|c| c.snapshot.is_some() && !c.watching) {
                    ui.weak(i18n::tr(
                        ui.ctx(),
                        "目录监控不可用，可手动刷新。",
                        "Directory monitoring is unavailable; refresh manually.",
                    ));
                }
                actions = self.contents(
                    ui,
                    client.and_then(|c| c.snapshot.as_deref()),
                    client.and_then(|c| c.error.as_ref()),
                    client.is_some_and(|c| c.loading),
                    client.is_some_and(|c| c.not_repository),
                    client.map_or(50, |c| c.limit),
                );
            });
        if actions.close {
            *open = false;
        }
        actions
    }
    /// Captures/tests use the same controls and immutable snapshot as production.
    pub fn contents(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&Snapshot>,
        error: Option<&GitError>,
        loading: bool,
        not_repository: bool,
        limit: usize,
    ) -> Actions {
        ui.style_mut().override_font_id = Some(header_font(ui));
        let locale = i18n::locale(ui.ctx());
        let mut actions = Actions::default();
        ui.horizontal(|ui| {
            ui.heading("Git");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                actions.close = chrome::icon_button(
                    ui,
                    egui::Id::new("git-close"),
                    chrome::ActionIcon::Close,
                    locale.text("收起 Git 面板", "Close Git panel"),
                )
                .clicked();
                actions.refresh = ui
                    .add_enabled(
                        !loading,
                        egui::Button::new(if loading {
                            locale.text("读取中…", "Reading…")
                        } else {
                            locale.text("刷新", "Refresh")
                        }),
                    )
                    .on_hover_text(locale.text("读取本地仓库", "Read local repository"))
                    .clicked();
            });
        });
        if let Some(error) = error {
            let message = match error {
                GitError::Missing => locale
                    .text(
                        "未找到 Git，请安装 Git for Windows 后刷新。",
                        "Git was not found. Install Git for Windows, then refresh.",
                    )
                    .into(),
                GitError::Command(message) => {
                    format!(
                        "{}\n{}",
                        locale.text("读取失败", "Read failed"),
                        display_text(message, 700)
                    )
                }
                GitError::Cancelled => String::new(),
                GitError::InvalidUtf8 => locale
                    .text(
                        "文件内容不是有效 UTF-8，无法准确显示。",
                        "File content is not valid UTF-8 and cannot be displayed accurately.",
                    )
                    .into(),
            };
            ui.colored_label(chrome::WAITING, message);
        }
        let Some(snapshot) = snapshot else {
            if !loading && error.is_none() {
                ui.label(if not_repository {
                    locale.text(
                        "当前目录不在 Git 仓库中。",
                        "The current directory is not in a Git repository.",
                    )
                } else {
                    locale.text(
                        "选择一个终端以查看其仓库。",
                        "Select a terminal to inspect its repository.",
                    )
                });
            }
            return actions;
        };
        ui.label(
            RichText::new(
                snapshot
                    .root
                    .file_name()
                    .unwrap_or(snapshot.root.as_os_str())
                    .to_string_lossy(),
            )
            .strong(),
        )
        .on_hover_text(snapshot.root.display().to_string());
        ui.horizontal_wrapped(|ui| {
            branch_icon(ui);
            ui.label(RichText::new(&snapshot.branch).strong());
            if snapshot.detached {
                ui.weak(locale.text("游离 HEAD", "Detached HEAD"));
            }
            if snapshot.unborn {
                ui.weak(locale.text("尚无提交", "No commits yet"));
            }
        });
        ui.horizontal(|ui|{
            ui.label(RichText::new(format!("+{}",snapshot.added)).color(chrome::COMPLETED));
            ui.label(RichText::new(format!("−{}",snapshot.deleted)).color(chrome::FAILED));
            ui.weak(format!("{} {}",snapshot.total_files,locale.text("个文件","files")));
        }).response.on_hover_text(locale.text("相对 HEAD 的净变化，含暂存、未暂存及未跟踪文本；二进制不计行数。","Net changes against HEAD, including staged, unstaged and untracked text. Binary files have no line counts."));
        if snapshot.partial {
            ui.weak(locale.text(
                "部分文件未统计，或超过展示上限；总数不完整。",
                "Some files are excluded or exceed display limits; totals are partial.",
            ));
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.history,
                false,
                format!(
                    "{} ({})",
                    locale.text("更改", "Changes"),
                    snapshot.total_files
                ),
            );
            ui.selectable_value(&mut self.history, true, locale.text("提交历史", "History"));
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(if self.history {
                "git-history-scroll"
            } else {
                "git-files-scroll"
            })
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.history {
                    if snapshot.commits.is_empty() {
                        ui.weak(
                            locale.text("当前分支尚无提交。", "This branch has no commits yet."),
                        );
                    }
                    history::show(ui, self, snapshot, &mut actions);
                    if snapshot.more && limit < MAX_COMMITS {
                        actions.more = ui
                            .button(locale.text("加载更多提交", "Load more commits"))
                            .clicked();
                    } else if snapshot.more {
                        ui.weak(locale.text(
                            "已显示最近 300 条提交。",
                            "Showing the most recent 300 commits.",
                        ));
                    }
                } else {
                    if snapshot.files.is_empty() {
                        ui.label(locale.text("工作区干净", "Working tree clean"));
                    }
                    for file in &snapshot.files {
                        let response = files::show(
                            ui,
                            files::FileRow {
                                path: &file.path,
                                status: &format!("{}{}", file.index, file.worktree),
                                added: file.added,
                                deleted: file.deleted,
                                binary: file.binary,
                                omitted: file.omitted,
                                selected: self.open_file.as_ref().is_some_and(|(source, path)| {
                                    source == &crate::git::PatchSource::WorkingTree
                                        && path == &file.path
                                }),
                            },
                            egui::Id::new(("git-working-file", &file.path)),
                        );
                        if response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            actions.file =
                                Some((crate::git::PatchSource::WorkingTree, file.path.clone()));
                        }
                    }
                }
            });
        actions
    }
}
fn display_text(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut result: String = chars
        .by_ref()
        .take(limit)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}
fn date(timestamp: i64) -> String {
    let days = timestamp.div_euclid(86400) + 719468;
    let era = days.div_euclid(146097);
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
