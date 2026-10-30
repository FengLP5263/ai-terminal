use ai_terminal::{bridge::Bridge, status::Activity};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

#[test]
fn native_hook_executable_reports_metadata_without_gui_or_prompt_content() {
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ai-terminal-hook"))
        .env("AI_TERMINAL_ENDPOINT", &bridge.endpoint)
        .env("AI_TERMINAL_TOKEN", &bridge.token)
        .env("AI_TERMINAL_SESSION", "local")
        .env("AI_TERMINAL_GENERATION", "launch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(br#"{"hook_event_name":"Stop","session_id":"real-shape-id","turn_id":"one","prompt":"PRIVATE_FIXTURE_PROMPT","last_assistant_message":"PRIVATE_FIXTURE_RESULT"}"#).unwrap();
    let event = bridge
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("real hook executable must deliver a lifecycle event");
    assert_eq!(event.name, "Stop");
    assert!(
        !serde_json::to_string(&event)
            .unwrap()
            .contains("PRIVATE_FIXTURE")
    );
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "{}");
    let mut status = ai_terminal::status::Status::default();
    status.apply(&event, false);
    assert_eq!(status.activity, Activity::Idle);
    assert!(status.unread);
}
