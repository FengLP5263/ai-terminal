#![cfg(windows)]
use ai_terminal::{
    bridge::Bridge,
    launch,
    model::{self, Agent, SessionSpec},
    shell,
};
use egui_term::{BackendCommand, BackendSettings, PtyEvent, TerminalBackend};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

fn wait_directory(
    rx: &mpsc::Receiver<(u64, PtyEvent)>,
    generation: &str,
    expected: &Path,
    terminal: &mut TerminalBackend,
) -> PathBuf {
    let until = Instant::now() + Duration::from_secs(12);
    while Instant::now() < until {
        if let Ok((_, PtyEvent::Title(title))) = rx.recv_timeout(Duration::from_millis(50))
            && let Some(path) = shell::directory_from_title(&title, generation)
            && path.canonicalize().unwrap() == expected.canonicalize().unwrap()
        {
            return path;
        }
    }
    panic!(
        "shell did not report directory {}: {}",
        expected.display(),
        terminal.plain_text()
    );
}

fn directory_roundtrip(kind: Agent) {
    if launch::executable(kind).is_none() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("工作目录 with space & semicolon;");
    std::fs::create_dir(&project).unwrap();
    let data = root.path().join("runtime");
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let spec = SessionSpec {
        id: "shell".into(),
        name: kind.label().into(),
        directory: root.path().into(),
        agent: kind,
        agent_session_id: None,
        color: None,
    };
    let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    let plan = launch::build(&spec, &bridge, &data, helper).unwrap();
    let generation = plan.generation.clone();
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        930,
        egui::Context::default(),
        tx,
        BackendSettings {
            shell: plan.executable,
            args: plan.arguments,
            env: plan.environment,
            working_directory: Some(spec.directory.clone()),
        },
    )
    .unwrap();
    let _job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
    wait_directory(&rx, &generation, root.path(), &mut terminal);
    let target = project.to_string_lossy();
    let command = match kind {
        Agent::Shell => format!(
            "Set-Location -LiteralPath '{}'\r",
            target.replace('\'', "''")
        ),
        Agent::Cmd => format!("cd /d \"{target}\"\r"),
        Agent::Bash => format!(
            "cd -- '{}'\r",
            target.replace('\\', "/").replace('\'', "'\\''")
        ),
        _ => unreachable!(),
    };
    terminal.process_command(BackendCommand::Write(command.into_bytes()));
    let reported = wait_directory(&rx, &generation, &project, &mut terminal);
    assert_eq!(
        model::default_directory(Some(&reported), Some(root.path())),
        reported
    );
    terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
}
#[test]
fn powershell_reports_changed_working_directory() {
    directory_roundtrip(Agent::Shell);
}
#[test]
fn cmd_reports_changed_working_directory() {
    directory_roundtrip(Agent::Cmd);
}
#[test]
fn git_bash_reports_changed_working_directory() {
    directory_roundtrip(Agent::Bash);
}
#[test]
fn cwd_metadata_rejects_old_generations_and_invalid_paths() {
    let root = tempfile::tempdir().unwrap();
    let title = format!("{}old;{}", shell::CWD_PREFIX, root.path().display());
    assert!(shell::directory_from_title(&title, "new").is_none());
    assert!(shell::directory_from_title("ai-terminal-cwd;new;relative", "new").is_none());
    assert!(
        shell::directory_from_title("ai-terminal-cwd;new;C:/missing-fixture-dir", "new").is_none()
    );
}

mod support;
fn agent_roundtrip(kind: Agent) {
    if launch::executable(kind).is_none() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    std::fs::copy(support::fixture(), root.path().join("cli-fixture.exe")).unwrap();
    let fake = root.path().join("fake-agent.cmd");
    std::fs::write(
        &fake,
        b"@echo off\r\n\"%~dp0cli-fixture.exe\" \"%~dp0args.txt\" %*\r\n",
    )
    .unwrap();
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let spec = SessionSpec {
        id: "agent-shell".into(),
        name: kind.label().into(),
        directory: root.path().into(),
        agent: kind,
        agent_session_id: None,
        color: None,
    };
    let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    let mut plan = launch::build(&spec, &bridge, &root.path().join("runtime"), helper).unwrap();
    plan.environment.insert(
        "AI_TERMINAL_REAL_CODEX".into(),
        fake.to_string_lossy().into_owned(),
    );
    plan.environment.insert(
        "AI_TERMINAL_REAL_CLAUDE".into(),
        fake.to_string_lossy().into_owned(),
    );
    plan.environment
        .insert("AI_TERMINAL_FIXTURE_INTERACTIVE".into(), "1".into());
    let generation = plan.generation.clone();
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        931,
        egui::Context::default(),
        tx,
        BackendSettings {
            shell: plan.executable,
            args: plan.arguments,
            env: plan.environment,
            working_directory: Some(spec.directory.clone()),
        },
    )
    .unwrap();
    let _job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
    wait_directory(&rx, &generation, root.path(), &mut terminal);
    terminal.process_command(BackendCommand::Write(b"echo codex\r".to_vec()));
    wait_directory(&rx, &generation, root.path(), &mut terminal);
    assert!(
        bridge.receiver.try_recv().is_err(),
        "mentioning codex must not be treated as launching it"
    );
    for agent in ["codex", "claude"] {
        terminal.process_command(BackendCommand::Write(format!("{agent}\r").into_bytes()));
        let began = bridge
            .receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|e| {
                panic!(
                    "no {agent} start in {:?}: {e}; {}",
                    kind,
                    terminal.plain_text()
                )
            });
        assert_eq!(began.name, "ShellAgentStarted");
        assert_eq!(began.source.as_deref(), Some(agent));
        assert_eq!(began.generation, generation);
        assert!(began.invocation.is_some());
        assert!(began.process_id.is_some());
        let until = Instant::now() + Duration::from_secs(8);
        while !terminal.plain_text().contains("CLI_ENTRY_READY") && Instant::now() < until {
            let _ = rx.recv_timeout(Duration::from_millis(30));
        }
        assert!(terminal.plain_text().contains("CLI_ENTRY_READY"));
        terminal.process_command(BackendCommand::Write(b"fixture reply\r".to_vec()));
        let ended = bridge
            .receiver
            .recv_timeout(Duration::from_secs(8))
            .unwrap();
        assert_eq!(ended.name, "ShellAgentEnded");
        assert_eq!(ended.invocation, began.invocation);
        wait_directory(&rx, &generation, root.path(), &mut terminal);
    }
    terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
}
#[test]
fn powershell_recognizes_codex_and_claude_and_returns_to_shell() {
    agent_roundtrip(Agent::Shell);
}
#[test]
fn cmd_recognizes_codex_and_claude_and_returns_to_shell() {
    agent_roundtrip(Agent::Cmd);
}
#[test]
fn bash_recognizes_codex_and_claude_and_returns_to_shell() {
    agent_roundtrip(Agent::Bash);
}
