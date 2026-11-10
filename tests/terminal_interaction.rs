#![cfg(windows)]

use egui::{Event, ImeEvent, Key, Modifiers};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

fn terminal(ctx: &egui::Context, script: &str) -> TerminalBackend {
    let (tx, _) = mpsc::channel();
    TerminalBackend::new(
        733,
        ctx.clone(),
        tx,
        BackendSettings {
            shell: "powershell.exe".into(),
            args: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-Command".into(),
                script.into(),
            ],
            ..Default::default()
        },
    )
    .unwrap()
}

fn frame(
    ctx: &egui::Context,
    backend: &mut TerminalBackend,
    events: Vec<Event>,
    discard: bool,
) -> egui::FullOutput {
    frame_with_window_focus(ctx, backend, events, discard, true)
}

fn frame_with_window_focus(
    ctx: &egui::Context,
    backend: &mut TerminalBackend,
    events: Vec<Event>,
    discard: bool,
    focused: bool,
) -> egui::FullOutput {
    frame_with_selection(ctx, backend, events, discard, focused, false)
}

fn frame_with_selection(
    ctx: &egui::Context,
    backend: &mut TerminalBackend,
    events: Vec<Event>,
    discard: bool,
    focused: bool,
    drag_selection: bool,
) -> egui::FullOutput {
    let input = egui::RawInput {
        focused,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 500.0),
        )),
        events,
        ..Default::default()
    };
    ctx.run_ui(input, |ui| {
        let view = TerminalView::new(ui, backend)
            .set_focus(true)
            .set_drag_selection(drag_selection)
            .set_size(egui::vec2(780.0, 460.0));
        let rect = ui.add(view).rect;
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new("terminal-test-rect"), rect));
        if discard && ui.ctx().current_pass_index() == 0 {
            ui.ctx().request_discard("exercise repeat layout pass");
        }
    })
}

fn wait_text(backend: &mut TerminalBackend, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut text = String::new();
    while Instant::now() < deadline {
        text = backend.plain_text();
        if text.contains(needle) {
            return text;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("terminal did not emit {needle:?}: {text}");
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn unknown_viewport_geometry_does_not_resize_conpty_to_the_default_canvas() {
    let ctx = egui::Context::default();
    ai_terminal::appearance::setup(&ctx);
    let mut backend = terminal(&ctx, "Start-Sleep -Seconds 20");
    let before: egui_term::WindowSize = backend.sync().terminal_size.into();
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        let view = TerminalView::new(ui, &mut backend)
            .set_font(ai_terminal::appearance::terminal_font(14.0));
        ui.add(view);
    });
    let after: egui_term::WindowSize = backend.sync().terminal_size.into();
    assert_eq!(
        (after.num_cols, after.num_lines),
        (before.num_cols, before.num_lines)
    );
    frame(&ctx, &mut backend, vec![], false);
    let real: egui_term::WindowSize = backend.sync().terminal_size.into();
    assert_ne!(
        (real.num_cols, real.num_lines),
        (before.num_cols, before.num_lines),
        "real viewport resize remains enabled"
    );
}

#[test]
fn sizing_and_invisible_layouts_do_not_resize_the_live_terminal() {
    let ctx = egui::Context::default();
    ai_terminal::appearance::setup(&ctx);
    let mut backend = terminal(&ctx, "Start-Sleep -Seconds 20");
    frame(&ctx, &mut backend, vec![], false);
    let before: egui_term::WindowSize = backend.sync().terminal_size.into();
    for sizing in [true, false] {
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 500.0),
                )),
                ..Default::default()
            },
            |ui| {
                let builder = egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(9000.0, 9000.0),
                ));
                let builder = if sizing {
                    builder.sizing_pass()
                } else {
                    builder.invisible()
                };
                ui.scope_builder(builder, |ui| {
                    let view = TerminalView::new(ui, &mut backend)
                        .set_font(ai_terminal::appearance::terminal_font(14.0))
                        .set_size(egui::vec2(9000.0, 9000.0));
                    ui.add(view);
                });
            },
        );
        let after: egui_term::WindowSize = backend.sync().terminal_size.into();
        assert_eq!(
            (after.num_cols, after.num_lines),
            (before.num_cols, before.num_lines),
            "a measurement-only pass resized the live ConPTY"
        );
    }
}

