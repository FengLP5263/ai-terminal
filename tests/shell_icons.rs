use ai_terminal::{appearance, model::Agent, sidebar::Icons};
use egui::{FontFamily, Pos2, Rect};

#[test]
fn shell_avatars_use_the_pinned_codicons_glyphs_at_each_scale() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let icons = Icons::new(&ctx);
    for scale in [1.0, 1.5, 2.0] {
        ctx.set_zoom_factor(scale);
        for (agent, glyph) in [
            (Agent::Shell, "\u{ebc7}"),
            (Agent::Bash, "\u{ebca}"),
            (Agent::Cmd, "\u{ebc4}"),
        ] {
            let output = ctx.run_ui(Default::default(), |ui| {
                icons.paint(
                    ui.painter(),
                    Rect::from_min_size(Pos2::new(20.0, 20.0), egui::vec2(36.0, 36.0)),
                    agent,
                );
            });
            let galley = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(text) if text.galley.job.text == glyph => Some(&text.galley),
                    _ => None,
                })
                .expect("Official terminal glyph must be painted");
            assert!(
                galley
                    .job
                    .sections
                    .iter()
                    .all(|s| s.format.font_id.family == FontFamily::Name("codicons".into()))
            );
            assert!(galley.size().x > 0.0 && galley.size().y > 0.0);
        }
    }
}
