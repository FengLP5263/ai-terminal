use ai_terminal::{
    discovery::CliSearch,
    launch,
    model::{Agent, SessionSpec},
};

#[test]
fn international_cli_is_distinct_from_cn_and_the_ide() {
    let root = tempfile::tempdir().unwrap();
    let search = CliSearch::from_directories([root.path().into()]);
    for file in ["qoder.exe", "qoder.cmd", "qodercn.exe"] {
        std::fs::write(root.path().join(file), "IDE or CN fixture").unwrap();
    }
    assert!(search.find(Agent::Qoder).is_none());
    std::fs::write(root.path().join("qodercli.cmd"), "@echo off").unwrap();
    assert_eq!(
        search.find(Agent::Qoder).unwrap(),
        root.path().join("qodercli.cmd")
    );
    std::fs::write(root.path().join("qodercli.exe"), "CLI fixture").unwrap();
    assert_eq!(
        search.find(Agent::Qoder).unwrap(),
        root.path().join("qodercli.exe")
    );
    assert_eq!(
        search.find(Agent::QoderCn).unwrap(),
        root.path().join("qodercn.exe")
    );
}

#[test]
fn international_hooks_resume_and_saved_identity_stay_independent_of_cn() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = SessionSpec {
        id: "same-id".into(),
        name: "Qoder 🌎".into(),
        directory: root.path().into(),
        agent: Agent::Qoder,
        agent_session_id: Some("history".into()),
        color: None,
    };
    let args = launch::agent_arguments(
        &spec,
        root.path(),
        std::path::Path::new("C:/helper with space/hook.exe"),
    )
    .unwrap();
    assert_eq!(&args[2..], ["--resume", "history"]);
    let hooks: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert!(hooks["hooks"].get("PostToolBatch").is_none());
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "PermissionRequest",
        "Notification",
        "Stop",
        "StopFailure",
        "SessionEnd",
    ] {
        assert_eq!(
            hooks["hooks"][event][0]["hooks"][0]["args"],
            serde_json::json!(["--hook"])
        );
    }
    let restored: SessionSpec =
        serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
    assert_eq!(restored.agent, Agent::Qoder);
    assert_eq!(restored.agent_session_id.as_deref(), Some("history"));
    spec.agent = Agent::QoderCn;
    let cn_args =
        launch::agent_arguments(&spec, root.path(), std::path::Path::new("hook.exe")).unwrap();
    assert_ne!(args[1], cn_args[1]);
    assert_eq!(Agent::Qoder.label(), "Qoder");
    assert!(!Agent::Qoder.is_shell());
    assert!(!root.path().join(".qoder").exists());
}