#[test]
fn repaint_preserves_chinese_composition() {
    let ctx = egui::Context::default();
    let mut backend = terminal(&ctx, "Start-Sleep -Seconds 20");
    frame(&ctx, &mut backend, vec![], false);
    let output = frame(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "nihao".into(),
            active_range_chars: None,
        })],
        false,
    );
    assert!(
        !output
            .platform_output
            .ime
            .unwrap()
            .should_interrupt_composition,
        "a normal repaint must not cancel the native Chinese candidate window"
    );
    let repainted = frame(&ctx, &mut backend, vec![], false);
    assert!(
        repainted.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "nihao"
        )),
        "a normal repaint must retain the pending composition preview"
    );
}

#[test]
fn chinese_commit_and_enter_reach_real_conpty_once() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::InputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write('PROBE_READY'); $line=[Console]::ReadLine(); [Console]::Write('INPUT_HEX:'+ [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($line))); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "PROBE_READY");
    frame(&ctx, &mut backend, vec![Event::Text("abc".into())], true);
    frame(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "ni".into(),
            active_range_chars: None,
        })],
        false,
    );
    frame(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Commit("你好".into()))],
        false,
    );
    frame(&ctx, &mut backend, vec![key(Key::Enter)], false);
    let text = wait_text(&mut backend, "INPUT_HEX:");
    assert!(
        text.contains("INPUT_HEX:61-62-63-E4-BD-A0-E5-A5-BD"),
        "input was dropped or duplicated: {text}"
    );
}

#[test]
fn queued_text_and_editing_keys_keep_order_in_a_real_conpty() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::InputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write('BATCH_READY'); $line=[Console]::ReadLine(); [Console]::Write('BATCH_HEX:'+ [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($line))); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "BATCH_READY");
    frame(
        &ctx,
        &mut backend,
        vec![
            Event::Text("a".into()),
            Event::Text("b".into()),
            key(Key::Backspace),
            Event::Text("你好".into()),
            key(Key::Enter),
        ],
        false,
    );
    let text = wait_text(&mut backend, "BATCH_HEX:");
    assert!(
        text.contains("BATCH_HEX:61-E4-BD-A0-E5-A5-BD"),
        "queued input changed order or was lost: {text}"
    );
}

#[test]
fn renderer_preserves_combining_marks_and_hides_concealed_text() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write(([char]27)+'[2J'+([char]27)+'[H'+'e'+([char]0x301)+' '+([char]27)+'[8m'+'HIDDEN'+([char]27)+'[0m'+' DISPLAY_READY'); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "DISPLAY_READY");
    let output = frame(&ctx, &mut backend, vec![], false);
    let text: String = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        text.contains("e\u{301}"),
        "combining character was lost: {text:?}"
    );
    assert!(
        !text.contains("HIDDEN"),
        "concealed terminal cells were painted: {text:?}"
    );
}

#[test]
fn terminal_owns_tab_arrows_and_escape() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::Write('KEY_PROBE_READY'); for($i=0;$i -lt 4;$i++){ $k=[Console]::ReadKey($true); [Console]::Write('KEY:'+$k.Key.ToString()+';') }; Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "KEY_PROBE_READY");
    for value in [Key::Tab, Key::ArrowLeft, Key::ArrowUp, Key::Escape] {
        frame(&ctx, &mut backend, vec![key(value)], false);
        std::thread::sleep(Duration::from_millis(30));
    }
    let text = wait_text(&mut backend, "KEY:Escape;");
    for value in ["KEY:Tab;", "KEY:LeftArrow;", "KEY:UpArrow;", "KEY:Escape;"] {
        assert!(text.contains(value), "lost special key {value}: {text}");
    }
}

