use crate::types::Size;
use egui::{Context, FontId};

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub font_type: FontId,
}
impl Default for FontSettings {
    fn default() -> Self {
        Self {
            font_type: FontId::monospace(14.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalFont {
    font_type: FontId,
    variants: Option<[FontId; 3]>,
}
impl Default for TerminalFont {
    fn default() -> Self {
        Self::new(FontSettings::default())
    }
}
impl TerminalFont {
    pub fn new(settings: FontSettings) -> Self {
        Self {
            font_type: settings.font_type,
            variants: None,
        }
    }
    pub fn with_variants(mut self, bold: FontId, italic: FontId, bold_italic: FontId) -> Self {
        self.variants = Some([bold, italic, bold_italic]);
        self
    }
    pub fn font_type(&self) -> FontId {
        self.font_type.clone()
    }
    pub fn styled_font(&self, bold: bool, italic: bool) -> FontId {
        match (&self.variants, bold, italic) {
            (Some(fonts), true, true) => fonts[2].clone(),
            (Some(fonts), true, false) => fonts[0].clone(),
            (Some(fonts), false, true) => fonts[1].clone(),
            _ => self.font_type(),
        }
    }
    /// Logical dimensions come from a physical pixel grid, including 125/150% DPI.
    pub fn metrics(&self, ctx: &Context) -> (Size, f32) {
        let scale = ctx.pixels_per_point();
        ctx.fonts_mut(|fonts| {
            let sample =
                fonts.layout_no_wrap("Mg国".into(), self.font_type(), egui::Color32::PLACEHOLDER);
            let width = fonts
                .glyph_width(&self.font_type, 'm')
                .max(fonts.glyph_width(&self.font_type, '国') / 2.0);
            let width = (width * scale).round().max(1.0) / scale;
            let height =
                ((sample.size().y.max(self.font_type.size * 1.35) + 1.0) * scale).ceil() / scale;
            let ascent = sample
                .rows
                .first()
                .and_then(|row| row.glyphs.first().map(|g| row.pos.y + g.pos.y))
                .unwrap_or(self.font_type.size);
            let baseline = ((ascent + (height - sample.size().y) / 2.0) * scale).round() / scale;
            (Size::new(width, height), baseline)
        })
    }
    pub fn font_measure(&self, ctx: &Context) -> Size {
        self.metrics(ctx).0
    }
}
