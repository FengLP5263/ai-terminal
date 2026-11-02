#![cfg(windows)]
use ai_terminal::{launch, model::Agent};
use std::process::Command;

#[test]
fn installed_codex_accepts_scoped_hook_toml_without_auth_or_trust_changes() {
    let Some(binary) = launch::executable(Agent::Codex) else {
        return;
    };
    let isolated_home = tempfile::tempdir().unwrap();
    let helper = std::path::Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    let mut command = Command::new(binary);
    command
        .env("CODEX_HOME", isolated_home.path())
        .env("AI_TERMINAL_HELPER", helper)
        .current_dir(isolated_home.path());
    for setting in launch::codex_hook_overrides()
        .into_iter()
        .chain(launch::codex_notification_overrides())
    {
        command.args(["-c", &setting]);
    }
    let output = command
        .args(["--no-daemon", "features", "list"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Codex rejected generated hook config: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stdout.is_empty(),
        "installed CLI must return its feature configuration"
    );
    assert!(!isolated_home.path().join("auth.json").exists());
}

#[test]
fn codex_keeps_native_tui_and_uses_an_owned_engine_for_scoped_state() {
    let root = tempfile::tempdir().unwrap();
    let spec = ai_terminal::model::SessionSpec {
        id: "codex".into(),
        name: "test".into(),
        directory: root.path().into(),
        agent: Agent::Codex,
        agent_session_id: None,
        color: None,
    };
    let args = launch::agent_arguments(
        &spec,
        root.path(),
        std::path::Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook")),
    )
    .unwrap();
    assert!(args.iter().any(|arg| arg == "--no-daemon"));
    assert!(args.iter().any(|arg| arg.contains("tui.notifications")));
    assert!(
        args.iter()
            .any(|arg| arg.contains("tui.notification_method='osc9'"))
    );
    assert!(!args.iter().any(|arg| arg == "--no-alt-screen"));
}
