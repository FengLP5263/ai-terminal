use ai_terminal::{
    control::{self, Command, Operation, Request, Response, SCHEMA_VERSION},
    model::{Agent, SessionSpec, Workspace},
};
use serde_json::json;
use std::{sync::Arc, time::Duration};

fn workspace(root: &std::path::Path) -> Workspace {
    Workspace {
        sessions: vec![SessionSpec {
            id: "leader".into(),
            name: "主会话 ❤️".into(),
            directory: root.into(),
            agent: Agent::Shell,
            agent_session_id: Some("preserved-history".into()),
            color: None,
        }],
        active: Some("leader".into()),
        ..Default::default()
    }
}
fn ops(value: serde_json::Value) -> Vec<Operation> {
    serde_json::from_value(value).unwrap()
}

#[test]
fn batch_builds_team_with_current_references_and_global_order() {
    let dir = tempfile::tempdir().unwrap();
    let original = workspace(dir.path());
    let plan = ops(json!([
        {"op":"session.create","key":"frontend","name":"前端 ✨","directory":dir.path(),"color":"blue"},
        {"op":"session.create","key":"backend","name":"后端","directory":dir.path(),"shell":"cmd"},
        {"op":"group.create","key":"team","name":"研发组","members":["$current","$frontend","$backend"]},
        {"op":"session.move","id":"$backend","before":"$frontend"},
        {"op":"session.rename","id":"$current","name":"协调员"}
    ]));
    let p = control::prepare(
        &original,
        &plan,
        Some("leader"),
        Some(&control::revision(&original)),
    )
    .unwrap();
    assert_eq!(original.sessions.len(), 1);
    assert_eq!(
        p.workspace
            .sessions
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["协调员", "后端", "前端 ✨"]
    );
    assert_eq!(
        p.workspace.sessions[0].agent_session_id.as_deref(),
        Some("preserved-history")
    );
    assert_eq!(p.workspace.groups[0].members.len(), 3);
    assert_eq!(p.workspace.active.as_deref(), Some("leader"));
}

#[test]
fn failed_tail_and_conflict_leave_original_workspace_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let before = serde_json::to_vec(&w).unwrap();
    let plan = ops(json!([
        {"op":"session.create","name":"不会留下半组","directory":dir.path()},
        {"op":"session.rename","id":"missing","name":"错误"}
    ]));
    assert_eq!(
        control::prepare(&w, &plan, None, None).err().unwrap().code,
        "not_found"
    );
    assert_eq!(serde_json::to_vec(&w).unwrap(), before);
    assert_eq!(
        control::prepare(&w, &plan, None, Some("stale"))
            .err()
            .unwrap()
            .code,
        "conflict"
    );
}

#[test]
fn rejects_bad_names_keys_member_duplicates_and_ambiguous_moves() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    for plan in [
        json!([{"op":"session.rename","id":"leader","name":"  "}]),
        json!([{"op":"session.rename","id":"leader","name":"bad\nname"}]),
        json!([{"op":"group.create","name":"x","members":[]}]),
        json!([{"op":"group.create","name":"x","members":["leader","leader"]}]),
        json!([{"op":"session.move","id":"leader","before":"leader","after":"leader"}]),
        json!([{"op":"session.create","key":"current","name":"x","directory":dir.path()}]),
        json!([{"op":"session.rename","id":"$future","name":"x"}]),
        json!([{"op":"session.create","name":"x","directory":"relative"}]),
    ] {
        assert!(control::prepare(&w, &ops(plan), None, None).is_err());
    }
    assert!(serde_json::from_value::<Operation>(json!({"op":"session.create","name":"x","directory":dir.path(),"prompt":"never transport this"})).is_err());
}

#[test]
fn group_move_and_dissolve_keep_sessions_and_legacy_data() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let p = control::prepare(
        &w,
        &ops(json!([
            {"op":"group.create","key":"old","name":"旧组","members":["leader"]},
            {"op":"group.create","key":"new","name":"新组","members":["leader"]},
            {"op":"group.dissolve","id":"$new"}
        ])),
        None,
        None,
    )
    .unwrap();
    assert!(p.workspace.groups.is_empty());
    assert_eq!(p.workspace.sessions.len(), 1);
    assert_eq!(
        p.workspace.sessions[0].agent_session_id,
        w.sessions[0].agent_session_id
    );
    assert!(
        control::prepare(
            &w,
            &ops(json!([{"op":"session.rename","id":"current","name":"x"}])),
            None,
            None
        )
        .is_err()
    );
    assert!(control::prepare(&w, &[], None, None).is_err());
    let too_many = vec![
        Operation::Rename {
            id: "leader".into(),
            name: "x".into()
        };
        129
    ];
    assert!(control::prepare(&w, &too_many, None, None).is_err());
}

