//! Standard egui shaping/input, with native color emoji painted over transparent cluster slots.
use egui::{Color32, FontFamily, FontId, Galley, Painter, Pos2, Rect, Ui, Vec2};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

fn is_emoji(cluster: &str) -> bool {
    // An explicit text-presentation selector must keep the source font and color.
    if cluster.contains('\u{fe0e}') {
        return false;
    }
    cluster.chars().any(|c| {
        matches!(c as u32,
        0x1f000..=0x1faff | 0x2600..=0x27bf | 0xfe0f | 0x20e3 |
        0x00a9 | 0x00ae | 0x203c | 0x2049 | 0x2122 | 0x2139 |
        0x2194..=0x21ff | 0x2300..=0x23ff | 0x2b00..=0x2bff | 0x3030 | 0x303d | 0x3297 | 0x3299)
    })
}

pub fn layout(ui: &Ui, text: &str, size: f32, color: Color32, width: f32) -> Arc<Galley> {
    layout_rows(ui.ctx(), text, size, color, width, 1)
}

pub(crate) fn layout_rows(
    ctx: &egui::Context,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    layout_font_rows(ctx, text, FontId::proportional(size), color, width, rows)
}

pub(crate) fn layout_font_rows(
    ctx: &egui::Context,
    text: &str,
    font_id: FontId,
    color: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width.max(1.0);
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = true;
    let emoji_face = ctx.fonts(|fonts| {
        fonts
            .families()
            .contains(&FontFamily::Name("ui-emoji".into()))
    });
    let mut has_emoji = false;
    for cluster in text.graphemes(true) {
        let emoji = is_emoji(cluster);
        has_emoji |= emoji;
        let font = if emoji && emoji_face {
            FontId::new(font_id.size, FontFamily::Name("ui-emoji".into()))
        } else {
            font_id.clone()
        };
        job.append(
            cluster,
            0.0,
            egui::TextFormat {
                font_id: font,
                color: if emoji { Color32::TRANSPARENT } else { color },
                ..Default::default()
            },
        );
    }
    finish_layout(ctx, job, has_emoji)
}

/// Preserve styled source text while reserving complete grapheme slots for color emoji.
/// Syntax spans may split a ZWJ/variation sequence, so reshape by grapheme before layout.
pub fn layout_job(ctx: &egui::Context, mut job: egui::text::LayoutJob) -> Arc<Galley> {
    if !job.text.graphemes(true).any(is_emoji) {
        return ctx.fonts_mut(|fonts| fonts.layout_job(job));
    }
    let emoji_face = ctx.fonts(|fonts| {
        fonts
            .families()
            .contains(&FontFamily::Name("ui-emoji".into()))
    });
    let original = std::mem::take(&mut job.sections);
    let mut section_index = 0;
    for (start, cluster) in job.text.grapheme_indices(true) {
        let end = egui::text::ByteIndex(start + cluster.len());
        let start = egui::text::ByteIndex(start);
        while section_index < original.len() && original[section_index].byte_range.end <= start {
            section_index += 1;
        }
        let Some(section) = original.get(section_index) else {
            break;
        };
        if is_emoji(cluster) {
            let mut format = section.format.clone();
            format.color = Color32::TRANSPARENT;
            if emoji_face {
                format.font_id.family = FontFamily::Name("ui-emoji".into());
            }
            push_section(
                &mut job.sections,
                egui::text::LayoutSection {
                    leading_space: if start == section.byte_range.start {
                        section.leading_space
                    } else {
                        0.0
                    },
                    byte_range: start..end,
                    format,
                },
            );
        } else {
            let mut cursor = start;
            while cursor < end {
                let section = &original[section_index];
                let mut part = section.clone();
                part.byte_range = cursor..end.min(section.byte_range.end);
                if cursor != section.byte_range.start {
                    part.leading_space = 0.0;
                }
                cursor = part.byte_range.end;
                push_section(&mut job.sections, part);
                if cursor < end {
                    section_index += 1;
                }
            }
        }
    }
    finish_layout(ctx, job, true)
}

fn push_section(sections: &mut Vec<egui::text::LayoutSection>, next: egui::text::LayoutSection) {
    if let Some(previous) = sections.last_mut()
        && previous.byte_range.end == next.byte_range.start
        && next.leading_space == 0.0
        && previous.format == next.format
    {
        previous.byte_range.end = next.byte_range.end;
    } else {
        sections.push(next);
    }
}

