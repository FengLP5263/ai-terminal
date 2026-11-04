#![cfg(windows)]
use egui_term::{BackendSettings, CODEX_ATTENTION_EVENT, PtyEvent, TerminalBackend};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn native_codex_lifecycle_survives_a_hidden_windows_conpty() {
    use egui_term::{CODEX_PROGRAM_EVENT, CodexProgramStatus as Native};
    let (tx, rx) = mpsc::channel();
    let terminal = TerminalBackend::new(735, egui::Context::default(), tx, BackendSettings {
        shell: "powershell.exe".into(),
        args: vec!["-NoLogo".into(), "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(),
            "Start-Sleep -Milliseconds 750; $e=[char]27; [Console]::Write($e.ToString()+']7501;state=idle:app=codex'+$e+'\\'+$e+']7501;state=working:app=codex'+$e+'\\'+$e+']7501;state=blocked:app=codex:kind=permission'+[char]7+$e+']7501;state=working:app=codex'+$e+'\\'+$e+']7501;state=done:app=codex'+$e+'\\'); Start-Sleep -Seconds 2".into()],
        ..Default::default()
    }).unwrap();
    terminal.set_visible(false);
    terminal.set_codex_notifications(true);
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut states = Vec::new();
    while Instant::now() < deadline && states.last() != Some(&Native::Done) {
        if let Ok((735, PtyEvent::Title(value))) = rx.recv_timeout(Duration::from_millis(100))
            && value.starts_with(CODEX_PROGRAM_EVENT)
        {
            states.extend(terminal.take_codex_program_statuses(&value));
            assert!(terminal.take_codex_program_statuses(&value).is_empty());
        }
    }
    assert_eq!(
        states,
        [
            Native::Idle,
            Native::Working,
            Native::Permission,
            Native::Working,
            Native::Done
        ]
    );
    terminal.set_codex_notifications(false);
    assert!(
        terminal
            .take_codex_program_statuses(&format!("{CODEX_PROGRAM_EVENT};1"))
            .is_empty()
    );
}

#[test]
fn codex_attention_notification_survives_a_real_hidden_conpty() {
    let (tx, rx) = mpsc::channel();
    let terminal = TerminalBackend::new(733, egui::Context::default(), tx, BackendSettings {
        shell: "powershell.exe".into(),
        args: vec!["-NoLogo".into(), "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(),
            "Start-Sleep -Milliseconds 750; [Console]::Write(([char]27).ToString()+']9;Approval requested: synthetic'+[char]7); Start-Sleep -Seconds 1".into()],
        ..Default::default()
    }).unwrap();
    terminal.set_visible(false);
    terminal.set_codex_notifications(true);
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut found = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok((733, PtyEvent::Title(value)))
                if value.starts_with(CODEX_ATTENTION_EVENT)
                    && terminal.take_codex_attention(&value) == Some(false) =>
            {
                assert!(
                    terminal.take_codex_attention(&value).is_none(),
                    "a notice must be accepted once"
                );
                terminal.set_codex_notifications(false);
                terminal.set_codex_notifications(true);
                assert!(
                    terminal.take_codex_attention(&value).is_none(),
                    "a previous invocation must be ignored"
                );
                found = true;
                break;
            }
            _ => {}
        }
    }
    assert!(
        found,
        "Codex attention OSC was not delivered through Windows ConPTY"
    );
}

#[test]
fn observed_conpty_preserves_regular_output_while_notifications_are_enabled() {
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(734, egui::Context::default(), tx, BackendSettings {
        shell: "powershell.exe".into(),
        args: vec!["-NoLogo".into(), "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(),
            "Start-Sleep -Milliseconds 500; 1..1500 | ForEach-Object { [Console]::WriteLine('sample-output-'+$_) }; [Console]::WriteLine('OBSERVED_OUTPUT_COMPLETE'); Start-Sleep -Seconds 1".into()],
        ..Default::default()
    }).unwrap();
    terminal.set_visible(false);
    terminal.set_codex_notifications(true);
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut found = false;
    while Instant::now() < deadline {
        if terminal.plain_text().contains("OBSERVED_OUTPUT_COMPLETE") {
            found = true;
            break;
        }
        let _ = rx.recv_timeout(Duration::from_millis(50));
    }
    assert!(
        found,
        "enabling notification observation must not interrupt PTY output"
    );
}
