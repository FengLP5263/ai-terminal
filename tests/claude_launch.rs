#![cfg(windows)]
use ai_terminal::{
    launch,
    model::{Agent, SessionSpec},
};

#[test]
fn managed_claude_settings_include_recovery_events_and_cli_accepts_arguments() {
    let root = tempfile::tempdir().unwrap();
    let spec = SessionSpec {
        id: "claude-test".into(),
        name: "test".into(),
        directory: root.path().into(),
        agent: Agent::Claude,
        agent_session_id: None,
        color: None,
    };
    let args = launch::agent_arguments(
        &spec,
        root.path(),
        std::path::Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook")),
    )
    .unwrap();
    let settings = args
        .windows(2)
        .find(|pair| pair[0] == "--settings")
        .unwrap()[1]
        .clone();
    let data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
    for event in ["Notification", "PostToolBatch", "PermissionDenied"] {
        assert!(data["hooks"][event].is_array(), "missing {event} hook");
    }
    let Some(binary) = launch::executable(Agent::Claude) else {
        return;
    };
    let config = root.path().join("claude-home");
    std::fs::create_dir(&config).unwrap();
    let output = std::process::Command::new(binary)
        .args(["--settings", &settings, "--help"])
        .env("CLAUDE_CONFIG_DIR", config)
        .env_remove("CLAUDECODE")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Claude rejected settings: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