#[test]
fn canceled_composition_does_not_block_editing_or_enter() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::InputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write('EDIT_PROBE_READY'); $line=[Console]::ReadLine(); [Console]::Write('EDIT_HEX:'+ [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($line))); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "EDIT_PROBE_READY");
    frame(&ctx, &mut backend, vec![Event::Text("ac".into())], false);
    frame(&ctx, &mut backend, vec![key(Key::ArrowLeft)], false);
    frame(&ctx, &mut backend, vec![Event::Text("b".into())], false);
    frame(&ctx, &mut backend, vec![key(Key::End)], false);
    frame(&ctx, &mut backend, vec![Event::Text("Z".into())], false);
    frame(&ctx, &mut backend, vec![key(Key::Backspace)], false);
    frame(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "pin".into(),
            active_range_chars: None,
        })],
        false,
    );
    frame(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Preedit {
            text: String::new(),
            active_range_chars: None,
        })],
        false,
    );
    frame(&ctx, &mut backend, vec![Event::Text("你好".into())], false);
    frame(&ctx, &mut backend, vec![key(Key::Enter)], false);
    let text = wait_text(&mut backend, "EDIT_HEX:");
    assert!(
        text.contains("EDIT_HEX:61-62-63-E4-BD-A0-E5-A5-BD"),
        "line editing corrupted: {text}"
    );
}

#[test]
fn renderer_honors_requested_beam_cursor() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::Write(([char]27)+'[2J'+([char]27)+'[H'+'BEAM_READY'+([char]27)+'[6 q'); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "BEAM_READY");
    let output = frame(&ctx, &mut backend, vec![], false);
    assert!(
        output.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Rect(rect) =>
                rect.rect.width() <= 2.0
                    && rect.rect.height() >= 10.0
                    && rect.fill != egui::Color32::TRANSPARENT,
            _ => false,
        }),
        "the requested caret was painted as a solid character block"
    );
}

#[test]
fn wide_character_cursor_inverts_the_cell_color_above_both_backgrounds() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $e=[char]27; [Console]::Write($e+'[2J'+$e+'[H'+$e+']12;#000000'+[char]7+$e+'[48;2;48;48;48m然后还有这个问题'+$e+'[0m'+$e+'[3;1HCURSOR_READY'+$e+'[1;15H'+$e+'[2 q'); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "CURSOR_READY");
    for scale in [1.0, 1.03, 1.1, 1.5] {
        ctx.set_zoom_factor(scale);
        frame(&ctx, &mut backend, vec![], false);
        let output = frame(&ctx, &mut backend, vec![], false);
        let size = backend.sync().terminal_size;
        let cursor = output
            .shapes
            .iter()
            .position(|s| {
                matches!(&s.shape,
                    egui::Shape::Rect(r) if r.fill == egui::Color32::BLACK
                        && (r.rect.width()-size.cell_width*2.0).abs()<0.1
                        && (r.rect.height()-size.cell_height).abs()<0.1
                )
            })
            .expect("full two-cell black cursor");
        assert!(
            output
                .shapes
                .iter()
                .enumerate()
                .all(|(i, s)| !matches!(&s.shape,
                    egui::Shape::Rect(r) if r.fill == egui::Color32::from_gray(48)
                ) || i < cursor),
            "wide spacer background covered cursor at {scale}"
        );
        assert!(
            output.shapes[cursor + 1..]
                .iter()
                .any(|s| matches!(&s.shape,
                    egui::Shape::Text(t) if t.galley.job.text == "题"
                        && t.override_text_color == Some(egui::Color32::from_gray(48))
                )),
            "inverted glyph must use its gray cell background at {scale}"
        );
    }
}

#[test]
fn wheel_scrolls_terminal_output_up_and_down_without_typing_history_keys() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "1..160 | ForEach-Object { [Console]::WriteLine(('SCROLL-LINE-'+$_)) }; [Console]::WriteLine('SCROLL_READY'); Start-Sleep -Seconds 30",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "SCROLL_READY");
    frame(&ctx, &mut backend, vec![], false);
    let bottom = backend.plain_text();
    assert_eq!(backend.sync().display_offset, 0);
    frame(
        &ctx,
        &mut backend,
        vec![
            Event::PointerMoved(egui::pos2(200.0, 200.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 3.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ],
        false,
    );
    assert!(backend.sync().display_offset > 0);
    assert_ne!(backend.plain_text(), bottom);
    frame(
        &ctx,
        &mut backend,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -42.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        }],
        false,
    );
    assert_eq!(backend.sync().display_offset, 0);
}

