//! Read-only media presentation. Disk I/O, decoding and syntax preparation belong to the worker.
use crate::{
    chrome,
    git::FilePatch,
    i18n,
    project_files::{Document, FileClient},
    project_media::{self, Markdown, PreviewImage},
};
use egui::{Color32, TextureHandle};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

type Images = BTreeMap<String, Result<Arc<egui::ColorImage>, String>>;
#[derive(Default)]
struct PreparedImages(Mutex<Images>);
impl egui::load::ImageLoader for PreparedImages {
    fn id(&self) -> &str {
        concat!(module_path!(), "::PreparedImages")
    }
    fn load(
        &self,
        _: &egui::Context,
        uri: &str,
        _: egui::load::SizeHint,
    ) -> egui::load::ImageLoadResult {
        match self.0.lock().unwrap().get(uri).cloned() {
            Some(Ok(image)) => Ok(egui::load::ImagePoll::Ready { image }),
            Some(Err(message)) => Err(egui::load::LoadError::Loading(message)),
            None => Err(egui::load::LoadError::NotSupported),
        }
    }
    fn forget(&self, uri: &str) {
        self.0.lock().unwrap().remove(uri);
    }
    fn forget_all(&self) {
        self.0.lock().unwrap().clear();
    }
    fn byte_size(&self) -> usize {
        self.0
            .lock()
            .unwrap()
            .values()
            .filter_map(|i| i.as_ref().ok())
            .map(|i| i.pixels.len() * 4)
            .sum()
    }
}

