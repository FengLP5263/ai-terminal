//! Native command execution stays in background logic, including minimized windows.
use super::*;
use ai_terminal::control::actions::*;
use std::path::{Component, Path};
#[derive(Default)]
pub(crate) struct CaptureState {
    pending: Option<(String, PathBuf, Instant)>,
    receiver: Option<std::sync::mpsc::Receiver<Result<Value, String>>>,
    result: Option<Value>,
}
impl CaptureState {
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        if let Some((id, path, deadline)) = &self.pending {
            let image = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot {
                        image, user_data, ..
                    } if user_data
                        .data
                        .as_ref()
                        .and_then(|d| d.downcast_ref::<String>())
                        == Some(id) =>
                    {
                        Some(image.clone())
                    }
                    _ => None,
                })
            });
            if let Some(image) = image {
                let path = path.clone();
                let (send, receive) = std::sync::mpsc::sync_channel(1);
                let context = ctx.clone();
                let spawned = std::thread::Builder::new()
                    .name("control-capture".into())
                    .spawn(move || {
                        let _ = send.send(save_capture(&path, &image));
                        context.request_repaint_of(egui::ViewportId::ROOT);
                    });
                match spawned {
                    Ok(_) => self.receiver = Some(receive),
                    Err(e) => self.result = Some(json!({"state":"failed","error":e.to_string()})),
                }
                self.pending = None;
            } else if Instant::now() >= *deadline {
                self.result = Some(
                    json!({ "state":"failed","error":"Capture timed out; restore the main window and retry" } ),
                );
                self.pending = None;
            }
        }
        if let Some(receiver) = &self.receiver
            && let Ok(result) = receiver.try_recv()
        {
            self.result = Some(match result {
                Ok(v) => v,
                Err(e) => json!({ "state":"failed","error":e } ),
            });
            self.receiver = None;
        }
    }
    fn start(&mut self, path: &Path, ctx: &egui::Context) -> ApiResult<Value> {
        if self.pending.is_some() || self.receiver.is_some() {
            return Err(ApiError::new("busy", "A capture is already pending"));
        }
        if !path.is_absolute()
            || path
                .extension()
                .is_none_or(|s| !s.eq_ignore_ascii_case("png"))
        {
            return Err(invalid("Capture output must be an absolute PNG path"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let result = json!({ "state":"pending","capture_id":id,"output":path } );
        self.result = Some(result.clone());
        self.pending = Some((
            id.clone(),
            path.into(),
            Instant::now() + Duration::from_secs(10),
        ));
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::Screenshot(egui::UserData::new(id)),
        );
        ctx.request_repaint_after(Duration::from_secs(10));
        Ok(result)
    }
}
fn save_capture(path: &Path, image: &egui::ColorImage) -> Result<Value, String> {
    use std::io::Write;
    if image.pixels.len() > 16_777_216 {
        return Err("Capture exceeds 64 MiB".into());
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let rgba = image::RgbaImage::from_raw(
            image.width() as u32,
            image.height() as u32,
            image.as_raw().to_vec(),
        )
        .ok_or("Invalid framebuffer")?;
        let mut writer = std::io::BufWriter::new(file);
        image::DynamicImage::ImageRgba8(rgba)
            .write_to(&mut writer, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        Ok(json!({"state":"complete","output":path,"width":image.width(),"height":image.height()}))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(path);
    }
    result
}

fn page<T: serde::Serialize>(items: &[T], offset: usize, limit: usize) -> ApiResult<Value> {
    if limit == 0 || limit > 200 {
        return Err(invalid("limit must be between 1 and 200"));
    }
    let mut bytes = 0;
    let items_page: Vec<_> = items
        .iter()
        .skip(offset)
        .take(limit)
        .take_while(|item| {
            bytes += serde_json::to_vec(item).map_or(262_145, |v| v.len());
            bytes <= 262_144
        })
        .collect();
    if items_page.is_empty() && offset < items.len() {
        return Err(ApiError::new(
            "too_large",
            "One item exceeds the response page budget",
        ));
    }
    let next = offset.saturating_add(items_page.len());
    Ok(
        json!({ "items":items_page,"total":items.len(),"offset":offset,"next_offset":(next < items.len()).then_some(next) } ),
    )
}
fn commit_id(id: &str) -> ApiResult<()> {
    if !matches!(id.len(), 40 | 64) || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("Use a full 40 or 64 character commit ID"));
    }
    Ok(())
}
fn read_error(error: &impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

impl App {
    fn native_settings(&self) -> Value {
        json!({ "settings":self.preferences,"ui_scale_percent":self.preferences.ui_scale*100.,"floating_scale_percent":self.preferences.floating_scale*100.,"floating_transparency_percent":(1.-self.preferences.floating_opacity)*100.,"settings_open":self.settings_open } )
    }
    fn native_save_settings(&mut self, p: model::UiSettings, ctx: &egui::Context) -> ApiResult<()> {
        if !ai_terminal::appearance::font_available(p.terminal_font) {
            return Err(ApiError::new(
                "unavailable",
                "The selected font is not installed",
            ));
        }
        let mut workspace = self.workspace_snapshot();
        workspace.ui = p;
        if let Some(id) = &self.active {
            workspace
                .session_panels
                .insert(id.clone(), model::ProjectPanel::from_settings(p));
        }
        self.control_save(&workspace)?;
        if p.terminal_font != self.preferences.terminal_font {
            let actual = ai_terminal::appearance::set_terminal_font(ctx, p.terminal_font);
            self.font = ai_terminal::appearance::terminal_font_for(actual, 14.);
        }
        self.preferences = p;
        self.preferences_save_at = None;
        self.dirty = false;
        self.save_error = None;
        i18n::apply(ctx, p.language);
        ctx.set_zoom_factor(p.ui_scale);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
        Ok(())
    }
    fn native_path(&self, path: &Path) -> ApiResult<PathBuf> {
        let root = self
            .project_files
            .root
            .as_deref()
            .ok_or_else(|| ApiError::new("not_ready", "Project root is not ready"))?;
        let path = if path.is_absolute() {
            path.strip_prefix(root)
                .map_err(|_| invalid("Path is outside the current project"))?
        } else {
            path
        };
        let mut normalized = PathBuf::new();
        for c in path.components() {
            match c {
                Component::CurDir => {}
                Component::Normal(p) => normalized.push(p),
                _ => {
                    return Err(invalid(
                        "Use a relative project path without parent traversal",
                    ));
                }
            }
        }
        if normalized.to_string_lossy().len() > 16_384
            || normalized.to_string_lossy().contains('\0')
        {
            return Err(invalid("Invalid project path"));
        }
        Ok(normalized)
    }
    fn native_document(
        &mut self,
        ctx: &egui::Context,
    ) -> ApiResult<Arc<ai_terminal::project_files::Document>> {
        let file = self
            .project_files
            .file
            .as_ref()
            .ok_or_else(|| ApiError::new("not_found", "No project file is open"))?;
        if file.loading {
            return Err(ApiError::new("not_ready", "File is still loading"));
        }
        let data = file.data.clone().ok_or_else(|| {
            ApiError::new(
                "preview_failed",
                file.error.as_ref().map(read_error).unwrap_or_default(),
            )
        })?;
        self.project_view
            .preview_mut()
            .prepare(ctx, &file.path, &data);
        Ok(data)
    }
    pub(super) fn control_native(
        &mut self,
        action: &Action,
        caller: Option<&str>,
        ctx: &egui::Context,
    ) -> ApiResult<Value> {
        use egui::ViewportCommand as V;
        match action {
            Action::SettingsShow => Ok(self.native_settings()),
            Action::SettingsSet { patch } => {
                let mut p = patch.apply(self.preferences)?;
                if patch.floating_center == Some(true) || patch.floating_position_pixels.is_some() {
                    let count = self.sessions.len().saturating_sub(
                        self.groups
                            .iter()
                            .map(|g| g.members.len().saturating_sub(1))
                            .sum::<usize>(),
                    );
                    p.floating_position_pixels = Some(
                        (if patch.floating_center == Some(true) {
                            ai_terminal::floating::centered_geometry_at_scale
                        } else {
                            ai_terminal::floating::geometry_at_scale
                        })(
                            ctx, count, p.floating_position_pixels, p.floating_scale
                        )
                        .0,
                    );
                }
                self.native_save_settings(p, ctx)?;
                if patch.floating_center == Some(true) || patch.floating_position_pixels.is_some() {
                    self.control_floating_position = p.floating_position_pixels;
                }
                Ok(self.native_settings())
            }
            Action::SettingsReset => {
                let p = model::UiSettings {
                    desktop_shortcut: self.preferences.desktop_shortcut,
                    ..Default::default()
                };
                self.native_save_settings(p, ctx)?;
                self.control_floating_position = None;
                Ok(self.native_settings())
            }
            Action::Fonts => Ok(
                json!({ "fonts":ai_terminal::font_choice::FontChoice::ALL.into_iter().map(|f|json!({ "id":f,"name":f.family(),"available":ai_terminal::appearance::font_available(f) } )).collect::<Vec<_>>() } ),
            ),
            Action::WindowShow => {
                let pixels_per_point = ctx.pixels_per_point();
                Ok(ctx.input(|i| {
                    let v = i.viewport();
                    json!({"focused":v.focused,"minimized":v.minimized,"maximized":v.maximized,"inner_rect":v.inner_rect.map(|r|[r.min.x,r.min.y,r.width(),r.height()]),"outer_rect":v.outer_rect.map(|r|[r.min.x,r.min.y,r.width(),r.height()]),"pixels_per_point":pixels_per_point,"ui_scale":self.preferences.ui_scale})
                }))
            }
            Action::WindowState { state } => {
                match state {
                    WindowState::Focus => ai_terminal::floating::restore_main(ctx),
                    WindowState::Minimize => {
                        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, V::Minimized(true))
                    }
                    WindowState::Maximize => {
                        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, V::Minimized(false));
                        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, V::Maximized(true));
                    }
                    WindowState::Restore => {
                        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, V::Maximized(false));
                        ai_terminal::floating::restore_main(ctx);
                    }
                }
                Ok(json!({ "requested":state } ))
            }
            Action::WindowSet {
                width,
                height,
                x,
                y,
            } => {
                let size = match (width, height) {
                    (Some(w), Some(h)) => Some([
                        range(*w, 640., 16_384., "width")?,
                        range(*h, 400., 16_384., "height")?,
                    ]),
                    (None, None) => None,
                    _ => return Err(invalid("Specify both width and height")),
                };
                let pos = match (x, y) {
                    (Some(x), Some(y)) => Some([
                        range(*x, -100_000., 100_000., "x")?,
                        range(*y, -100_000., 100_000., "y")?,
                    ]),
                    (None, None) => None,
                    _ => return Err(invalid("Specify both x and y")),
                };
                if let Some([w, h]) = size {
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        V::InnerSize(egui::vec2(w, h)),
                    );
                }
                if let Some([x, y]) = pos {
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        V::OuterPosition(egui::pos2(x, y)),
                    );
                }
                Ok(json!({ "requested_size":size,"requested_position":pos } ))
            }
            Action::Capture { output } => self.control_capture.start(output, ctx),
            Action::CaptureStatus => Ok(self
                .control_capture
                .result
                .clone()
                .unwrap_or(json!({ "state":"none" } ))),
            Action::Panel { panel, open } => {
                let mut p = self.preferences;
                match panel {
                    Panel::Settings => {}
                    Panel::Files => {
                        p.project_files_open = *open;
                        if *open {
                            p.git_panel_open = false;
                        }
                    }
                    Panel::Git | Panel::History | Panel::Changes => {
                        p.git_panel_open = *open;
                        if *open {
                            p.project_files_open = false;
                        }
                    }
                }
                if *panel != Panel::Settings {
                    self.native_save_settings(p, ctx)?;
                }
                if matches!(panel, Panel::History | Panel::Changes) {
                    self.git_view.set_history(*panel == Panel::History);
                }
                if *panel == Panel::Settings {
                    self.settings_open = *open;
                }
                self.sync_visibility();
                ctx.request_repaint();
                Ok(self.native_settings())
            }
            Action::ProjectShow => Ok(
                json!({ "root":self.project_files.root,"session":self.active,"file":self.project_files.file.as_ref().map(|f|&f.path),"directory_cache_limit_reached":self.project_files.limit_reached } ),
            ),
            Action::DirectoryList {
                path,
                offset,
                limit,
            } => {
                let path = self.native_path(path)?;
                self.project_files.ensure_directory(&path);
                let state = self.project_files.directories.get(&path).ok_or_else(|| {
                    ApiError::new(
                        "limit_reached",
                        "Directory cache is full; collapse folders first",
                    )
                })?;
                let entries=state.data.as_ref().map(|d|d.entries.iter().map(|e|json!({ "path":e.path,"name":e.name,"kind":match e.kind { ai_terminal::project_files::EntryKind::Directory=>"directory",ai_terminal::project_files::EntryKind::File=>"file",ai_terminal::project_files::EntryKind::Link=>"link",_=>"other" } ,"ignored":e.ignored } )).collect::<Vec<_>>()).unwrap_or_default();
                let mut value = page(&entries, *offset, *limit)?;
                value["path"] = json!(path);
                value["loading"] = json!(state.loading);
                value["error"] = json!(state.error.as_ref().map(read_error));
                value["partial"] = json!(state.data.as_ref().is_some_and(|d| d.partial));
                Ok(value)
            }
            Action::DirectoryExpand { path, expand } => {
                let path = self.native_path(path)?;
                if *expand {
                    self.project_view.expand(&path, &mut self.project_files);
                } else {
                    self.project_view.collapse(&path, &mut self.project_files);
                }
                ctx.request_repaint();
                Ok(json!({ "path":path,"expanded":expand } ))
            }
            Action::DirectoryCollapseAll => {
                self.project_view.collapse_all(&mut self.project_files);
                ctx.request_repaint();
                Ok(json!({ "collapsed":true } ))
            }
            Action::FilesRefresh => {
                self.project_files.refresh();
                Ok(json!({ "queued":true } ))
            }
            Action::FileOpen { path } => {
                let path = self.native_path(path)?;
                if path.as_os_str().is_empty() {
                    return Err(invalid("Specify a file path"));
                }
                self.open_project_file(&path, ctx);
                ctx.request_repaint();
                let loading = self
                    .project_files
                    .file
                    .as_ref()
                    .is_some_and(|file| file.loading);
                Ok(json!({ "path":path,"loading":loading } ))
            }
            Action::FileClose => {
                self.project_view.release_document(ctx);
                self.project_files.close_file();
                self.terminal_focus_pending = true;
                self.sync_visibility();
                ctx.request_repaint();
                Ok(json!({ "closed":true } ))
            }
            Action::FileShow {
                content,
                offset,
                limit,
            } => {
                let Some(file) = &self.project_files.file else {
                    return Ok(json!({ "open":false } ));
                };
                let mut value = json!({ "open":true,"path":file.path,"loading":file.loading,"error":file.error.as_ref().map(read_error) } );
                if let Some(data) = &file.data {
                    let (source, zoom) = self.project_view.preview_mut().controls();
                    value["document"] = json!({ "encoding":data.encoding,"page_bytes":data.bytes,"total_bytes":data.page.total_bytes,"page_offset":data.page.cursor.offset,"first_line":data.page.cursor.line,"has_next_page":data.page.next.is_some(),"has_previous_page":self.project_files.can_previous_page(),"kind":if data.media.is_some(){ "media" } else if data.image.is_some(){ "image" } else if data.markdown.is_some(){ "markdown" } else{ "text" } ,"source":source,"image_zoom_percent":zoom.map(|v|v*100.) } );
                    if *content {
                        value["content"] = page(&data.content.lines, *offset, *limit)?;
                    }
                }
                Ok(value)
            }
            Action::FilePage { direction } => {
                if self.project_files.file.is_none() {
                    return Err(ApiError::new("not_found", "No file is open"));
                }
                match direction {
                    PageDirection::Next => self.project_files.next_page(),
                    PageDirection::Previous => self.project_files.previous_page(),
                    PageDirection::First | PageDirection::Reload => {
                        self.project_files.reload_file()
                    }
                };
                ctx.request_repaint();
                Ok(json!({ "requested":direction } ))
            }
            Action::FileMode { source } => {
                let data = self.native_document(ctx)?;
                if data.markdown.is_none() {
                    return Err(invalid("Source/preview switching requires Markdown"));
                }
                self.project_view.preview_mut().set_source(*source);
                ctx.request_repaint();
                Ok(json!({ "source":source } ))
            }
            Action::ImageZoom { percent } => {
                let zoom = percent
                    .map(|v| range(v, 5., 800., "percent").map(|v| v / 100.))
                    .transpose()?;
                let data = self.native_document(ctx)?;
                if data.image.is_none() {
                    return Err(invalid("No image preview is open"));
                }
                self.project_view.preview_mut().set_zoom(zoom);
                ctx.request_repaint();
                Ok(json!({ "zoom_percent":percent,"fit":percent.is_none() } ))
            }
            Action::Media { operation, value } => {
                if *operation == MediaOperation::Status
                    && self.project_files.file.as_ref().is_some_and(|f| f.loading)
                {
                    return Ok(json!({"loading":true}));
                }
                let data = self.native_document(ctx)?;
                let media = data
                    .media
                    .as_ref()
                    .ok_or_else(|| invalid("No audio or video preview is open"))?;
                #[cfg(windows)]
                {
                    let player = self
                        .project_view
                        .preview_mut()
                        .media_player(ctx, media)
                        .map_err(|e| ApiError::new("playback_failed", e))?;
                    if !player.ready_after_restore() {
                        return if *operation == MediaOperation::Status {
                            Ok(json!({"loading":true}))
                        } else {
                            Err(ApiError::new(
                                "not_ready",
                                "Restoring media playback position",
                            ))
                        };
                    }
                    let result = match operation {
                        MediaOperation::Status => Ok(()),
                        MediaOperation::Play => player.play(),
                        MediaOperation::Pause => player.pause(),
                        MediaOperation::Seek => {
                            let seconds =
                                value.filter(|v| v.is_finite() && *v >= 0.).ok_or_else(|| {
                                    invalid("Seek seconds must be finite and nonnegative")
                                })?;
                            if !player.duration().is_finite() {
                                return Err(ApiError::new(
                                    "not_ready",
                                    "Media duration is not ready",
                                ));
                            }
                            if seconds > player.duration() {
                                return Err(invalid("Seek exceeds media duration"));
                            }
                            player.seek(seconds)
                        }
                        MediaOperation::Volume => {
                            let v = value
                                .filter(|v| v.is_finite() && (0. ..=100.).contains(v))
                                .ok_or_else(|| {
                                    invalid("Volume percent must be between 0 and 100")
                                })?;
                            player.volume(v / 100.)
                        }
                    };
                    result.map_err(|e| ApiError::new("playback_failed", e.to_string()))?;
                    ctx.request_repaint();
                    Ok(
                        json!({"loading":false,"position_seconds":player.position().is_finite().then(||player.position()),"duration_seconds":player.duration().is_finite().then(||player.duration()),"paused":player.paused(),"volume_percent":player.current_volume()*100.,"failed":player.failed()}),
                    )
                }
                #[cfg(not(windows))]
                {
                    let _ = (media, operation, value);
                    Err(ApiError::new(
                        "unsupported_platform",
                        "Media playback requires Windows",
                    ))
                }
            }
            Action::GitShow {
                section,
                offset,
                limit,
            } => self.native_git_show(*section, *offset, *limit),
            Action::GitRefresh | Action::GitMore => {
                let git = self
                    .git
                    .as_mut()
                    .ok_or_else(|| ApiError::new("unavailable", "Git is unavailable"))?;
                if matches!(action, Action::GitMore) {
                    git.more();
                } else {
                    git.refresh();
                }
                Ok(json!({ "queued":true } ))
            }
            Action::GitInspect { id } => {
                commit_id(id)?;
                let git = self
                    .git
                    .as_mut()
                    .ok_or_else(|| ApiError::new("unavailable", "Git is unavailable"))?;
                git.inspect(id);
                self.git_view.select_commit(id);
                Ok(json!({ "queued":true,"id":id } ))
            }
            Action::GitOpenDiff { path, commit } => {
                if let Some(id) = commit {
                    commit_id(id)?;
                }
                if path.is_empty() || path.len() > 16_384 || path.contains('\0') {
                    return Err(invalid("Invalid repository path"));
                }
                let git = self
                    .git
                    .as_mut()
                    .ok_or_else(|| ApiError::new("unavailable", "Git is unavailable"))?;
                git.open_file(
                    commit
                        .as_ref()
                        .map(|id| ai_terminal::git::PatchSource::Commit(id.clone()))
                        .unwrap_or(ai_terminal::git::PatchSource::WorkingTree),
                    path,
                );
                self.project_view.release_document(ctx);
                self.project_files.close_file();
                self.sync_visibility();
                ctx.request_repaint();
                Ok(json!({ "queued":true,"path":path } ))
            }
            Action::GitCloseDiff => {
                if let Some(git) = &mut self.git {
                    git.close_patch();
                }
                self.terminal_focus_pending = true;
                self.sync_visibility();
                ctx.request_repaint();
                Ok(json!({ "closed":true } ))
            }
            Action::TerminalRead { id, offset, limit } => {
                let i = self.control_index(id, caller)?;
                let backend = self.sessions[i]
                    .backend
                    .as_mut()
                    .ok_or_else(|| ApiError::new("session_stopped", "Session is not running"))?;
                let text = backend.plain_text();
                let lines: Vec<_> = text.lines().collect();
                let mut value = page(&lines, *offset, *limit)?;
                let content = backend.last_content();
                value["display_offset"] = json!(content.display_offset);
                value["history_size"] = json!(content.history_size);
                value["scope"] = json!("current_viewport");
                Ok(value)
            }
            Action::TerminalWrite { id, text, submit } => {
                if text.len() > 65_536 || text.contains('\0') {
                    return Err(invalid("Terminal text must be at most 64 KiB without NUL"));
                }
                let i = self.control_index(id, caller)?;
                let backend = self.sessions[i]
                    .backend
                    .as_mut()
                    .ok_or_else(|| ApiError::new("session_stopped", "Session is not running"))?;
                let bracketed = backend
                    .sync()
                    .terminal_mode
                    .contains(egui_term::TerminalMode::BRACKETED_PASTE);
                // Match paste semantics: strip escape characters inside bracketed paste.
                let mut bytes = Vec::new();
                if bracketed {
                    bytes.extend_from_slice(b"\x1b[200~");
                    bytes.extend(text.bytes().filter(|b| *b != 0x1b));
                    bytes.extend_from_slice(b"\x1b[201~");
                } else {
                    bytes.extend_from_slice(
                        text.replace("\r\n", "\n").replace('\n', "\r").as_bytes(),
                    );
                }
                if *submit {
                    bytes.push(b'\r');
                }
                backend.process_command(egui_term::BackendCommand::Write(bytes));
                Ok(
                    json!({ "id":self.sessions[i].spec.id,"input_bytes":text.len(),"submitted":submit } ),
                )
            }
            Action::TerminalKey { id, key } => {
                let i = self.control_index(id, caller)?;
                let backend = self.sessions[i]
                    .backend
                    .as_mut()
                    .ok_or_else(|| ApiError::new("session_stopped", "Session is not running"))?;
                let app = backend
                    .sync()
                    .terminal_mode
                    .contains(egui_term::TerminalMode::APP_CURSOR);
                backend.process_command(egui_term::BackendCommand::Write(key.bytes(app).to_vec()));
                Ok(json!({ "id":self.sessions[i].spec.id,"key":key } ))
            }
            Action::TerminalScroll { id, lines, offset } => {
                let command = match (lines, offset) {
                    (Some(n), None) => {
                        if n.unsigned_abs() > 5000 {
                            return Err(invalid("Scroll lines must be within -5000..5000"));
                        }
                        egui_term::BackendCommand::Scroll(*n)
                    }
                    (None, Some(n)) => {
                        if *n > 5000 {
                            return Err(invalid("History offset exceeds 5000"));
                        }
                        egui_term::BackendCommand::ScrollTo(*n)
                    }
                    _ => return Err(invalid("Specify exactly one of lines or offset")),
                };
                let i = self.control_index(id, caller)?;
                let backend = self.sessions[i]
                    .backend
                    .as_mut()
                    .ok_or_else(|| ApiError::new("session_stopped", "Session is not running"))?;
                backend.process_command(command);
                ctx.request_repaint();
                Ok(json!({ "id":self.sessions[i].spec.id,"scrolled":true } ))
            }
            Action::SessionStop { id, force, restart } => {
                let i = self.control_index(id, caller)?;
                if self.sessions[i].backend.is_some() && !*force {
                    return Err(ApiError::new(
                        "session_running",
                        "Stopping/restarting the owned process tree requires --force",
                    ));
                }
                if self.control_closers.len() >= 32 {
                    return Err(ApiError::new("busy", "Session cleanup queue is full"));
                }
                if let Some(resources) = self.sessions[i].take_resources() {
                    let context = ctx.clone();
                    let task = crate::shutdown::ShutdownTask::start(
                        vec![resources],
                        || Ok(()),
                        Arc::new(move || context.request_repaint_of(egui::ViewportId::ROOT)),
                    )
                    .map_err(|e| ApiError::new("cleanup_failed", e.to_string()))?;
                    self.control_closers.push(task);
                }
                if *restart {
                    self.start_session(i, ctx);
                    if self.sessions[i].backend.is_none() {
                        return Err(ApiError::new(
                            "launch_failed",
                            self.sessions[i].error.clone().unwrap_or_default(),
                        ));
                    }
                }
                self.sync_visibility();
                ctx.request_repaint();
                Ok(self.control_session(i))
            }
            Action::SessionAcknowledge { id } => {
                let i = self.control_index(id, caller)?;
                self.sessions[i].status.acknowledge();
                ctx.request_repaint();
                Ok(self.control_session(i))
            }
            Action::Quit { force } => {
                if !force && self.sessions.iter().any(|s| s.backend.is_some()) {
                    return Err(ApiError::new(
                        "session_running",
                        "Quitting stops this manager's owned terminals; use --force",
                    ));
                }
                self.begin_shutdown(ctx);
                Ok(json!({ "shutdown":"scheduled" } ))
            }
        }
    }
    fn native_git_show(
        &self,
        section: GitSection,
        offset: usize,
        limit: usize,
    ) -> ApiResult<Value> {
        let git = self
            .git
            .as_ref()
            .ok_or_else(|| ApiError::new("unavailable", "Git is unavailable"))?;
        let mut value = json!({ "loading":git.loading,"not_repository":git.not_repository,"error":git.error.as_ref().map(read_error) } );
        match section {
            GitSection::Status => {
                if let Some(s) = &git.snapshot {
                    value["status"] = json!({ "root":s.root,"branch":s.branch,"detached":s.detached,"unborn":s.unborn,"added":s.added,"deleted":s.deleted,"total_files":s.total_files,"partial":s.partial,"omitted_files":s.omitted_files,"more_history":s.more } );
                }
            }
            GitSection::Files => {
                if let Some(s) = &git.snapshot {
                    value["files"] = page(&s.files, offset, limit)?;
                    value["partial"] = json!(s.partial);
                    value["omitted_files"] = json!(s.omitted_files);
                }
            }
            GitSection::History => {
                if let Some(s) = &git.snapshot {
                    value["commits"] = page(&s.commits, offset, limit)?;
                    value["more_history"] = json!(s.more);
                }
            }
            GitSection::Commit => {
                if let Some(i) = &git.inspection {
                    value["loading"] = json!(i.loading);
                    value["error"] = json!(i.error.as_ref().map(read_error));
                    if let Some(d) = &i.data {
                        value["commit"] = json!({ "id":d.id,"message":d.message.chars().take(32_768).collect::<String>(),"message_truncated":d.message.chars().count()>32_768,"parents":d.parents,"files":page(&d.files,offset,limit)?,"total_files":d.total_files,"added":d.added,"deleted":d.deleted,"partial":d.partial } );
                    }
                }
            }
            GitSection::Diff => {
                if let Some(p) = &git.patch {
                    value["loading"] = json!(p.loading);
                    value["error"] = json!(p.error.as_ref().map(read_error));
                    value["path"] = json!(p.path);
                    value["source"] = json!(p.source);
                    if let Some(d) = &p.data {
                        value["lines"] = page(&d.lines, offset, limit)?;
                        value["binary"] = json!(d.binary);
                        value["partial"] = json!(d.partial);
                    }
                }
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::*;
    #[test]
    fn response_page_uses_byte_budget_for_wide_text() {
        let lines = vec!["中".repeat(4000); 200];
        let result = page(&lines, 0, 200).unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() < 270_000);
        let next = result["next_offset"].as_u64().unwrap() as usize;
        assert!(next > 0 && next < 200);
        assert_eq!(page(&lines, next, 200).unwrap()["offset"], next);
    }
    #[test]
    fn capture_encodes_png_without_overwriting_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("capture.png");
        let image = egui::ColorImage::filled([3, 2], egui::Color32::RED);
        save_capture(&path, &image).unwrap();
        let original = std::fs::read(&path).unwrap();
        assert_eq!(image::load_from_memory(&original).unwrap().width(), 3);
        assert!(save_capture(&path, &image).is_err());
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}
