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
        .current_dir(isolated_home.path());
    for setting in launch::codex_hook_overrides(helper) {
        command.args(["-c", &setting]);
    }
    let output = command.args(["features", "list"]).output().unwrap();
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