#[test]
fn alternate_screen_wheel_never_injects_up_or_down_arrow_sequences() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "$e=[char]27; [Console]::Write($e+'[?1049h'+$e+'[?1007h'+'ALT_WHEEL_READY'); $c=[Console]::ReadKey($true); [Console]::WriteLine(('FIRST_INPUT_'+[int]$c.KeyChar)); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "ALT_WHEEL_READY");
    assert!(
        backend.sync().terminal_mode.contains(
            egui_term::TerminalMode::ALT_SCREEN | egui_term::TerminalMode::ALTERNATE_SCROLL
        )
    );
    frame(
        &ctx,
        &mut backend,
        vec![
            Event::PointerMoved(egui::pos2(200.0, 200.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 3.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ],
        false,
    );
    backend.process_command(egui_term::BackendCommand::Write(b"z".to_vec()));
    wait_text(&mut backend, "FIRST_INPUT_122");
}

fn mouse_terminal(ctx: &egui::Context, mode: &str) -> TerminalBackend {
    static FIXTURE: std::sync::OnceLock<(tempfile::TempDir, std::path::PathBuf)> =
        std::sync::OnceLock::new();
    let binary = &FIXTURE
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let exe = dir.path().join("mouse-console.exe");
            let result = std::process::Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "mouse_console"])
                .arg(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/mouse_console.rs"),
                )
                .arg("-o")
                .arg(&exe)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            (dir, exe)
        })
        .1;
    let (tx, _) = mpsc::channel();
    TerminalBackend::new(
        735,
        ctx.clone(),
        tx,
        BackendSettings {
            shell: binary.to_string_lossy().into_owned(),
            args: vec![mode.into()],
            ..Default::default()
        },
    )
    .unwrap()
}

fn pointer_button(pos: egui::Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}

#[test]
fn scrollbar_drags_real_history_without_pty_input_or_geometry_changes() {
    let ctx = egui::Context::default();
    let mut backend = mouse_terminal(&ctx, "scrollbar");
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "APP_WHEEL_READY");
    frame(&ctx, &mut backend, vec![], false);
    let pid = backend.pty_id();
    let initial: egui_term::WindowSize = backend.sync().terminal_size.into();
    let history = backend.last_content().history_size;
    assert!(
        history > 300,
        "fixture must create real ConPTY history: {history}"
    );
    let rect: egui::Rect = ctx.data(|d| d.get_temp(egui::Id::new("terminal-test-rect")).unwrap());
    let top = egui::pos2(rect.right() - 6.0, rect.top() + 2.0);
    for events in [
        vec![
            Event::PointerMoved(top),
            pointer_button(top, true, Modifiers::NONE),
        ],
        vec![Event::PointerMoved(top - egui::vec2(0.0, 30.0))],
    ] {
        frame(&ctx, &mut backend, events, false);
    }
    assert_eq!(
        backend.sync().display_offset,
        history,
        "drag reaches oldest retained line"
    );
    let visible: String = backend
        .last_content()
        .cells
        .iter()
        .map(|c| c.cell.c)
        .collect();
    assert!(
        visible.contains("HISTORY_LINE_0000"),
        "oldest content must be visible: {visible}"
    );
    let middle = rect.center_top() + egui::vec2(rect.width() * 0.5 - 6.0, rect.height() * 0.5);
    frame(&ctx, &mut backend, vec![Event::PointerMoved(middle)], false);
    let offset = backend.sync().display_offset;
    assert!(
        offset > history / 4 && offset < history * 3 / 4,
        "mid-track offset {offset}/{history}"
    );
    let bottom = egui::pos2(rect.right() - 6.0, rect.bottom() + 25.0);
    frame(&ctx, &mut backend, vec![Event::PointerMoved(bottom)], false);
    frame(
        &ctx,
        &mut backend,
        vec![pointer_button(bottom, false, Modifiers::NONE)],
        false,
    );
    assert_eq!(backend.sync().display_offset, 0);
    assert!(backend.selectable_content().is_empty());
    let after: egui_term::WindowSize = backend.last_content().terminal_size.into();
    assert_eq!(
        (initial.num_cols, initial.num_lines),
        (after.num_cols, after.num_lines)
    );
    assert_eq!(pid, backend.pty_id());
    // In SGR all-motion mode, the first received byte must still be our sentinel.
    backend.process_command(egui_term::BackendCommand::Write(b"z".to_vec()));
    wait_text(&mut backend, "REPORT_0_7A");
}

