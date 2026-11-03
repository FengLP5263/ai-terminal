#![cfg(windows)]
use ai_terminal::{discovery::CliSearch, launch, model::Agent};
use egui_term::{BackendCommand, BackendSettings, TerminalBackend};
use std::os::windows::process::CommandExt;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

mod support;
use support::fixture;

fn shim(root: &Path) -> PathBuf {
    let prefix = root.join("工具 it's & 100% ! custom prefix");
    std::fs::create_dir_all(&prefix).unwrap();
    std::fs::copy(fixture(), prefix.join("cli-fixture.exe")).unwrap();
    let script = prefix.join("codex.cmd");
    std::fs::write(
        &script,
        b"@echo off\r\n\"%~dp0cli-fixture.exe\" \"%~dp0args.txt\" %*\r\n",
    )
    .unwrap();
    script
}

#[test]
fn custom_cmd_entry_preserves_complex_hook_args_and_the_child_exit_code() {
    let root = tempfile::tempdir().unwrap();
    let script = shim(root.path());
    let search = CliSearch::from_directories([script.parent().unwrap().to_owned()]);
    let binary = search
        .find(Agent::Codex)
        .expect("public cmd entry without any npm private layout");
    let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    let mut args = launch::codex_hook_overrides();
    args.extend([
        script
            .with_file_name("it's %CLI_TEST_VAR% & hook.exe")
            .to_string_lossy()
            .into_owned(),
        "空格 argument".into(),
        "literal %CLI_TEST_VAR% !CLI_TEST_VAR! & ^ < > |".into(),
        "a\"quoted\"value".into(),
    ]);
    let (program, planned) = launch::pty_command(&binary, args.clone(), helper).unwrap();
    assert_eq!(program.trim_matches('"'), helper.to_str().unwrap());
    let output = Command::new(helper)
        .args(&planned)
        .current_dir(root.path())
        .env("CLI_TEST_VAR", "MUST_NOT_EXPAND")
        .stdin(Stdio::null())
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(37),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(script.with_file_name("args.txt")).unwrap(),
        format!("{args:?}")
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("CLI_ENTRY_READY"));
}

#[test]
fn a_standard_stale_native_npm_entry_is_skipped_before_the_real_launch() {
    let root = tempfile::tempdir().unwrap();
    let old = root.path().join("stale prefix");
    let healthy = root.path().join("healthy prefix");
    std::fs::create_dir(&old).unwrap();
    std::fs::create_dir(&healthy).unwrap();
    let header = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n";
    std::fs::write(
        old.join("claude.cmd"),
        format!("{header}\"%dp0%\\removed.exe\"  %*\r\n"),
    )
    .unwrap();
    std::fs::copy(fixture(), healthy.join("installed.exe")).unwrap();
    let script = healthy.join("claude.cmd");
    std::fs::write(&script, format!("{header}\"%dp0%\\installed.exe\"  %*\r\n")).unwrap();
    let selected = CliSearch::from_directories([old, healthy])
        .resolve(Agent::Claude)
        .unwrap();
    assert_eq!(selected, script);
    let output_file = root.path().join("arguments.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_ai-terminal-hook"))
        .arg("--launch-cli")
        .arg(selected)
        .arg(&output_file)
        .arg("literal 中文")
        .stdin(Stdio::null())
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(37),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("CLI_ENTRY_READY"));
    assert!(
        std::fs::read_to_string(output_file)
            .unwrap()
            .contains("literal 中文")
    );
}

#[test]
fn cmd_entry_has_interactive_conpty_handles_and_stays_in_the_owned_process_tree() {
    let root = tempfile::tempdir().unwrap();
    let script = shim(root.path());
    let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    let (program, args) =
        launch::pty_command(&script, vec!["fixture-session".into()], helper).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        922,
        egui::Context::default(),
        tx,
        BackendSettings {
            shell: program,
            args,
            working_directory: Some(root.path().into()),
            env: std::collections::HashMap::from([(
                "AI_TERMINAL_FIXTURE_INTERACTIVE".into(),
                "1".into(),
            )]),
        },
    )
    .unwrap();
    let pid = terminal.pty_id();
    let job = platform::ProcessJob::attach(pid).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !terminal.plain_text().contains("CLI_ENTRY_READY") && Instant::now() < deadline {
        let _ = rx.recv_timeout(Duration::from_millis(30));
    }
    assert!(
        terminal.plain_text().contains("CLI_ENTRY_READY"),
        "runner must inherit the actual ConPTY output handle"
    );
    terminal.process_command(BackendCommand::Write(
        "hello 中文输入\r\n".as_bytes().to_vec(),
    ));
    let input = script.with_file_name("args.txt.input");
    while !input.exists() && Instant::now() < deadline {
        let _ = rx.recv_timeout(Duration::from_millis(30));
    }
    assert!(
        std::fs::read_to_string(input)
            .unwrap()
            .contains("hello 中文输入")
    );
    assert_eq!(
        std::fs::read_to_string(script.with_file_name("args.txt")).unwrap(),
        "[\"fixture-session\"]"
    );
    drop(terminal);
    drop(job);
    // Open only our own test root process, never enumerate/terminate user CLIs.
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::*,
        };
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !process.is_null() {
            assert_eq!(WaitForSingleObject(process, 5000), WAIT_OBJECT_0);
            CloseHandle(process);
        }
    }
}

#[test]
fn launching_from_another_managed_shell_resolves_the_real_cli_not_its_old_shim() {
    const CHILD: &str = "AI_TERMINAL_DISCOVERY_CHILD";
    if let Some(expected) = std::env::var_os(CHILD) {
        let search = CliSearch::current();
        assert_eq!(search.find(Agent::Codex), Some(PathBuf::from(expected)));
        let old = PathBuf::from(std::env::var_os("AI_TERMINAL_SHIM_DIR").unwrap());
        assert!(!std::env::split_paths(&search.child_path().unwrap()).any(|p| p == old));
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old shim");
    let real = dir.path().join("real cli");
    for p in [&old, &real] {
        std::fs::create_dir(p).unwrap();
        std::fs::write(p.join("codex.exe"), b"fixture").unwrap();
    }
    let mut paths = vec![old.clone(), real.clone()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "launching_from_another_managed_shell_resolves_the_real_cli_not_its_old_shim",
        ])
        .env(CHILD, real.join("codex.exe"))
        .env("AI_TERMINAL_SHIM_DIR", old)
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
