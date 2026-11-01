#![cfg(windows)]
use egui_term::{BackendCommand, BackendSettings, TerminalBackend};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn conpty_accepts_input_drains_hidden_output_and_copies_only_viewport() {
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        919,
        egui::Context::default(),
        tx,
        BackendSettings {
            shell: "cmd.exe".into(),
            args: vec!["/Q".into()],
            ..Default::default()
        },
    )
    .expect("create real Windows ConPTY");
    terminal.set_visible(false);
    terminal.process_command(BackendCommand::Write(
        b"for /L %i in (1,1,12000) do @echo output-%i\r\necho NATIVE_PTY_DRAIN_OK\r\n".to_vec(),
    ));
    let began = Instant::now();
    // This verifies draining, not a CPU/console throughput SLA. CMD may take >15s
    // to emit 12,000 individual lines on a loaded Windows desktop.
    let deadline = began + Duration::from_secs(30);
    let mut found = false;
    while Instant::now() < deadline {
        if terminal.plain_text().contains("NATIVE_PTY_DRAIN_OK")
            && terminal.plain_text().contains("output-12000")
        {
            found = true;
            break;
        }
        // Waiting here is test synchronization, not production polling.
        let _ = rx.recv_timeout(Duration::from_millis(40));
    }
    assert!(
        found,
        "hidden session did not finish draining after {:?}; viewport: {}",
        began.elapsed(),
        terminal.plain_text()
    );
    let snapshot = terminal.sync();
    assert!(
        snapshot.cells.len() <= 80 * 50,
        "snapshot grew with scrollback"
    );
    assert!(snapshot.display_offset == 0);
    terminal.process_command(BackendCommand::Write(b"exit\r\n".to_vec()));
}
