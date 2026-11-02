#![cfg(windows)]
use ai_terminal::{
    appearance, dialogs,
    i18n::{self, Language},
};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};

fn text_rect(output: &egui::FullOutput, text: &str) -> Option<Rect> {
    fn find(shape: &egui::Shape, text: &str) -> Option<Rect> {
        match shape {
            egui::Shape::Text(t) if t.galley.job.text == text => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, text)),
            _ => None,
        }
    }
    output.shapes.iter().find_map(|s| find(&s.shape, text))
}

#[test]
fn rename_requires_a_nonempty_name_and_preserves_unicode_when_saved() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    i18n::apply(&ctx, Language::En);
    let _renderer = egui_term::TextRendererSession::new(&ctx);
    let mut open = true;
    let mut name = String::new();
    let mut tick = 0;
    let mut frame = |events: Vec<Event>, name: &mut String| {
        tick += 1;
        let mut submitted = false;
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 650.0))),
                time: Some(tick as f64 * 0.1),
                events,
                ..Default::default()
            },
            |_| {
                submitted = dialogs::rename(&ctx, &mut open, name);
            },
        );
        (output, submitted)
    };
    frame(vec![], &mut name);
    let (out, _) = frame(vec![], &mut name);
    let point = text_rect(&out, "Save").unwrap().center();
    let pointer = |pressed| {
        vec![
            Event::PointerMoved(point),
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ]
    };
    frame(pointer(true), &mut name);
    assert!(!frame(pointer(false), &mut name).1);
    name = "AI终端 👩🏽‍💻 ❤️".into();
    frame(vec![], &mut name);
    frame(pointer(true), &mut name);
    assert!(frame(pointer(false), &mut name).1);
    assert_eq!(name, "AI终端 👩🏽‍💻 ❤️");
}
