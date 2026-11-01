#![cfg(windows)]
use ai_terminal::{appearance, ui_text};
use egui::{
    Color32, Event, Id,
    text::{CCursor, CCursorRange},
};

#[test]
fn emoji_clusters_have_single_slots_and_editing_copies_original_unicode() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let _renderer = egui_term::TextRendererSession::new(&ctx);
    let id = Id::new("rename-test");
    let mut text = "项目 👋🏽 👩‍💻 ❤️".to_owned();
    let original = text.clone();
    let _ = ctx.run_ui(Default::default(), |ui| {
        let base = ui_text::layout(ui, "👋", 14.0, Color32::WHITE, 200.0);
        let modified = ui_text::layout(ui, "👋🏽", 14.0, Color32::WHITE, 200.0);
        let joined = ui_text::layout(ui, "👩‍💻", 14.0, Color32::WHITE, 200.0);
        assert!(base.size().x > 0.0);
        assert!(
            modified.size().x <= base.size().x * 1.15,
            "skin modifier split into another glyph"
        );
        assert!(
            joined.size().x <= base.size().x * 1.5,
            "ZWJ split into separate emoji"
        );
        ui_text::edit(ui, &mut text, id);
    });
    ctx.memory_mut(|m| m.request_focus(id));
    let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(0),
        CCursor::new(text.chars().count()),
    )));
    state.store(&ctx, id);
    let output = ctx.run_ui(
        egui::RawInput {
            events: vec![Event::Copy],
            ..Default::default()
        },
        |ui| {
            ui_text::edit(ui, &mut text, id);
        },
    );
    assert!(
        output
            .platform_output
            .commands
            .iter()
            .any(|c| matches!(c, egui::OutputCommand::CopyText(s) if s == &original))
    );
    let replacement = "AI 🚀 👩🏽‍💻";
    let _ = ctx.run_ui(
        egui::RawInput {
            events: vec![Event::Paste(replacement.into())],
            ..Default::default()
        },
        |ui| {
            ui_text::edit(ui, &mut text, id);
        },
    );
    assert_eq!(text, replacement);
}

#[test]
fn selecting_emoji_does_not_restore_monochrome_outline_geometry() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let _renderer = egui_term::TextRendererSession::new(&ctx);
    let id = Id::new("selected-emoji");
    let mut text = "👋🏽 👩‍💻 ❤️".to_owned();
    let _ = ctx.run_ui(Default::default(), |ui| {
        ui_text::edit(ui, &mut text, id);
    });
    ctx.memory_mut(|m| m.request_focus(id));
    let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(0),
        CCursor::new(text.chars().count()),
    )));
    state.store(&ctx, id);
    let _ = ctx.run_ui(Default::default(), |ui| {
        let output = ui_text::edit(ui, &mut text, id);
        assert!(!output.cursor_range.unwrap().is_empty());
        for row in &output.galley.rows {
            let visuals = &row.visuals;
            assert!(
                visuals.mesh.vertices[visuals.glyph_vertex_range.clone()]
                    .iter()
                    .all(|v| v.pos == egui::Pos2::ZERO)
            );
        }
    });
}