#[test]
fn scrollbar_keeps_columns_when_history_appears_and_stays_bounded() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "Start-Sleep -Milliseconds 700; 1..5400 | ForEach-Object {Write-Output ('BOUNDED_LINE_'+$_)}; Write-Output 'HISTORY_READY'; Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    let initial: egui_term::WindowSize = backend.sync().terminal_size.into();
    assert_eq!(backend.last_content().history_size, 0);
    wait_text(&mut backend, "HISTORY_READY");
    frame(&ctx, &mut backend, vec![], false);
    let after: egui_term::WindowSize = backend.sync().terminal_size.into();
    assert_eq!(
        (initial.num_cols, initial.num_lines),
        (after.num_cols, after.num_lines)
    );
    assert_eq!(backend.last_content().history_size, 5000);
    assert!(
        backend.last_content().cells.len() <= after.num_cols as usize * after.num_lines as usize
    );
    let rect: egui::Rect = ctx.data(|d| d.get_temp(egui::Id::new("terminal-test-rect")).unwrap());
    let pos = egui::pos2(rect.right() - 6.0, rect.top() + 3.0);
    frame(
        &ctx,
        &mut backend,
        vec![pointer_button(pos, true, Modifiers::NONE)],
        false,
    );
    frame(
        &ctx,
        &mut backend,
        vec![pointer_button(pos, false, Modifiers::NONE)],
        false,
    );
    assert!(
        backend.sync().display_offset > 4500,
        "track click jumps without scrolling every row"
    );
}

fn selection_points(ctx: &egui::Context, backend: &TerminalBackend) -> [egui::Pos2; 2] {
    let rect: egui::Rect =
        ctx.data(|data| data.get_temp(egui::Id::new("terminal-test-rect")).unwrap());
    let size = backend.last_content().terminal_size;
    [0.1, 8.9].map(|column| rect.min + egui::vec2(column * size.cell_width, 1.5 * size.cell_height))
}

fn select_fixture(
    ctx: &egui::Context,
    backend: &mut TerminalBackend,
    prefer_local: bool,
    modifiers: Modifiers,
) {
    let [start, end] = selection_points(ctx, backend);
    for events in [
        vec![
            Event::PointerMoved(start),
            pointer_button(start, true, modifiers),
        ],
        vec![Event::PointerMoved(end)],
        vec![pointer_button(end, false, modifiers)],
    ] {
        frame_with_selection(ctx, backend, events, false, true, prefer_local);
    }
}

#[test]
fn coding_cli_left_drag_selects_and_copies_without_sending_mouse_or_interrupt() {
    for mode in ["sgr", "legacy", "select-motion", "select-all", "local"] {
        let ctx = egui::Context::default();
        let mut backend = mouse_terminal(&ctx, mode);
        frame_with_selection(&ctx, &mut backend, vec![], false, true, true);
        wait_text(&mut backend, "SELECT_ME");
        frame_with_selection(&ctx, &mut backend, vec![], false, true, true);
        select_fixture(&ctx, &mut backend, true, Modifiers::NONE);
        assert_eq!(backend.selectable_content(), "SELECT_ME", "{mode}");
        let output = frame_with_selection(&ctx, &mut backend, vec![Event::Copy], false, true, true);
        assert!(
            output
                .platform_output
                .commands
                .iter()
                .any(|command| matches!(
                    command, egui::OutputCommand::CopyText(text) if text == "SELECT_ME"
                )),
            "{mode}: selected text was not copied"
        );
        // A sentinel reaches the raw console first only if dragging/copying did
        // not leak a mouse report or Ctrl+C into the running application.
        backend.process_command(egui_term::BackendCommand::Write(b"z".to_vec()));
        wait_text(&mut backend, "REPORT_0_7A");
    }
}

#[test]
fn coding_cli_click_without_drag_still_reaches_the_application() {
    let ctx = egui::Context::default();
    let mut backend = mouse_terminal(&ctx, "select-all");
    frame_with_selection(&ctx, &mut backend, vec![], false, true, true);
    wait_text(&mut backend, "SELECT_ME");
    let [pos, _] = selection_points(&ctx, &backend);
    frame_with_selection(
        &ctx,
        &mut backend,
        vec![pointer_button(pos, true, Modifiers::NONE)],
        false,
        true,
        true,
    );
    frame_with_selection(
        &ctx,
        &mut backend,
        vec![pointer_button(pos, false, Modifiers::NONE)],
        false,
        true,
        true,
    );
    wait_text(&mut backend, "REPORT_0_1B-5B-3C-30-3B-31-3B-32-4D");
    wait_text(&mut backend, "REPORT_1_1B-5B-3C-30-3B-31-3B-32-6D");
    assert!(backend.selectable_content().is_empty());
}

