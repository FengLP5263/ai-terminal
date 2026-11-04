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
        eprintln!(
            "{} is not installed; native directory check not run",
            kind.label()
        );
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
    let mut plan = launch::build(&spec, &bridge, &data, helper).unwrap();
    isolate_bash_home(kind, root.path(), &mut plan);
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
        eprintln!(
            "{} is not installed; native Agent entry check not run",
            kind.label()
        );
        return;
    }
    let root = tempfile::tempdir().unwrap();
    std::fs::copy(support::fixture(), root.path().join("cli-fixture.exe")).unwrap();
    let fake = root.path().join("fake-agent.cmd");
    std::fs::write(
        &fake,
        b"@echo off\r\n\"%~dp0cli-fixture.exe\" \"%~dp0args.txt\" %*\r\nexit /b 23\r\n",
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
    isolate_bash_home(kind, root.path(), &mut plan);
    plan.environment.insert(
        "AI_TERMINAL_REAL_CODEX".into(),
        fake.to_string_lossy().into_owned(),
    );
    plan.environment.insert(
        "AI_TERMINAL_REAL_CLAUDE".into(),
        fake.to_string_lossy().into_owned(),
    );
    plan.environment.insert(
        "AI_TERMINAL_REAL_QODER_CN".into(),
        fake.to_string_lossy().into_owned(),
    );
    for key in [
        "AI_TERMINAL_REAL_KIMI",
        "AI_TERMINAL_REAL_ZCODE",
        "AI_TERMINAL_REAL_TRAE",
    ] {
        plan.environment
            .insert(key.into(), fake.to_string_lossy().into_owned());
    }
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
    for (agent, source) in [
        ("codex", "codex"),
        ("claude", "claude"),
        ("qodercn", "qodercn"),
        ("qoderclicn", "qodercn"),
        ("qodercn --settings=custom-settings.json", "qodercn"),
        ("kimi", "kimi"),
        ("zcode", "zcode"),
        ("traecli", "traecli"),
    ] {
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
        assert_eq!(began.source.as_deref(), Some(source));
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
        assert_eq!(ended.exit_code, Some(23));
        assert_eq!(ended.invocation, began.invocation);
        if agent.contains("--settings=") {
            assert_eq!(
                std::fs::read_to_string(root.path().join("args.txt")).unwrap(),
                "[\"--settings=custom-settings.json\"]",
                "an explicit user configuration must not be replaced or duplicated",
            );
            assert!(!root.path().join("custom-settings.json").exists());
        }
        wait_directory(&rx, &generation, root.path(), &mut terminal);
    }
    terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
}
#[test]
fn powershell_recognizes_supported_agents_and_returns_to_shell() {
    agent_roundtrip(Agent::Shell);
}
#[test]
fn cmd_recognizes_supported_agents_and_returns_to_shell() {
    agent_roundtrip(Agent::Cmd);
}
#[test]
fn bash_recognizes_supported_agents_and_returns_to_shell() {
    agent_roundtrip(Agent::Bash);
}

fn isolate_bash_home(kind: Agent, home: &Path, plan: &mut launch::Launch) {
    if kind == Agent::Bash {
        plan.environment
            .insert("HOME".into(), home.to_string_lossy().replace('\\', "/"));
        std::fs::write(home.join(".bash_profile"), ". \"$HOME/.bashrc\"\n").unwrap();
        std::fs::write(home.join(".bashrc"), "PS1='TEST_PROMPT> '\n").unwrap();
    }
}

