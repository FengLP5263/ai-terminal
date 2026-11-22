//! Apply native color glyphs to labels emitted by the Markdown renderer.
use egui::{Color32, Pos2, Rect, Shape, Ui, epaint::TextShape, layers::ShapeIdx};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn paint_labels(ui: &mut Ui, has_emoji: bool, show: impl FnOnce(&mut Ui)) {
    if !has_emoji || !cfg!(windows) {
        show(ui);
        return;
    }
    let layer = ui.layer_id();
    let start = ui.graphics_mut(|layers| layers.entry(layer).next_idx().0);
    show(ui);
    let labels = ui.graphics_mut(|layers| {
        layers
            .entry(layer)
            .all_entries()
            .enumerate()
            .skip(start)
            .filter_map(|(index, entry)| {
                let Shape::Text(text) = &entry.shape else {
                    return None;
                };
                (text.angle == 0.0
                    && entry.clip_rect.intersects(text.visual_bounding_rect())
                    && text
                        .galley
                        .text()
                        .graphemes(true)
                        .any(crate::ui_text::is_emoji))
                .then(|| (ShapeIdx(index), entry.clip_rect, text.clone()))
            })
            .collect::<Vec<_>>()
    });
    for (index, clip, mut label) in labels {
        let meshes = color_glyphs(ui, clip, &mut label);
        if !meshes.is_empty() {
            ui.graphics_mut(|layers| {
                let list = layers.entry(layer);
                // Keep a Text shape at the original index for egui label selection.
                list.set(index, clip, Shape::Text(label));
                list.extend(clip, meshes);
            });
        }
    }
}

fn color_glyphs(ui: &Ui, clip: Rect, label: &mut TextShape) -> Vec<Shape> {
    let Some(section) = label.galley.job.sections.first() else {
        return vec![];
    };
    let size = section.format.font_id.size;
    let font = crate::appearance::current_terminal_font(ui.ctx(), size);
    let mut meshes = Vec::new();
    let mut masks = Vec::new();
    let mut end = 0;
    // Shaping can give several glyphs the same character (e.g. both flag
    // indicators get the first indicator's value). Use the original text.
    let displayed_end = label.galley.end().index.0;
    for cluster in label.galley.text().graphemes(true) {
        let begin = end;
        end += cluster.chars().count();
        if end > displayed_end || !crate::ui_text::is_emoji(cluster) {
            continue;
        }
        let cursor = |index, prefer_next_row| {
            label.galley.layout_from_cursor(egui::text::CCursor {
                index: egui::text::CharIndex(index),
                prefer_next_row,
            })
        };
        let first = cursor(begin, true);
        let last = cursor(end, false);
        if first.row != last.row || first.column == last.column {
            continue;
        }
        let row = &label.galley.rows[first.row];
        let origin = label.pos + row.pos.to_vec2();
        let vertices = row.glyphs[first.column.0].first_vertex as usize
            ..row
                .glyphs
                .get(last.column.0)
                .map_or(row.visuals.glyph_vertex_range.end, |g| {
                    g.first_vertex as usize
                });
        let original = &row.visuals.mesh.vertices[vertices.clone()];
        // Code blocks already reserve and paint these slots through ui_text.
        if !original.is_empty()
            && original
                .iter()
                .all(|v| v.pos == Pos2::ZERO || v.color == Color32::TRANSPARENT)
        {
            continue;
        }
        let rect = Rect::from_min_max(
            origin + egui::vec2(row.x_offset(first.column), 0.0),
            origin + egui::vec2(row.x_offset(last.column), row.height()),
        );
        if rect.is_positive()
            && clip.intersects(rect)
            && let Some(mut shape) = font.emoji_shape(ui.ctx(), size, cluster, rect)
        {
            if label.opacity_factor != 1.0
                && let Shape::Mesh(mesh) = &mut shape
            {
                for vertex in &mut Arc::make_mut(mesh).vertices {
                    vertex.color = vertex.color.gamma_multiply(label.opacity_factor);
                }
            }
            meshes.push(shape);
            masks.push((first.row, vertices));
        }
    }
    if !masks.is_empty() {
        let galley = Arc::make_mut(&mut label.galley);
        for (row, vertices) in masks {
            let row = Arc::make_mut(&mut galley.rows[row].row);
            for vertex in &mut row.visuals.mesh.vertices[vertices] {
                vertex.pos = Pos2::ZERO;
            }
        }
    }
    meshes
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn flag_uses_the_original_regional_indicators() {
        let ctx = egui::Context::default();
        crate::appearance::setup(&ctx);
        let _renderer = egui_term::TextRendererSession::new(&ctx);
        let flag = "🇨🇳";
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let galley = ui.painter().layout_no_wrap(
                flag.into(),
                egui::FontId::proportional(18.0),
                Color32::WHITE,
            );
            let row = &galley.rows[0];
            let rect = Rect::from_min_max(
                row.pos + egui::vec2(row.glyphs[0].pos.x, 0.0),
                row.pos + row.size,
            );
            let expected = crate::appearance::current_terminal_font(&ctx, 18.0)
                .emoji_shape(&ctx, 18.0, flag, rect)
                .expect("native flag shape");
            let mut label = TextShape::new(Pos2::ZERO, galley, Color32::WHITE);
            assert_eq!(
                color_glyphs(ui, Rect::EVERYTHING, &mut label),
                vec![expected]
            );
            assert_eq!(label.galley.text(), flag);
        });
    }
}