fn finish_layout(ctx: &egui::Context, job: egui::text::LayoutJob, has_emoji: bool) -> Arc<Galley> {
    let mut galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
    if has_emoji {
        // TextEdit recolors selected glyph vertices. Degenerate only the transparent
        // outline geometry so selection cannot bring back monochrome emoji ghosts.
        // Keep shaped glyph/cursor positions and selection backgrounds untouched.
        for placed in &mut Arc::make_mut(&mut galley).rows {
            for vertex in &mut Arc::make_mut(&mut placed.row).visuals.mesh.vertices {
                if vertex.color == Color32::TRANSPARENT {
                    vertex.pos = Pos2::ZERO;
                }
            }
        }
    }
    galley
}

pub fn paint_emoji(
    painter: &Painter,
    origin: Pos2,
    galley: &Galley,
    size: f32,
    fallback_color: Color32,
) {
    let font = crate::appearance::current_terminal_font(painter.ctx(), size);
    let mut index = 0;
    for cluster in galley.job.text.graphemes(true) {
        let start = index;
        index += cluster.chars().count();
        if !is_emoji(cluster) {
            continue;
        }
        let from = galley.pos_from_cursor(egui::text::CCursor {
            index: egui::text::CharIndex(start),
            prefer_next_row: true,
        });
        let to = galley.pos_from_cursor(egui::text::CCursor {
            index: egui::text::CharIndex(index),
            prefer_next_row: false,
        });
        let rect = Rect::from_min_max(
            origin + from.min.to_vec2(),
            origin + egui::vec2(to.left(), from.bottom()),
        );
        if rect.width() <= 0.0 || !painter.clip_rect().intersects(rect) {
            continue;
        }
        if let Some(shape) = font.emoji_shape(painter.ctx(), size, cluster, rect) {
            painter.add(shape);
        } else {
            let family = if painter
                .ctx()
                .fonts(|f| f.families().contains(&FontFamily::Name("ui-emoji".into())))
            {
                FontFamily::Name("ui-emoji".into())
            } else {
                FontFamily::Proportional
            };
            painter
                .with_clip_rect(rect.intersect(painter.clip_rect()))
                .text(
                    rect.min,
                    egui::Align2::LEFT_TOP,
                    cluster,
                    FontId::new(size, family),
                    fallback_color,
                );
        }
    }
}

pub fn paint(
    ui: &Ui,
    painter: &Painter,
    pos: Pos2,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
) -> Vec2 {
    let galley = layout(ui, text, size, color, width);
    painter.galley(pos, galley.clone(), color);
    paint_emoji(painter, pos, &galley, size, color);
    galley.size()
}

pub fn label(ui: &mut Ui, text: &str, size: f32, color: Color32) -> egui::Response {
    let width = ui.available_width();
    label_with_width(ui, text, size, color, width)
}

pub fn label_with_width(
    ui: &mut Ui,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
) -> egui::Response {
    label_with_font(ui, text, FontId::proportional(size), color, width)
}

pub fn label_with_font(
    ui: &mut Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
) -> egui::Response {
    let size = font.size;
    let galley = layout_font_rows(ui.ctx(), text, font, color, width, 1);
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text));
    ui.painter().galley(rect.min, galley.clone(), color);
    paint_emoji(ui.painter(), rect.min, &galley, size, color);
    response
}

pub fn edit(ui: &mut Ui, text: &mut String, id: egui::Id) -> egui::text_edit::TextEditOutput {
    edit_impl(ui, text, id, None)
}

pub fn edit_sized(
    ui: &mut Ui,
    text: &mut String,
    id: egui::Id,
    width: f32,
) -> egui::text_edit::TextEditOutput {
    edit_impl(ui, text, id, Some(width))
}

fn edit_impl(
    ui: &mut Ui,
    text: &mut String,
    id: egui::Id,
    width: Option<f32>,
) -> egui::text_edit::TextEditOutput {
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    let color = ui.visuals().text_color();
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _: f32| {
        layout(ui, text.as_str(), size, color, f32::INFINITY)
    };
    let mut edit = egui::TextEdit::singleline(text)
        .id(id)
        .layouter(&mut layouter);
    if let Some(width) = width {
        edit = edit
            .desired_width(width)
            .margin(egui::Margin::symmetric(10, 8));
    }
    let output = edit.show(ui);
    paint_emoji(
        &ui.painter().with_clip_rect(output.text_clip_rect),
        output.galley_pos,
        &output.galley,
        size,
        color,
    );
    output
}