#[test]
fn git_bash_preserves_profiles_aliases_completion_and_both_prompt_command_forms() {
    if launch::executable(Agent::Bash).is_none() {
        eprintln!("Git Bash is not installed; native profile check not run");
        return;
    }
    for (profile, prompt) in [
        (".bash_profile", "PROMPT_COMMAND='__user_prompt'"),
        (".bash_profile", "PROMPT_COMMAND=('__user_prompt' 'true')"),
        (".bash_login", "PROMPT_COMMAND='__user_prompt'"),
        (".profile", "PROMPT_COMMAND=('__user_prompt' 'true')"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("用户配置 with space & '分号;");
        std::fs::create_dir(&home).unwrap();
        let bridge = Bridge::start(Arc::new(|| {})).unwrap();
        let spec = SessionSpec {
            id: "profile-test".into(),
            name: "Git Bash".into(),
            directory: home.clone(),
            agent: Agent::Bash,
            agent_session_id: None,
            color: None,
        };
        let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
        let mut plan = launch::build(&spec, &bridge, &home.join("runtime"), helper).unwrap();
        isolate_bash_home(Agent::Bash, &home, &mut plan);
        std::fs::remove_file(home.join(".bash_profile")).unwrap();
        std::fs::write(
            home.join(profile),
            "export PROFILE_FIRST=yes\n. \"$HOME/.bashrc\"\n",
        )
        .unwrap();
        for file in [".bash_profile", ".bash_login", ".profile"]
            .into_iter()
            .skip_while(|file| *file != profile)
            .skip(1)
        {
            std::fs::write(home.join(file), "export PROFILE_FIRST=wrong\n").unwrap();
        }
        std::fs::write(
            home.join(".bashrc"),
            format!(
                r#"export PATH=/usr/bin:/bin
PS1='PROFILE_PROMPT> '
alias startup_alias='printf ALIAS_OK'
complete -W 'alpha beta' startup_command
__user_prompt() {{ USER_EXIT=$?; export USER_EXIT; }}
{prompt}
"#
            ),
        )
        .unwrap();
        let generation = plan.generation.clone();
        let (tx, rx) = mpsc::channel();
        let mut terminal = TerminalBackend::new(
            932,
            egui::Context::default(),
            tx,
            BackendSettings {
                shell: plan.executable,
                args: plan.arguments,
                env: plan.environment,
                working_directory: Some(home.clone()),
            },
        )
        .unwrap();
        let _job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
        wait_directory(&rx, &generation, &home, &mut terminal);
        terminal.process_command(BackendCommand::Write(b"false\r".to_vec()));
        wait_directory(&rx, &generation, &home, &mut terminal);
        terminal.process_command(BackendCommand::Write(r#"[[ "$PROFILE_FIRST" == yes && "$USER_EXIT" == 1 && "${PROMPT_COMMAND[0]}" == __user_prompt && "$(command -v codex)" == */bin/codex* ]] && alias startup_alias >/dev/null && complete -p startup_command >/dev/null && printf '\nPROFILE_CONFIG_OK\n'
"#.replace('\n', "\r").into_bytes()));
        wait_directory(&rx, &generation, &home, &mut terminal);
        let text = terminal.plain_text();
        assert!(
            text.lines().any(|line| line.trim() == "PROFILE_CONFIG_OK"),
            "{prompt}: {text}"
        );
        assert!(
            text.contains("PROFILE_PROMPT> "),
            "user PS1 must survive: {text}"
        );
        assert!(
            !text.contains("command not found"),
            "Git system PATH must be available: {text}"
        );
        terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
    }
}

#[test]
fn nested_interactive_shells_emit_correlated_entry_and_exit_without_command_false_positives() {
    let root = tempfile::tempdir().unwrap();
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let spec = SessionSpec {
        id: "nested-shells".into(),
        name: "CMD".into(),
        directory: root.path().into(),
        agent: Agent::Cmd,
        agent_session_id: None,
        color: None,
    };
    let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    let mut plan = launch::build(&spec, &bridge, &root.path().join("runtime"), helper).unwrap();
    isolate_bash_home(Agent::Bash, root.path(), &mut plan);
    std::fs::copy(support::fixture(), root.path().join("cli-fixture.exe")).unwrap();
    let fake = root.path().join("fake-agent.cmd");
    std::fs::write(
        &fake,
        b"@echo off\r\n\"%~dp0cli-fixture.exe\" \"%~dp0args.txt\" %*\r\nexit /b 23\r\n",
    )
    .unwrap();
    plan.environment.insert(
        "AI_TERMINAL_REAL_CODEX".into(),
        fake.to_string_lossy().into_owned(),
    );
    plan.environment
        .insert("AI_TERMINAL_FIXTURE_INTERACTIVE".into(), "1".into());
    let generation = plan.generation.clone();
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        933,
        egui::Context::default(),
        tx,
        BackendSettings {
            shell: plan.executable,
            args: plan.arguments,
            env: plan.environment,
            working_directory: Some(root.path().into()),
        },
    )
    .unwrap();
    let _job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
    wait_directory(&rx, &generation, root.path(), &mut terminal);
    terminal.process_command(BackendCommand::Write(
        b"powershell -NoProfile -Command \"Write-Output ONE_OFF_DONE\"\r".to_vec(),
    ));
    wait_directory(&rx, &generation, root.path(), &mut terminal);
    assert!(
        bridge.receiver.try_recv().is_err(),
        "one-off commands are not a shell switch"
    );
    terminal.process_command(BackendCommand::Write(
        b"powershell -NoProfile -NoLogo\r".to_vec(),
    ));
    let powershell = bridge
        .receiver
        .recv_timeout(Duration::from_secs(8))
        .expect("PowerShell entry");
    assert_eq!(powershell.name, "ShellStarted");
    assert_eq!(powershell.source.as_deref(), Some("powershell"));
    assert!(powershell.parent_invocation.is_none());
    assert!(
        powershell
            .console_processes
            .as_ref()
            .unwrap()
            .contains(&terminal.pty_id())
    );
    terminal.process_command(BackendCommand::Write(b"cmd /d\r".to_vec()));
    let cmd = bridge
        .receiver
        .recv_timeout(Duration::from_secs(8))
        .expect("nested CMD entry");
    assert_eq!(cmd.name, "ShellStarted");
    assert_eq!(cmd.source.as_deref(), Some("cmd"));
    assert_eq!(cmd.parent_invocation, powershell.invocation);
    terminal.process_command(BackendCommand::Write(b"codex\r".to_vec()));
    let agent = bridge
        .receiver
        .recv_timeout(Duration::from_secs(8))
        .expect("nested Agent entry");
    assert_eq!(agent.name, "ShellAgentStarted");
    assert_eq!(agent.parent_invocation, cmd.invocation);
    assert!(
        agent
            .console_processes
            .as_ref()
            .unwrap()
            .contains(&terminal.pty_id())
    );
    let until = Instant::now() + Duration::from_secs(8);
    while !terminal.plain_text().contains("CLI_ENTRY_READY") && Instant::now() < until {
        let _ = rx.recv_timeout(Duration::from_millis(30));
    }
    assert!(terminal.plain_text().contains("CLI_ENTRY_READY"));
    terminal.process_command(BackendCommand::Write(b"fixture reply\r".to_vec()));
    let agent_exit = bridge
        .receiver
        .recv_timeout(Duration::from_secs(8))
        .expect("nested Agent exit");
    assert_eq!(agent_exit.name, "ShellAgentEnded");
    assert_eq!(agent_exit.invocation, agent.invocation);
    terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
    let cmd_exit = bridge
        .receiver
        .recv_timeout(Duration::from_secs(8))
        .expect("nested CMD exit");
    assert_eq!(cmd_exit.name, "ShellEnded");
    assert_eq!(cmd_exit.invocation, cmd.invocation);
    terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
    let powershell_exit = bridge
        .receiver
        .recv_timeout(Duration::from_secs(8))
        .expect("PowerShell exit");
    assert_eq!(powershell_exit.name, "ShellEnded");
    assert_eq!(powershell_exit.invocation, powershell.invocation);
    wait_directory(&rx, &generation, root.path(), &mut terminal);
    terminal.process_command(BackendCommand::Write(b"exit\r".to_vec()));
}
