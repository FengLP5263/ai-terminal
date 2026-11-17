#![cfg(windows)]
use ai_terminal::{appearance, ui_text};
use egui::{
    Color32, Event, Id,
    text::{CCursor, CCursorRange},
};

#[test]
fn wrapped_multiline_labels_keep_complete_emoji_near_line_boundaries() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let _renderer = egui_term::TextRendererSession::new(&ctx);
    for prefix in 0..24 {
        for emoji in ["❤️", "👩🏽‍💻", "🇨🇳", "1️⃣"] {
            let text = format!(
                "{}{emoji} end\nC:/Projects/很长的目录/保持原文",
                "a".repeat(prefix)
            );
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 500.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.set_max_width(120.0);
                    ui_text::multiline_label(ui, &text);
                },
            );
            let galley = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.job.text.replace('\n', "") == text.replace('\n', "") =>
                    {
                        Some(&t.galley)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(galley.rows.len() > 2);
            assert!(galley.size().x <= 121.0);
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Mesh(m) if m.texture_id!=egui::TextureId::default())),"wrapped emoji lost its native bitmap: {prefix} {emoji}");
        }
    }
}

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

#[test]
fn styled_emoji_keeps_source_syntax_colors_and_complete_graphemes() {
    use egui::{FontFamily, FontId, TextFormat, text::LayoutJob};
    use unicode_segmentation::UnicodeSegmentation;
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let _renderer = egui_term::TextRendererSession::new(&ctx);
    let source = "键 🔥 ⛔ 👩🏽‍💻 🇨🇳 1️⃣ ❤️ end";
    let colors = [Color32::LIGHT_BLUE, Color32::LIGHT_RED];
    let _ = ctx.run_ui(Default::default(), |ui| {
        let mut job = LayoutJob::default();
        job.wrap.max_width = 700.0;
        // Deliberately split modifiers, ZWJ and variation selectors across syntax spans.
        for (index, ch) in source.chars().enumerate() {
            job.append(
                &ch.to_string(),
                0.0,
                TextFormat {
                    font_id: FontId::monospace(16.0),
                    color: colors[index % 2],
                    ..Default::default()
                },
            );
        }
        let original = job.clone();
        let galley = ui_text::layout_job(ui.ctx(), job);
        assert_eq!(galley.job.text, source, "Source bytes must not change");
        assert_eq!(galley.job.wrap, original.wrap);
        for (start, cluster) in source.grapheme_indices(true) {
            let end = start + cluster.len();
            let span = galley
                .job
                .sections
                .iter()
                .find(|s| s.byte_range.contains(&egui::text::ByteIndex(start)))
                .unwrap();
            if cluster.chars().any(|c| c as u32 >= 0x1f000) || ["⛔", "1️⃣", "❤️"].contains(&cluster)
            {
                assert!(span.byte_range.end.0 >= end, "Split emoji {cluster}");
                assert_eq!(span.format.color, Color32::TRANSPARENT);
                assert_eq!(
                    span.format.font_id.family,
                    FontFamily::Name("ui-emoji".into())
                );
                let from = galley.pos_from_cursor(CCursor::new(source[..start].chars().count()));
                let to = galley.pos_from_cursor(CCursor::new(source[..end].chars().count()));
                assert!(to.left() > from.left());
                assert!(
                    to.left() - from.left() < 36.0,
                    "Multiple slots for {cluster}"
                );
            } else {
                let before = original
                    .sections
                    .iter()
                    .find(|s| s.byte_range.contains(&egui::text::ByteIndex(start)))
                    .unwrap();
                assert_eq!(span.format, before.format);
            }
        }
        for row in &galley.rows {
            assert!(
                row.visuals
                    .mesh
                    .vertices
                    .iter()
                    .filter(|v| v.color == Color32::TRANSPARENT)
                    .all(|v| v.pos == egui::Pos2::ZERO)
            );
        }
        ui.painter()
            .galley(egui::pos2(8.0, 8.0), galley.clone(), Color32::WHITE);
        ui_text::paint_emoji(
            ui.painter(),
            egui::pos2(8.0, 8.0),
            &galley,
            16.0,
            Color32::WHITE,
        );
    });
}

#[test]
fn styled_plain_text_and_explicit_text_symbols_keep_original_layout() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let _ = ctx.run_ui(Default::default(), |ui| {
        for source in ["let value = 42; 中文", "✖\u{fe0e} ⛔\u{fe0e}"] {
            let job = egui::text::LayoutJob::simple_singleline(
                source.into(),
                egui::FontId::monospace(15.0),
                Color32::LIGHT_BLUE,
            );
            let normal = ui.fonts_mut(|fonts| fonts.layout_job(job.clone()));
            let styled = ui_text::layout_job(ui.ctx(), job.clone());
            assert_eq!(*styled.job, job);
            assert_eq!(styled.size(), normal.size());
        }
    });
}
