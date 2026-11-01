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
    assert_eq!(status.activity, Activity::Completed);
    assert!(status.unread);
}

#[test]
fn child_metadata_is_ignored_but_a_named_root_agent_still_reports() {
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    for child_agent in [true, false] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ai-terminal-hook"))
            .env("AI_TERMINAL_ENDPOINT", &bridge.endpoint)
            .env("AI_TERMINAL_TOKEN", &bridge.token)
            .env("AI_TERMINAL_SESSION", "local")
            .env("AI_TERMINAL_GENERATION", "launch")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let payload = serde_json::json!({"hook_event_name":"PreToolUse","session_id":"main","tool_name":"AskUserQuestion","agent_type":"reviewer","agent_id":if child_agent {Some("child-id")}else{None}});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        if child_agent {
            assert!(bridge.receiver.try_recv().is_err());
        } else {
            assert_eq!(
                bridge
                    .receiver
                    .recv_timeout(Duration::from_secs(3))
                    .unwrap()
                    .name,
                "PreToolUse"
            );
        }
    }
}
