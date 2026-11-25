use super::*;
use ai_terminal::control::{self, ApiError, ApiResult, Command, Request, Response};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[path = "control/native.rs"]
pub(super) mod native;

impl App {
    pub(super) fn session_panel_states(&self) -> BTreeMap<String, model::ProjectPanel> {
        self.sessions
            .iter()
            .map(|s| {
                let panel = if Some(&s.spec.id) == self.active.as_ref() {
                    model::ProjectPanel::from_settings(self.preferences)
                } else {
                    s.project_panel
                };
                (s.spec.id.clone(), panel)
            })
            .collect()
    }
    fn workspace_revision(&self) -> String {
        control::revision(&self.workspace_snapshot())
    }
    fn control_session(&self, index: usize) -> Value {
        let session = &self.sessions[index];
        json!({
            "id": session.spec.id, "name": session.spec.name, "directory": session.spec.directory,
            "shell": match session.spec.agent { Agent::Shell => "powershell", Agent::Cmd => "cmd", Agent::Bash => "bash", _ => "powershell" },
            "color": session.spec.color, "group_id": model::session_group(&self.groups, &session.spec.id).map(|g| &g.id),
            "index": index, "active": self.active.as_deref() == Some(&session.spec.id),
            "project_panel": if self.active.as_deref() == Some(&session.spec.id) {
                model::ProjectPanel::from_settings(self.preferences)
            } else { session.project_panel },
            "running": session.backend.is_some(), "agent": session.display_agent().label(),
            "process_id": session.backend.as_ref().map(|b| b.pty_id()),
            "activity": match session.status.activity { Activity::Unknown => "unknown", Activity::Idle => "idle", Activity::Completed => "completed", Activity::Working => "working", Activity::Failed => "failed", Activity::Waiting => "waiting", Activity::Exited => "exited" },
            "unread": session.status.unread, "error": session.error,
            "question_unread": session.status.question_unread,
        })
    }
    fn control_groups(&self) -> Vec<Value> {
        self.groups.iter().map(|g| json!({"id": g.id, "name": g.name, "members": self.sessions.iter().filter(|s| g.members.contains(&s.spec.id)).map(|s| &s.spec.id).collect::<Vec<_>>() })).collect()
    }
    fn control_index(&self, id: &str, caller: Option<&str>) -> ApiResult<usize> {
        let id = control::resolve(id, caller, &BTreeMap::new())?;
        self.sessions
            .iter()
            .position(|s| s.spec.id == id)
            .ok_or_else(|| ApiError::new("not_found", "Session not found"))
    }
    fn control_save(&self, workspace: &Workspace) -> ApiResult<()> {
        if self.load_failed {
            return Err(ApiError::new(
                "read_only",
                "Workspace failed to load; original data must be preserved",
            ));
        }
        model::save(&self.directory.join("sessions.json"), workspace)
            .map_err(|e| ApiError::new("save_failed", e))
    }
    fn control_command(&mut self, request: &Request, ctx: &egui::Context) -> ApiResult<Value> {
        let caller = request.caller_session.as_deref();
        match &request.action {
            Command::Native { action } => self.control_native(action, caller, ctx),
            Command::Info => {
                let d = &self
                    .control
                    .as_ref()
                    .ok_or_else(|| ApiError::new("unavailable", "CLI is disabled"))?
                    .descriptor;
                Ok(
                    json!({"instance_id": d.instance_id, "pid": d.pid, "version": d.version, "data_directory": d.data_directory, "caller_session": caller, "revision": self.workspace_revision(), "schema_version": control::SCHEMA_VERSION, "capabilities": {"agent_resume":true,"native_controls":true, "settings":true,"window":true,"floating":true,"panels":true,"files":true,"git_readonly":true,"media":true,"terminal_io":true,"session_lifecycle":true,"capture":true}}),
                )
            }
            Command::Receipt { id } => self
                .control_receipts
                .iter()
                .find(|r| r.0 == *id)
                .map(|r| serde_json::to_value(&r.2).unwrap())
                .ok_or_else(|| {
                    ApiError::new(
                        "not_found",
                        "Receipt expired or unknown; inspect the workspace before retrying",
                    )
                }),
            Command::SessionList { offset, limit } => {
                if *limit == 0 || *limit > 200 {
                    return Err(ApiError::new(
                        "invalid_argument",
                        "limit must be between 1 and 200",
                    ));
                }
                let sessions: Vec<_> = (*offset..self.sessions.len())
                    .take(*limit)
                    .map(|i| self.control_session(i))
                    .collect();
                let next = offset.saturating_add(sessions.len());
                Ok(
                    json!({"sessions": sessions, "total": self.sessions.len(), "offset": offset, "next_offset": (next < self.sessions.len()).then_some(next), "revision": self.workspace_revision()}),
                )
            }
            Command::SessionShow { id } => {
                Ok(self.control_session(self.control_index(id, caller)?))
            }
            Command::GroupList => {
                Ok(json!({"groups": self.control_groups(), "revision": self.workspace_revision()}))
            }
            Command::WorkspaceShow => Ok(
                json!({"sessions": (0..self.sessions.len()).map(|i| self.control_session(i)).collect::<Vec<_>>(), "groups": self.control_groups(), "active": self.active, "caller_session": caller, "revision": self.workspace_revision()}),
            ),
            Command::Apply {
                operations,
                dry_run,
                if_revision,
            } => {
                let workspace = self.workspace_snapshot();
                let revision = control::revision(&workspace);
                let prepared =
                    control::prepare(&workspace, operations, caller, if_revision.as_deref())?;
                let after = control::revision(&prepared.workspace);
                if !dry_run {
                    self.control_save(&prepared.workspace)?;
                    let mut existing: BTreeMap<_, _> = std::mem::take(&mut self.sessions)
                        .into_iter()
                        .map(|s| (s.spec.id.clone(), s))
                        .collect();
                    self.sessions = prepared
                        .workspace
                        .sessions
                        .into_iter()
                        .map(|spec| {
                            let unnamed = prepared.workspace.unnamed_sessions.contains(&spec.id);
                            if let Some(mut live) = existing.remove(&spec.id) {
                                live.project_panel = prepared
                                    .workspace
                                    .session_panels
                                    .get(&spec.id)
                                    .copied()
                                    .unwrap_or_default();
                                live.spec = spec;
                                live.unnamed = unnamed;
                                live
                            } else {
                                Session::new(spec)
                            }
                        })
                        .collect();
                    debug_assert!(
                        existing.is_empty(),
                        "Metadata operations cannot remove sessions"
                    );
                    self.groups = prepared.workspace.groups;
                    self.dirty = false;
                    self.save_error = None;
                    self.sync_visibility();
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                }
                Ok(
                    json!({"dry_run": dry_run, "revision_before": revision, "revision": after, "results": prepared.results}),
                )
            }
            Command::Start { id, activate }
            | Command::Run { id, activate, .. }
            | Command::Resume { id, activate, .. } => {
                let i = self.control_index(id, caller)?;
                let id = self.sessions[i].spec.id.clone();
                let launch_agent = match &request.action {
                    Command::Run { agent, .. } => Some((*agent, Vec::new())),
                    Command::Resume {
                        agent,
                        conversation_id,
                        ..
                    } => Some((*agent, control::resume_arguments(*agent, conversation_id)?)),
                    _ => None,
                };
                if let Some((agent, arguments)) = launch_agent {
                    if agent.is_shell() {
                        return Err(ApiError::new(
                            "invalid_argument",
                            "Use session start for a Shell",
                        ));
                    }
                    if self.sessions[i].backend.is_some() {
                        return Err(ApiError::new(
                            "session_running",
                            "Agent launch requires an unstarted session; do not send a command into a live terminal",
                        ));
                    }
                    self.start_session_with_agent_arguments(i, ctx, Some(agent), arguments);
                } else {
                    self.start_session(i, ctx);
                }
                if self.sessions[i].backend.is_none() {
                    return Err(ApiError::new(
                        "launch_failed",
                        self.sessions[i]
                            .error
                            .clone()
                            .unwrap_or_else(|| "Terminal launch failed".into()),
                    ));
                }
                if *activate {
                    self.activate(id, ctx);
                    ai_terminal::floating::restore_main(ctx);
                }
                let mut result = json!({"session": self.control_session(i), "launched": true, "readiness": "process_started"});
                if let Command::Resume {
                    agent,
                    conversation_id,
                    ..
                } = &request.action
                {
                    result["resume_requested"] =
                        json!({"agent": agent, "conversation_id": conversation_id});
                }
                Ok(result)
            }
            Command::Close { id, force } => {
                if self.control_closers.len() >= 32 {
                    return Err(ApiError::new("busy", "Session cleanup queue is full"));
                }
                let i = self.control_index(id, caller)?;
                if self.sessions[i].backend.is_some() && !force {
                    return Err(ApiError::new(
                        "session_running",
                        "Closing this session stops its owned process tree; repeat with --force if intended",
                    ));
                }
                let id = self.sessions[i].spec.id.clone();
                let mut next = self.workspace_snapshot();
                next.sessions.retain(|s| s.id != id);
                next.session_panels.remove(&id);
                next.unnamed_sessions.remove(&id);
                model::assign_group(&mut next.groups, &id, None);
                if next.active.as_deref() == Some(&id) {
                    next.active = next.sessions.first().map(|s| s.id.clone());
                }
                self.control_save(&next)?;
                let was_active = self.active.as_deref() == Some(&id);
                let mut cleanup_error = None;
                if let Some(resources) = self.sessions[i].take_resources() {
                    let context = ctx.clone();
                    match crate::shutdown::ShutdownTask::start(
                        vec![resources],
                        || Ok(()),
                        Arc::new(move || context.request_repaint_of(egui::ViewportId::ROOT)),
                    ) {
                        Ok(task) => self.control_closers.push(task),
                        Err(e) => cleanup_error = Some(e.to_string()),
                    }
                }
                self.sessions.remove(i);
                self.groups = next.groups;
                // Keep the previous active ID until activate restores the next
                // session's panel rather than caching the closed one's panel.
                self.rename = self.rename.take().filter(|v| *v != id);
                self.close_session = self.close_session.take().filter(|v| *v != id);
                self.dirty = false;
                if was_active {
                    if let Some(next) = next.active {
                        self.activate(next, ctx);
                    } else {
                        self.clear_project_preview(ctx);
                        self.active = None;
                        model::ProjectPanel::Closed.apply(&mut self.preferences);
                    }
                }
                self.sync_visibility();
                Ok(
                    json!({"id": id, "closed": true, "process_cleanup": "scheduled", "cleanup_error": cleanup_error}),
                )
            }
        }
    }
    pub(super) fn poll_control(&mut self, ctx: &egui::Context) {
        self.control_capture.poll(ctx);
        self.control_closers.retain(|task| task.poll().is_none());
        let Some(server) = &self.control else {
            return;
        };
        let instance = server.descriptor.instance_id.clone();
        let pending: Vec<_> = server.receiver.try_iter().take(8).collect();
        let filled = pending.len() == 8;
        for pending in pending {
            let request = pending.request;
            let id = request.request_id.clone();
            let bytes = match control::actions::fingerprint(&serde_json::to_vec(&request).unwrap())
            {
                Ok(bytes) => bytes,
                Err(error) => {
                    let _ = pending
                        .reply
                        .send(Response::new(&id, &instance, Err(error)));
                    continue;
                }
            };
            let response = if let Some((_, previous, response)) =
                self.control_receipts.iter().find(|r| r.0 == id)
            {
                if previous == &bytes {
                    response.clone()
                } else {
                    Response::new(
                        &id,
                        &instance,
                        Err(ApiError::new(
                            "request_id_reused",
                            "Request ID already belongs to a different command",
                        )),
                    )
                }
            } else {
                let result = if pending.expires <= Instant::now() {
                    Err(ApiError::new(
                        "busy",
                        "Request expired before execution; no change was made",
                    ))
                } else if uuid::Uuid::parse_str(&id).is_err() {
                    Err(ApiError::new(
                        "invalid_argument",
                        "request_id must be a UUID",
                    ))
                } else if self.closing.is_some() || self.shutdown_saved {
                    Err(ApiError::new("busy", "Manager is shutting down"))
                } else {
                    self.control_command(&request, ctx)
                };
                let response = Response::new(&id, &instance, result);
                if matches!(
                    request.action,
                    Command::Apply { dry_run: false, .. }
                        | Command::Start { .. }
                        | Command::Run { .. }
                        | Command::Resume { .. }
                        | Command::Close { .. }
                ) || matches!(&request.action, Command::Native { action } if action.cache_receipt())
                {
                    self.control_receipts
                        .push_back((id, bytes, response.clone()));
                }
                while self.control_receipts.len() > 64
                    || self
                        .control_receipts
                        .iter()
                        .map(|r| r.1.len() + serde_json::to_vec(&r.2).unwrap().len())
                        .sum::<usize>()
                        > 4_194_304
                {
                    self.control_receipts.pop_front();
                }
                response
            };
            let _ = pending.reply.send(response);
        }
        if filled {
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use control::actions::{Action, SettingsPatch};
    use control::{Operation, Shell};

    #[test]
    fn project_panels_follow_sessions_and_survive_save_and_reload() {
        use control::actions::Panel;
        let (dir, ctx, mut app) = app();
        app.bridge = None; // Navigation only; this test creates no PTY.
        let mut ids = Vec::new();
        for _ in 0..3 {
            let result = app
                .control_command(&request(create(dir.path())), &ctx)
                .unwrap();
            ids.push(result["results"][0]["id"].as_str().unwrap().to_owned());
        }
        app.activate(ids[0].clone(), &ctx);
        app.control_native(
            &Action::Panel {
                panel: Panel::Files,
                open: true,
            },
            None,
            &ctx,
        )
        .unwrap();
        app.activate(ids[1].clone(), &ctx);
        assert!(!app.preferences.project_files_open && !app.preferences.git_panel_open);
        app.control_native(
            &Action::Panel {
                panel: Panel::Git,
                open: true,
            },
            None,
            &ctx,
        )
        .unwrap();
        app.activate(ids[2].clone(), &ctx);
        assert!(!app.preferences.project_files_open && !app.preferences.git_panel_open);
        app.activate(ids[0].clone(), &ctx);
        assert!(app.preferences.project_files_open && !app.preferences.git_panel_open);
        app.activate(ids[1].clone(), &ctx);
        assert!(!app.preferences.project_files_open && app.preferences.git_panel_open);
        // Metadata changes must not replace live per-session navigation state.
        app.control_command(
            &request(Command::Apply {
                operations: vec![Operation::Rename {
                    id: ids[0].clone(),
                    name: "renamed".into(),
                }],
                dry_run: false,
                if_revision: None,
            }),
            &ctx,
        )
        .unwrap();
        app.activate(ids[0].clone(), &ctx);
        assert!(app.preferences.project_files_open);
        app.dirty = true;
        app.persist();
        let saved = model::load(&dir.path().join("sessions.json")).unwrap();
        assert_eq!(saved.session_panels[&ids[0]], model::ProjectPanel::Files);
        assert_eq!(saved.session_panels[&ids[1]], model::ProjectPanel::Git);
        assert_eq!(saved.session_panels[&ids[2]], model::ProjectPanel::Closed);
        assert!(app.sessions.iter().all(|s| s.backend.is_none()));
        drop(app);
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut restored = App::new(&creation, dir.path().into(), false, false);
        restored.bridge = None;
        assert!(restored.preferences.project_files_open);
        restored.activate(ids[1].clone(), &ctx);
        assert!(restored.preferences.git_panel_open && !restored.preferences.project_files_open);
    }

    #[test]
    fn native_settings_are_atomic_and_receipts_do_not_retain_terminal_input() {
        let (dir, ctx, mut app) = app();
        let r = request(Command::Native {
            action: Action::SettingsSet {
                patch: SettingsPatch {
                    floating_enabled: Some(true),
                    floating_transparency_percent: Some(50.),
                    ..Default::default()
                },
            },
        });
        let first = rpc(&mut app, &ctx, r.clone());
        assert!(first.ok, "{:?}", first.error);
        assert_eq!(app.preferences.floating_opacity, 0.5);
        assert_eq!(
            model::load(&dir.path().join("sessions.json"))
                .unwrap()
                .ui
                .floating_opacity,
            0.5
        );
        assert_eq!(rpc(&mut app, &ctx, r).result, first.result);
        let private = "PRIVATE_INPUT_NEVER_RETAINED_测试";
        let write = request(Command::Native {
            action: Action::TerminalWrite {
                id: uuid::Uuid::new_v4().to_string(),
                text: private.into(),
                submit: false,
            },
        });
        assert!(!rpc(&mut app, &ctx, write).ok);
        assert!(
            app.control_receipts
                .iter()
                .all(|(_, digest, response)| digest.len() == 32
                    && !serde_json::to_string(response).unwrap().contains(private))
        );
        let before = app.control_receipts.len();
        assert!(
            rpc(
                &mut app,
                &ctx,
                request(Command::Native {
                    action: Action::FileShow {
                        content: true,
                        offset: 0,
                        limit: 1
                    }
                })
            )
            .ok
        );
        assert_eq!(before, app.control_receipts.len());
        std::fs::remove_file(dir.path().join("sessions.json")).unwrap();
        std::fs::create_dir(dir.path().join("sessions.json")).unwrap();
        let error = app
            .control_command(
                &request(Command::Native {
                    action: Action::SettingsSet {
                        patch: SettingsPatch {
                            floating_transparency_percent: Some(0.),
                            ..Default::default()
                        },
                    },
                }),
                &ctx,
            )
            .unwrap_err();
        assert_eq!(error.code, "save_failed");
        assert_eq!(app.preferences.floating_opacity, 0.5);
    }
    #[test]
    fn native_file_reads_are_paged_and_cannot_escape_the_project() {
        let (dir, ctx, mut app) = app();
        std::fs::write(dir.path().join("sample.md"), "# Hello\n你好 😀\nthird\n").unwrap();
        app.project_files.set_root(Some(dir.path()));
        assert!(
            app.control_native(
                &Action::FileOpen {
                    path: "../secret".into()
                },
                None,
                &ctx
            )
            .is_err()
        );
        app.control_native(
            &Action::FileOpen {
                path: "sample.md".into(),
            },
            None,
            &ctx,
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.project_files.file.as_ref().unwrap().loading {
            app.project_files.poll();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        let first = app
            .control_native(
                &Action::FileShow {
                    content: true,
                    offset: 0,
                    limit: 1,
                },
                None,
                &ctx,
            )
            .unwrap();
        assert_eq!(first["content"]["items"].as_array().unwrap().len(), 1);
        assert_eq!(first["content"]["next_offset"], 1);
        app.control_native(&Action::FileMode { source: true }, None, &ctx)
            .unwrap();
        app.control_native(&Action::FileClose, None, &ctx).unwrap();
        assert!(app.project_files.file.is_none());
    }

    fn app() -> (tempfile::TempDir, egui::Context, App) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&cc, dir.path().into(), false, false);
        app.startup_pending = false;
        (dir, ctx, app)
    }
    fn request(action: Command) -> Request {
        Request {
            schema_version: control::SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4().to_string(),
            caller_session: None,
            action,
        }
    }
    fn create(directory: &std::path::Path) -> Command {
        Command::Apply {
            operations: vec![Operation::Create {
                key: None,
                name: "后端 🛠️".into(),
                directory: directory.into(),
                shell: Shell::Powershell,
                color: None,
            }],
            dry_run: false,
            if_revision: None,
        }
    }
    fn rpc(app: &mut App, ctx: &egui::Context, r: Request) -> Response {
        let descriptor = app.control.as_ref().unwrap().descriptor.clone();
        let client = std::thread::spawn(move || control::call(&descriptor, r).unwrap());
        let deadline = Instant::now() + Duration::from_secs(4);
        while !client.is_finished() && Instant::now() < deadline {
            app.poll_control(ctx);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(client.is_finished(), "control round trip exceeded deadline");
        client.join().unwrap()
    }
    #[test]
    fn rpc_duplicate_receipt_never_creates_a_second_session() {
        let (dir, ctx, mut app) = app();
        let r = request(create(dir.path()));
        let first = rpc(&mut app, &ctx, r.clone());
        assert!(first.ok, "{:?}", first.error);
        let again = rpc(&mut app, &ctx, r.clone());
        assert_eq!(first.result, again.result);
        assert_eq!(app.sessions.len(), 1);
        let mut different = r;
        different.action = create(&dir.path().join("missing"));
        assert_eq!(
            rpc(&mut app, &ctx, different).error.unwrap().code,
            "request_id_reused"
        );
        let receipt = rpc(
            &mut app,
            &ctx,
            request(Command::Receipt {
                id: first.request_id.clone(),
            }),
        );
        assert_eq!(receipt.result.unwrap()["request_id"], first.request_id);
        assert_eq!(
            model::load(&dir.path().join("sessions.json"))
                .unwrap()
                .sessions
                .len(),
            1
        );
        let public = serde_json::to_string(&rpc(&mut app, &ctx, request(Command::Info))).unwrap();
        assert!(!public.contains(&app.control.as_ref().unwrap().descriptor.token));
    }
    #[test]
    fn dry_run_and_save_failure_do_not_modify_live_workspace() {
        let (dir, ctx, mut app) = app();
        let mut r = request(create(dir.path()));
        if let Command::Apply { dry_run, .. } = &mut r.action {
            *dry_run = true;
        }
        assert!(app.control_command(&r, &ctx).is_ok());
        assert!(app.sessions.is_empty());
        assert!(!dir.path().join("sessions.json").exists());
        std::fs::create_dir(dir.path().join("sessions.json")).unwrap();
        assert_eq!(
            app.control_command(&request(create(dir.path())), &ctx)
                .err()
                .unwrap()
                .code,
            "save_failed"
        );
        assert!(app.sessions.is_empty());
    }
    #[test]
    fn live_pty_identity_survives_batch_rename_color_group_and_move() {
        let (dir, ctx, mut app) = app();
        app.control_command(&request(create(dir.path())), &ctx)
            .unwrap();
        app.control_command(&request(create(dir.path())), &ctx)
            .unwrap();
        app.helper = Ok(std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ai-terminal-hook.exe"));
        app.start_session(0, &ctx);
        let pid = app.sessions[0]
            .backend
            .as_ref()
            .expect("build --bins before native tests")
            .pty_id();
        let id = app.sessions[0].spec.id.clone();
        let second = app.sessions[1].spec.id.clone();
        app.active = Some(second.clone());
        let mut r = request(Command::Apply {
            operations: vec![
                Operation::Rename {
                    id: "current".into(),
                    name: "协调 ❤️".into(),
                },
                Operation::Color {
                    id: id.clone(),
                    color: Some(model::SessionColor::Blue),
                },
                Operation::GroupCreate {
                    key: None,
                    name: "团队".into(),
                    members: vec![id.clone(), second.clone()],
                },
                Operation::Move {
                    id: id.clone(),
                    before: None,
                    after: Some(second.clone()),
                },
            ],
            dry_run: false,
            if_revision: None,
        });
        r.caller_session = Some(id.clone());
        app.control_command(&r, &ctx).unwrap();
        assert_eq!(app.sessions[1].spec.name, "协调 ❤️");
        assert_eq!(app.sessions[1].backend.as_ref().unwrap().pty_id(), pid);
        assert_eq!(app.active.as_deref(), Some(second.as_str()));
        assert_eq!(
            app.control_command(
                &request(Command::Run {
                    id: id.clone(),
                    agent: Agent::Codex,
                    activate: false
                }),
                &ctx
            )
            .err()
            .unwrap()
            .code,
            "session_running"
        );
        assert_eq!(
            app.control_command(
                &request(Command::Resume {
                    id: id.clone(),
                    agent: Agent::Codex,
                    conversation_id: "a1234567-1234-4234-8234-123456789abc".into(),
                    activate: false,
                }),
                &ctx,
            )
            .err()
            .unwrap()
            .code,
            "session_running"
        );
        assert_eq!(app.sessions[1].backend.as_ref().unwrap().pty_id(), pid);
        assert_eq!(
            app.control_command(&request(Command::Close { id, force: false }), &ctx)
                .err()
                .unwrap()
                .code,
            "session_running"
        );
        app.sessions[1].shutdown();
    }
}