#[test]
fn cli_parses_unicode_and_rejects_extra_or_conflicting_options() {
    fn parse(a: &[&str]) -> control::ApiResult<control::cli::Parsed> {
        control::cli::parse(a.iter().map(|s| (*s).into()).collect())
    }
    assert!(matches!(
        parse(&["session", "run", "current", "--agent", "qoder", "--json"])
            .unwrap()
            .command,
        Command::Run {
            agent: Agent::Qoder,
            ..
        }
    ));
    assert!(parse(&["session", "close", "current", "--force"]).is_ok());
    for a in [
        vec!["session", "rename", "x", "--name", "一", "--name", "二"],
        vec![
            "session", "run", "x", "--agent", "codex", "--prompt", "secret",
        ],
        vec!["session", "close", "x", "extra"],
        vec!["group", "list", "--typo", "x"],
        vec!["instance", "info", "--request-id", "not-uuid"],
    ] {
        assert!(parse(&a).is_err());
    }
}

#[test]
fn authenticated_transport_round_trip_wakes_without_gui_and_cleans_descriptor() {
    let dir = tempfile::tempdir().unwrap();
    let (wake_tx, wake_rx) = std::sync::mpsc::channel();
    let server = control::ControlServer::start(
        dir.path(),
        Arc::new(move || {
            let _ = wake_tx.send(());
        }),
    )
    .unwrap();
    let descriptor = control::read_descriptor(dir.path()).unwrap();
    let stored = std::fs::read_to_string(dir.path().join("runtime/control.json")).unwrap();
    assert!(
        !stored.contains(&descriptor.token),
        "capability must be encrypted at rest"
    );
    assert_eq!(descriptor.instance_id, server.descriptor.instance_id);
    let client = std::thread::spawn(move || {
        control::call(
            &descriptor,
            Request {
                schema_version: 1,
                request_id: uuid::Uuid::new_v4().to_string(),
                caller_session: None,
                action: Command::Info,
            },
        )
        .unwrap()
    });
    wake_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let pending = server
        .receiver
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    pending
        .reply
        .send(Response::new(
            &pending.request.request_id,
            &server.descriptor.instance_id,
            Ok(json!({"ready":true})),
        ))
        .unwrap();
    assert!(client.join().unwrap().ok);
    #[cfg(windows)]
    assert!(control::ControlServer::start(dir.path(), Arc::new(|| {})).is_err());
    drop(server);
    assert!(control::read_descriptor(dir.path()).is_err());
}

#[test]
fn wrong_capability_never_reaches_manager_queue_and_foreign_endpoint_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let server = control::ControlServer::start(dir.path(), Arc::new(|| {})).unwrap();
    let copied = tempfile::tempdir().unwrap();
    std::fs::create_dir(copied.path().join("runtime")).unwrap();
    std::fs::copy(
        dir.path().join("runtime/control.json"),
        copied.path().join("runtime/control.json"),
    )
    .unwrap();
    assert_eq!(
        control::read_descriptor(copied.path()).err().unwrap().code,
        "wrong_workspace"
    );
    let mut descriptor = server.descriptor.clone();
    descriptor.token = "incorrect".into();
    let request = Request {
        schema_version: SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4().to_string(),
        caller_session: None,
        action: Command::Info,
    };
    assert!(control::call(&descriptor, request.clone()).is_err());
    assert!(server.receiver.try_recv().is_err());
    descriptor.endpoint = "192.0.2.1:12345".into();
    assert!(control::call(&descriptor, request).is_err());
}

#[test]
fn actual_console_binary_reports_utf8_json_and_meaningful_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ai-terminal-cli"))
        .args([
            "--data-dir",
            dir.path().to_str().unwrap(),
            "session",
            "list",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["error"]["code"], "not_running");
    assert!(!dir.path().join("sessions.json").exists());
    assert!(output.stderr.is_empty());
}

#[cfg(windows)]
#[test]
fn one_new_gui_writer_per_directory_but_independent_workspaces_are_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let lease = control::ManagerLease::acquire(dir.path()).unwrap();
    assert!(control::ManagerLease::acquire(dir.path()).is_err());
    let other = control::ManagerLease::acquire(second.path()).unwrap();
    drop(lease);
    let again = control::ManagerLease::acquire(dir.path()).unwrap();
    drop((again, other));
}
