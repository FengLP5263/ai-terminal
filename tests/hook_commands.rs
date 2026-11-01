#![cfg(windows)]
use ai_terminal::{
    bridge::Bridge,
    launch,
    status::{Activity, Status},
};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

#[test]
fn stable_codex_command_reports_turns_from_relocated_unicode_helper_paths() {
    let root = tempfile::tempdir().unwrap();
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let setting = launch::codex_hook_overrides().into_iter().next().unwrap();
    let command = setting.split_once("command_windows=").unwrap().1;
    let command = serde_json::Deserializer::from_str(command)
        .into_iter::<String>()
        .next()
        .unwrap()
        .unwrap();
    let helper = Path::new(env!("CARGO_BIN_EXE_ai-terminal-hook"));
    for dirname in ["first install", "新版 中文 it's & 100%"] {
        let directory = root.path().join(dirname);
        std::fs::create_dir(&directory).unwrap();
        let relocated = directory.join("ai-terminal-hook.exe");
        std::fs::copy(helper, &relocated).unwrap();
        let mut status = Status::default();
        for (event, expected) in [
            ("UserPromptSubmit", Activity::Working),
            ("Stop", Activity::Completed),
        ] {
            let mut child =
                Command::new(launch::executable(ai_terminal::model::Agent::Shell).unwrap())
                    .args(["-NoLogo", "-NoProfile", "-Command", &command])
                    .env("AI_TERMINAL_HELPER", &relocated)
                    .env("AI_TERMINAL_ENDPOINT", &bridge.endpoint)
                    .env("AI_TERMINAL_TOKEN", &bridge.token)
                    .env("AI_TERMINAL_SESSION", "local")
                    .env("AI_TERMINAL_GENERATION", "generation")
                    .env("AI_TERMINAL_INVOCATION", "foreground-invocation")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap();
            write!(child.stdin.take().unwrap(),"{{\"hook_event_name\":\"{event}\",\"session_id\":\"conversation\",\"turn_id\":\"turn\"}}").unwrap();
            let received = bridge
                .receiver
                .recv_timeout(Duration::from_secs(8))
                .expect("actual generated hook command must deliver metadata");
            assert_eq!(
                received.invocation.as_deref(),
                Some("foreground-invocation")
            );
            assert!(status.apply(&received, true));
            assert_eq!(status.activity, expected);
            let result = child.wait_with_output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        assert!(
            !status.unread,
            "a viewed completion remains visible without becoming unread"
        );
    }
}
