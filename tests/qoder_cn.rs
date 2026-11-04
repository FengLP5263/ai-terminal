use ai_terminal::{
    discovery::CliSearch,
    launch,
    model::{Agent, SessionSpec, Workspace},
};
use std::path::Path;

#[test]
fn qoder_cn_finds_both_public_names_and_prefers_native_in_the_same_directory() {
    let root = tempfile::tempdir().unwrap();
    let search = CliSearch::from_directories([root.path().into()]);
    std::fs::write(root.path().join("qoder.exe"), "IDE, not the CN CLI").unwrap();
    assert!(search.find(Agent::QoderCn).is_none());
    std::fs::write(root.path().join("qodercn.cmd"), "@echo off").unwrap();
    assert_eq!(
        search.find(Agent::QoderCn),
        Some(root.path().join("qodercn.cmd"))
    );
    std::fs::write(root.path().join("qoderclicn.exe"), "native fixture").unwrap();
    assert_eq!(
        search.find(Agent::QoderCn),
        Some(root.path().join("qoderclicn.exe"))
    );
    std::fs::write(root.path().join("qodercn.exe"), "native alias fixture").unwrap();
    assert_eq!(
        search.find(Agent::QoderCn),
        Some(root.path().join("qodercn.exe"))
    );
}

#[test]
fn qoder_cn_preserves_directory_priority_and_rejects_stale_npm_launchers() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    let stale = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n)\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@qodercn-ai\\qoderclicn\\cli.js\" %*\r\n";
    std::fs::write(first.join("qodercn.cmd"), stale).unwrap();
    std::fs::write(second.join("qoderclicn.exe"), "fixture").unwrap();
    let search = CliSearch::from_directories([first.clone(), second.clone()]);
    assert_eq!(
        search.find(Agent::QoderCn),
        Some(second.join("qoderclicn.exe"))
    );
    std::fs::write(first.join("qoderclicn.bat"), "@echo off").unwrap();
    assert_eq!(
        search.find(Agent::QoderCn),
        Some(first.join("qoderclicn.bat"))
    );
}

#[test]
fn qoder_cn_injects_only_advisory_session_hooks_and_keeps_resume_arguments() {
    let root = tempfile::tempdir().unwrap();
    let runtime = root.path().join("运行目录 with space & quote'");
    let helper = root
        .path()
        .join("helper with space & quote'/ai-terminal-hook.exe");
    let spec = SessionSpec {
        id: "qoder-cn".into(),
        name: "Qoder CN".into(),
        directory: root.path().into(),
        agent: Agent::QoderCn,
        agent_session_id: Some("qoder-history-id".into()),
        color: None,
    };
    let args = launch::agent_arguments(&spec, &runtime, &helper).unwrap();
    assert_eq!(
        args,
        [
            "--settings",
            runtime.join("qoder-cn-qoder-cn.json").to_str().unwrap(),
            "--resume",
            "qoder-history-id"
        ]
    );
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 1);
    let hooks = value["hooks"].as_object().unwrap();
    assert!(!hooks.contains_key("PostToolBatch"));
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PermissionRequest",
        "Notification",
        "Stop",
        "StopFailure",
        "SessionEnd",
    ] {
        let command = &hooks[event][0]["hooks"][0];
        assert_eq!(command["type"], "command");
        assert_eq!(command["command"], helper.to_string_lossy().as_ref());
        assert_eq!(command["args"], serde_json::json!(["--hook"]));
        assert_eq!(command["timeout"], 2);
        assert!(
            command.get("async").is_none(),
            "preserve event order without model wakeups"
        );
    }
    assert!(!root.path().join(".qoder").exists());
    assert!(!root.path().join(".qoder-cn").exists());
    assert_eq!(Agent::QoderCn.label(), "Qoder CN");
    assert!(!Agent::QoderCn.is_shell());
}

#[test]
fn qoder_cn_workspace_metadata_roundtrips_without_inventing_agent_state() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let workspace = Workspace {
        sessions: vec![SessionSpec {
            id: "one".into(),
            name: "Qoder CN 👋".into(),
            directory: Path::new("C:/Projects/demo").into(),
            agent: Agent::QoderCn,
            agent_session_id: Some("history".into()),
            color: None,
        }],
        ..Default::default()
    };
    ai_terminal::model::save(&path, &workspace).unwrap();
    let restored = ai_terminal::model::load(&path).unwrap();
    assert_eq!(restored.sessions[0].agent, Agent::QoderCn);
    assert_eq!(
        restored.sessions[0].agent_session_id.as_deref(),
        Some("history")
    );
}

#[test]
fn qoder_cn_hook_shapes_drive_states_without_forwarding_conversation_content() {
    use ai_terminal::{
        bridge::Bridge,
        status::{Activity, Status},
    };
    use std::{
        io::Write,
        process::{Command, Stdio},
        sync::Arc,
        time::Duration,
    };
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let mut status = Status::default();
    for (event_name, tool, notification, expected) in [
        ("SessionStart", None, None, Activity::Idle),
        ("UserPromptSubmit", None, None, Activity::Working),
        ("PermissionRequest", Some("Bash"), None, Activity::Working),
        (
            "Notification",
            None,
            Some("permission_prompt"),
            Activity::Waiting,
        ),
        ("PostToolUse", Some("Bash"), None, Activity::Working),
        ("Stop", None, None, Activity::Completed),
        ("UserPromptSubmit", None, None, Activity::Working),
        (
            "PreToolUse",
            Some("AskUserQuestion"),
            None,
            Activity::Waiting,
        ),
        (
            "PostToolUse",
            Some("AskUserQuestion"),
            None,
            Activity::Working,
        ),
        ("StopFailure", None, None, Activity::Failed),
    ] {
        let payload = serde_json::json!({"hook_event_name": event_name, "session_id": "qoder-native-shape",
            "tool_name": tool, "notification_type": notification, "source": "startup",
            "prompt": "PRIVATE_PROMPT", "tool_input": {"command": "PRIVATE_ARGUMENT"},
            "last_assistant_message": "PRIVATE_RESULT", "transcript_path": "PRIVATE_TRANSCRIPT"});
        let mut child = Command::new(env!("CARGO_BIN_EXE_ai-terminal-hook"))
            .env("AI_TERMINAL_ENDPOINT", &bridge.endpoint)
            .env("AI_TERMINAL_TOKEN", &bridge.token)
            .env("AI_TERMINAL_SESSION", "local")
            .env("AI_TERMINAL_GENERATION", "qoder-generation")
            .env("AI_TERMINAL_INVOCATION", "qoder-invocation")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "{}");
        let event = bridge
            .receiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        assert_eq!(event.invocation.as_deref(), Some("qoder-invocation"));
        assert!(
            event.turn_id.is_none(),
            "Qoder payloads need not supply a turn ID"
        );
        assert!(!serde_json::to_string(&event).unwrap().contains("PRIVATE_"));
        assert!(status.apply(&event, false));
        assert_eq!(status.activity, expected, "{event_name}");
        if matches!(expected, Activity::Completed | Activity::Failed) {
            assert!(status.unread);
            status.acknowledge();
            assert!(!status.unread);
            assert_eq!(status.activity, expected);
        }
    }
}