#[derive(Default)]
pub struct View {
    identity: Option<(std::path::PathBuf, usize)>,
    document: Option<Arc<Document>>,
    source: bool,
    zoom: Option<f32>,
    texture: Option<TextureHandle>,
    checker: Option<TextureHandle>,
    markdown: CommonMarkCache,
    images: Arc<PreparedImages>,
    registered: bool,
    uris: Vec<String>,
    note: Option<String>,
    pending_link: Option<std::path::PathBuf>,
    player: Option<Result<crate::media_player::Player, String>>,
}
impl View {
    pub fn clear(&mut self, ctx: &egui::Context) {
        if self.identity.is_none() {
            return;
        }
        for uri in self.uris.drain(..) {
            ctx.forget_image(&uri);
        }
        self.images.0.lock().unwrap().clear();
        self.player = None;
        self.identity = None;
        self.document = None;
        self.texture = None;
        self.markdown = CommonMarkCache::default();
        self.note = None;
        self.pending_link = None;
    }
    fn select(&mut self, ctx: &egui::Context, path: &Path, data: &Arc<Document>) {
        let key = (path.to_path_buf(), Arc::as_ptr(data) as usize);
        if self.identity.as_ref() == Some(&key) {
            return;
        }
        let same_file = self.identity.as_ref().is_some_and(|(p, _)| p == path);
        self.clear(ctx);
        self.identity = Some(key);
        self.document = Some(data.clone());
        if !same_file {
            self.source = false;
            self.zoom = None;
        }
        if !self.registered {
            ctx.add_image_loader(self.images.clone());
            self.registered = true;
        }
        if let Some(md) = &data.markdown {
            for link in &md.links {
                if !link.starts_with('#') {
                    self.markdown.add_link_hook(link.clone());
                }
            }
            let locale = i18n::locale(ctx);
            let mut images = self.images.0.lock().unwrap();
            for (uri, result) in &md.images {
                self.uris.push(uri.clone());
                images.insert(
                    uri.clone(),
                    result
                        .as_ref()
                        .map(|i| i.pixels.clone())
                        .map_err(|e| crate::project_ui::error_message(e, locale)),
                );
            }
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        client: &mut FileClient,
        data: &Arc<Document>,
        path: &Path,
    ) {
        self.select(ui.ctx(), path, data);
        if let Some(media) = &data.media {
            match self
                .player
                .get_or_insert_with(|| crate::media_player::Player::new(ui.ctx(), media))
            {
                Ok(player) => player.show(ui),
                Err(error) => {
                    let locale = i18n::locale(ui.ctx());
                    ui.colored_label(
                        chrome::WAITING,
                        locale.text(
                            "无法初始化音视频预览，请检查 Windows 媒体组件。",
                            "Cannot initialize media preview. Check Windows media components.",
                        ),
                    );
                    ui.weak(error.as_str());
                }
            }
            return;
        }
        let locale = i18n::locale(ui.ctx());
        let paged = data.page.next.is_some() || data.page.cursor.offset > 0;
        let mut previous = false;
        let mut next = false;
        ui.horizontal_wrapped(|ui| {
            if project_media::markdown_path(path) {
                ui.selectable_value(&mut self.source, false, locale.text("预览", "Preview"));
                ui.selectable_value(&mut self.source, true, locale.text("源码", "Source"));
                ui.add_space(8.0);
            }
            if let Some(image) = &data.image {
                if ui
                    .selectable_label(self.zoom.is_none(), locale.text("适应窗口", "Fit"))
                    .clicked()
                {
                    self.zoom = None;
                }
                if ui
                    .selectable_label(self.zoom == Some(1.0), "100%")
                    .clicked()
                {
                    self.zoom = Some(1.0);
                }
                if ui.small_button("−").clicked() {
                    self.zoom = Some((self.zoom.unwrap_or(1.0) / 1.25).max(0.05));
                }
                if ui.small_button("+").clicked() {
                    self.zoom = Some((self.zoom.unwrap_or(1.0) * 1.25).min(8.0));
                }
                ui.weak(format!(
                    "{} × {} · {} · {}",
                    image.original[0],
                    image.original[1],
                    image.format,
                    byte_size(data.page.total_bytes)
                ));
            } else {
                if paged {
                    if data.page.cursor.offset > 0
                        && ui.button(locale.text("开头", "First")).clicked()
                    {
                        client.reload_file();
                    }
                    previous = ui
                        .add_enabled(
                            client.can_previous_page()
                                && !client.file.as_ref().is_some_and(|f| f.loading),
                            egui::Button::new(locale.text("上一页", "Previous")),
                        )
                        .clicked();
                    next = ui
                        .add_enabled(
                            data.page.next.is_some()
                                && !client.file.as_ref().is_some_and(|f| f.loading),
                            egui::Button::new(locale.text("下一页", "Next")),
                        )
                        .clicked();
                    let end = data.page.next.map_or(data.page.total_bytes, |c| c.offset);
                    ui.weak(format!(
                        "{}–{} / {}",
                        byte_size(data.page.cursor.offset),
                        byte_size(end),
                        byte_size(data.page.total_bytes)
                    ));
                } else {
                    ui.weak(format!(
                        "{} · {}",
                        data.encoding,
                        byte_size(data.page.total_bytes)
                    ));
                }
            }
        });
        if paged {
            ui.weak(locale.text(
                "分段查看；长行会折成连续行，内容不会省略。",
                "Paged view; long lines continue on following rows without omissions.",
            ));
        }
        if let Some(note) = &self.note {
            ui.weak(note);
        }
        if let Some(image) = &data.image {
            if image.first_frame {
                ui.weak(locale.text(
                    "静态预览（动画仅显示首帧）",
                    "Static preview (first frame for animation)",
                ));
            }
            if image.pixels.size != [image.original[0] as usize, image.original[1] as usize] {
                ui.weak(locale.text(
                    "大图已缩采样预览，原文件保持不变。",
                    "Large image downsampled for preview; original file unchanged.",
                ));
            }
            self.show_image(ui, image);
        } else if !self.source
            && let Some(md) = &data.markdown
        {
            self.show_markdown(ui, md, path);
        } else {
            if project_media::markdown_path(path) && !self.source {
                ui.weak(locale.text(
                    "此段 Markdown 结构较复杂，已切换为源码显示。",
                    "This Markdown section is too complex; showing source.",
                ));
            }
            if data.content.lines.is_empty() {
                ui.weak(locale.text("文件内容为空。", "File is empty."));
            }
            crate::git_ui::diff::body(
                ui,
                &data.content,
                ("project-document", &self.identity),
                true,
            );
        }
        // Apply requests only after rendering the immutable worker result.
        if previous {
            client.previous_page();
        }
        if next {
            client.next_page();
        }
        if let Some(link) = self.markdown_link() {
            client.open_file(&link);
        }
    }
    // A deferred local file click is stored independently of the CommonMark cache.
    fn markdown_link(&mut self) -> Option<std::path::PathBuf> {
        self.pending_link.take()
    }
    fn show_image(&mut self, ui: &mut egui::Ui, image: &PreviewImage) {
        let texture = self.texture.get_or_insert_with(|| {
            ui.ctx().load_texture(
                "project-image",
                egui::ImageData::Color(image.pixels.clone()),
                egui::TextureOptions::LINEAR,
            )
        });
        let checker = self.checker.get_or_insert_with(|| {
            let pixels = (0..16)
                .flat_map(|y| {
                    (0..16).map(move |x| {
                        Color32::from_gray(if (x / 8 + y / 8) % 2 == 0 { 48 } else { 62 })
                    })
                })
                .collect();
            ui.ctx().load_texture(
                "project-image-checker",
                egui::ColorImage::new([16, 16], pixels),
                egui::TextureOptions {
                    wrap_mode: egui::TextureWrapMode::Repeat,
                    ..egui::TextureOptions::NEAREST
                },
            )
        });
        let available = ui.available_size().max(egui::vec2(1.0, 1.0));
        let original = egui::vec2(image.original[0] as f32, image.original[1] as f32);
        let scale = self.zoom.unwrap_or_else(|| {
            (available.x / original.x)
                .min(available.y / original.y)
                .min(1.0)
        });
        let size = original * scale;
        egui::ScrollArea::both()
            .id_salt(("project-image", &self.identity))
            .show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                ui.painter().image(
                    checker.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, (size / 16.0).to_pos2()),
                    Color32::WHITE,
                );
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            });
    }
    fn show_markdown(&mut self, ui: &mut egui::Ui, md: &Markdown, path: &Path) {
        let document = self.document.as_ref().unwrap().clone();
        let render_code = move |ui: &mut egui::Ui, html: &str| {
            let md = document.markdown.as_ref().unwrap();
            if let Some(index) = html
                .trim()
                .strip_prefix(&format!("<{}-", md.code_marker))
                .and_then(|s| s.strip_suffix(" />"))
                .and_then(|s| s.parse::<usize>().ok())
                && let Some(patch) = md.code.get(index)
            {
                code_block(ui, patch);
            }
        };
        // Markdown prose uses the normal UI font; only code uses the monospace family.
        ui.style_mut().override_font_id = None;
        // Pages are bounded by the worker; use the stable renderer for full Markdown structures.
        egui::ScrollArea::vertical()
            .id_salt(("project-markdown", &self.identity))
            .show(ui, |ui| {
                CommonMarkViewer::new()
                    .explicit_image_uri_scheme(true)
                    .enable_scroll_to_heading(true)
                    .render_html_fn(Some(&render_code))
                    .show(ui, &mut self.markdown, &md.text);
            });
        for link in &md.links {
            if self.markdown.get_link_hook(link) == Some(true) {
                if project_media::external_link(link) {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(link));
                } else if let Ok(target) = project_media::local_target(path, link) {
                    self.pending_link = Some(target);
                } else {
                    self.note = Some(
                        i18n::tr(ui.ctx(), "不支持此链接。", "This link is not supported.").into(),
                    );
                }
            }
        }
    }
}
fn code_block(ui: &mut egui::Ui, patch: &FilePatch) {
    egui::Frame::new()
        .fill(Color32::from_gray(30))
        .inner_margin(10.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.weak(patch.path.strip_prefix("code.").unwrap_or(&patch.path));
                    if ui
                        .small_button(i18n::tr(ui.ctx(), "复制", "Copy"))
                        .clicked()
                    {
                        ui.ctx().copy_text(
                            patch
                                .lines
                                .iter()
                                .map(|l| l.text.as_str())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        );
                    }
                });
                for row in &patch.presentation.rows {
                    let Some(line) = patch.lines.get(row.source) else {
                        continue;
                    };
                    let mut job = egui::text::LayoutJob::default();
                    let font = egui::TextStyle::Monospace.resolve(ui.style());
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
                                font_id: font,
                                color: chrome::TEXT,
                                ..Default::default()
                            },
                        );
                    }
                    ui.add(egui::Label::new(job).selectable(true).wrap());
                }
            });
        });
}
pub fn byte_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
