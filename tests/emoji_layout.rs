#![cfg(windows)]
#[path = "support/text_console.rs"]
mod text_console;
use ai_terminal::appearance;
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
#[test]
fn real_conpty_emoji_modifiers_and_zwj_render_as_color_clusters() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let (tx, _) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        810,
        ctx.clone(),
        tx,
        BackendSettings {
            shell: text_console::executable().to_string_lossy().into_owned(),
            args: vec!["emoji".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(12);
    while !terminal.plain_text().contains("DONE") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 600.0),
        )),
        ..Default::default()
    };
    let output = ctx.run_ui(input, |ui| {
        let view = TerminalView::new(ui, &mut terminal).set_font(appearance::terminal_font(18.0));
        ui.add(view);
    });
    let quads: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Mesh(mesh) = &s.shape {
                Some(mesh.calc_bounds())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        quads.len(),
        9,
        "skin-tone and ZWJ pieces must not become separate pictures"
    );
    assert!(
        (quads[0].width() - quads[2].width()).abs() < 0.001,
        "VS16 heart was clipped to half an emoji cell"
    );
    let color_pixels = output
        .textures_delta
        .set
        .iter()
        .map(|(_, delta)| match &delta.image {
            egui::ImageData::Color(image) => image
                .pixels
                .iter()
                .filter(|p| p.r() > 180 && p.g() > 130 && p.b() < 90)
                .count(),
        })
        .sum::<usize>();
    assert!(
        color_pixels > 100,
        "actual terminal glyph atlas contains no color emoji"
    );
}

#[test]
fn native_caret_stays_above_glyphs_and_block_cursor_uses_its_real_background() {
    for shape in ["6", "2"] {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let _renderer = egui_term::TextRendererSession::new(&ctx);
        let (tx, _) = mpsc::channel();
        let mut terminal = TerminalBackend::new(
            815,
            ctx.clone(),
            tx,
            BackendSettings {
                shell: text_console::executable().to_string_lossy().into_owned(),
                args: vec![format!("caret-{shape}")],
                ..Default::default()
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        while !terminal.plain_text().contains('A') {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 600.0),
            )),
            focused: true,
            ..Default::default()
        };
        let output = ctx.run_ui(raw, |ui| {
            let view = TerminalView::new(ui, &mut terminal)
                .set_font(appearance::terminal_font(14.0))
                .set_focus(true);
            ui.add(view);
        });
        if shape == "6" {
            let glyph = output
                .shapes
                .iter()
                .rposition(|s| matches!(s.shape, egui::Shape::Mesh(_)))
                .unwrap();
            let caret=output.shapes.iter().rposition(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.rect.width()<=2.0 && r.rect.height()>10.0)).unwrap();
            assert!(
                caret > glyph,
                "native glyph tile erased the insertion caret"
            );
        } else {
            let cursor = egui::Color32::from_rgb(215, 215, 215);
            let painted = output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == cursor && rect.rect.width() > 5.0)
            });
            assert!(painted, "opaque glyph background erased the block cursor");
        }
    }
}
