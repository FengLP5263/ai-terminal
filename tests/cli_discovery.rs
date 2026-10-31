#![cfg(windows)]
use ai_terminal::{discovery::CliSearch, launch, model::Agent};
use egui_term::{BackendCommand, BackendSettings, TerminalBackend};
use std::os::windows::process::CommandExt;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{OnceLock, mpsc},
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

fn fixture() -> &'static Path {
    static BUILT: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &BUILT
        .get_or_init(|| {
            let root = tempfile::tempdir().unwrap();
            let output = root.path().join("cli-fixture.exe");
            let result = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "cli_args_fixture"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_args.rs"))
                .arg("-o")
                .arg(&output)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "fixture build: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            (root, output)
        })
        .1
}

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
    let mut args =
        launch::codex_hook_overrides(&script.with_file_name("it's %CLI_TEST_VAR% & hook.exe"));
    args.extend([
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