#[test]
fn copy_without_selection_keeps_ctrl_c_interrupt() {
    let ctx = egui::Context::default();
    let mut backend = mouse_terminal(&ctx, "select-all");
    frame_with_selection(&ctx, &mut backend, vec![], false, true, true);
    wait_text(&mut backend, "SELECT_ME");
    let output = frame_with_selection(&ctx, &mut backend, vec![Event::Copy], false, true, true);
    assert!(
        !output
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::CopyText(_)))
    );
    wait_text(&mut backend, "REPORT_0_03");
}

#[test]
fn generic_mouse_application_keeps_drag_reports_and_shift_selection() {
    for mode in ["select-motion", "select-all"] {
        let ctx = egui::Context::default();
        let mut backend = mouse_terminal(&ctx, mode);
        frame(&ctx, &mut backend, vec![], false);
        wait_text(&mut backend, "SELECT_ME");
        frame(&ctx, &mut backend, vec![], false);
        select_fixture(&ctx, &mut backend, false, Modifiers::NONE);
        wait_text(&mut backend, "REPORT_0_1B-5B-3C-30-");
        wait_text(&mut backend, "REPORT_1_1B-5B-3C-33-32-");
        wait_text(&mut backend, "REPORT_2_1B-5B-3C-30-");
        assert!(backend.selectable_content().is_empty());
        select_fixture(&ctx, &mut backend, false, Modifiers::SHIFT);
        assert_eq!(backend.selectable_content(), "SELECT_ME");
    }
}
#[test]
fn application_mouse_tracking_receives_real_wheel_reports_in_both_directions() {
    for mode in ["sgr", "legacy"] {
        let ctx = egui::Context::default();
        let mut backend = mouse_terminal(&ctx, mode);
        frame(&ctx, &mut backend, vec![], false);
        wait_text(&mut backend, "APP_WHEEL_READY");
        for (i, delta) in [1.0, -1.0].into_iter().enumerate() {
            frame(
                &ctx,
                &mut backend,
                vec![
                    Event::PointerMoved(egui::pos2(80.0, 140.0)),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, delta),
                        modifiers: Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ],
                false,
            );
            let button = if i == 0 { "34" } else { "35" };
            let prefix = if mode == "sgr" {
                format!("REPORT_{i}_1B-5B-3C-36-{button}-3B")
            } else {
                format!("REPORT_{i}_1B-5B-4D-{}", if i == 0 { "60" } else { "61" })
            };
            wait_text(&mut backend, &prefix);
        }
    }
}
#[test]
fn fullscreen_conversation_changes_page_on_point_wheel_without_local_history() {
    let ctx = egui::Context::default();
    let mut backend = mouse_terminal(&ctx, "pager");
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "history offset 100");
    frame(
        &ctx,
        &mut backend,
        vec![
            Event::PointerMoved(egui::pos2(80.0, 140.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 22.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ],
        false,
    );
    wait_text(&mut backend, "history offset 95");
    assert_eq!(backend.sync().display_offset, 0);
    frame(
        &ctx,
        &mut backend,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -22.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        }],
        false,
    );
    wait_text(&mut backend, "history offset 100");
}
#[test]
fn shift_wheel_uses_local_history_and_ctrl_wheel_sends_no_application_input() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "1..180 | ForEach-Object { [Console]::WriteLine(('LOCAL-'+$_)) }; $e=[char]27; [Console]::Write($e+'[?1000h'+$e+'[?1006h'+'OVERRIDE_READY'); $c=[Console]::ReadKey($true); [Console]::WriteLine(('FIRST_INPUT_'+[int]$c.KeyChar)); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "OVERRIDE_READY");
    frame(
        &ctx,
        &mut backend,
        vec![
            Event::PointerMoved(egui::pos2(80.0, 140.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 8.0),
                modifiers: Modifiers::SHIFT,
                phase: egui::TouchPhase::Move,
            },
        ],
        false,
    );
    assert!(backend.sync().display_offset > 0);
    frame(
        &ctx,
        &mut backend,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: Modifiers::CTRL,
            phase: egui::TouchPhase::Move,
        }],
        false,
    );
    backend.process_command(egui_term::BackendCommand::Write(b"z".to_vec()));
    wait_text(&mut backend, "FIRST_INPUT_122");
}

