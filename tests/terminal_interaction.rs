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
    let input = egui::RawInput {
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
            .set_size(egui::vec2(780.0, 460.0));
        ui.add(view);
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