#[test]
fn focus_loss_during_composition_does_not_block_input() {
    let ctx = egui::Context::default();
    let mut backend = terminal(
        &ctx,
        "[Console]::InputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write('FOCUS_RECOVERY_READY'); $line=[Console]::ReadLine(); [Console]::Write('FOCUS_RECOVERY_HEX:'+ [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($line))); Start-Sleep -Seconds 20",
    );
    frame(&ctx, &mut backend, vec![], false);
    wait_text(&mut backend, "FOCUS_RECOVERY_READY");
    frame(&ctx, &mut backend, vec![Event::Text("kept-".into())], false);
    frame(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "pin".into(),
            active_range_chars: None,
        })],
        false,
    );
    // Native IME cancellation can arrive in the unfocused frame. It must not
    // leave the terminal suppressing all subsequent keyboard input.
    frame_with_window_focus(
        &ctx,
        &mut backend,
        vec![Event::Ime(ImeEvent::Preedit {
            text: String::new(),
            active_range_chars: None,
        })],
        false,
        false,
    );
    frame(&ctx, &mut backend, vec![], false);
    frame(&ctx, &mut backend, vec![Event::Text("ok".into())], false);
    frame(&ctx, &mut backend, vec![key(Key::Enter)], false);
    let text = wait_text(&mut backend, "FOCUS_RECOVERY_HEX:");
    assert!(
        text.contains("FOCUS_RECOVERY_HEX:6B-65-70-74-2D-6F-6B"),
        "focus recovery dropped input or submitted unfinished composition: {text}"
    );
}

fn cursor_position_reports(private: bool) {
    // Interactive CLI agents query the cursor with `ESC[6n` while their stdin is
    // in raw VT mode. The ConPTY passthrough must forward each query to the
    // embedded terminal, which must answer exactly once per query so agents
    // that consume one report per query never see leftovers on their input line.
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/vt_cursor_probe.py"
    );
    let (tx, _) = mpsc::channel();
    let ctx = egui::Context::default();
    let mut backend = TerminalBackend::new(
        741,
        ctx.clone(),
        tx,
        BackendSettings {
            shell: "python.exe".into(),
            args: vec![
                "-X".into(),
                "utf8".into(),
                fixture.into(),
                if private { "private" } else { "standard" }.into(),
            ],
            ..Default::default()
        },
    )
    .unwrap();
    frame(&ctx, &mut backend, vec![], false);
    let text = wait_text(&mut backend, "GOT:");
    let hex_line = text
        .lines()
        .find(|line| line.contains("GOT:"))
        .expect("probe did not report captured stdin");
    let bytes: Vec<u8> = hex_line
        .trim_start_matches(|c: char| !c.is_ascii_hexdigit())
        .split(' ')
        .filter_map(|token| u8::from_str_radix(token, 16).ok())
        .collect();
    assert!(!bytes.is_empty(), "no stdin bytes captured: {text}");
    let reports: Vec<&[u8]> = bytes
        .split(|byte| *byte == 0x1b)
        .filter(|chunk| !chunk.is_empty())
        .collect();
    assert_eq!(
        reports.len(),
        2,
        "expected one cursor report per query, captured: {bytes:?}"
    );
    for report in reports {
        let report_text = String::from_utf8_lossy(report);
        let params: Vec<&str> = report_text
            .strip_prefix(if private { "[?" } else { "[" })
            .and_then(|rest| rest.strip_suffix('R'))
            .map(|body| body.split(';').collect())
            .unwrap_or_default();
        assert_eq!(
            params.len(),
            2,
            "cursor report is not `ESC[{{row}};{{col}}R`: {bytes:?}"
        );
        assert!(
            params
                .iter()
                .all(|param| !param.is_empty() && param.bytes().all(|b| b.is_ascii_digit())),
            "cursor report has non-numeric coordinates: {bytes:?}"
        );
    }
}

#[test]
fn cursor_position_queries_receive_exactly_one_report_per_query() {
    cursor_position_reports(false);
}
#[test]
fn private_cursor_queries_preserve_the_private_response_prefix() {
    cursor_position_reports(true);
}
