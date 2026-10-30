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

#[derive(Clone)]
pub(crate) struct NativeFont {
    pub family: String,
    pub faces: &'static [&'static [u8]],
}
impl std::fmt::Debug for NativeFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFont")
            .field("family", &self.family)
            .field("face_count", &self.faces.len())
            .finish()
    }
}
impl NativeFont {
    pub fn identity(&self) -> usize {
        self.faces
            .first()
            .map(|face| face.as_ptr() as usize)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct TerminalFont {
    font_type: FontId,
    variants: Option<[FontId; 3]>,
    pub(crate) native: Option<std::sync::Arc<NativeFont>>,
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
            native: None,
        }
    }
    pub fn with_variants(mut self, bold: FontId, italic: FontId, bold_italic: FontId) -> Self {
        self.variants = Some([bold, italic, bold_italic]);
        self
    }
    pub fn with_native_font(mut self, family: &str, faces: &'static [&'static [u8]]) -> Self {
        self.native = Some(std::sync::Arc::new(NativeFont {
            family: family.into(),
            faces,
        }));
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
    /// Rasterize at integer physical sizes, not fractional glyph sizes at 125/150% DPI.
    pub fn styled_font_at_scale(&self, bold: bool, italic: bool, scale: f32) -> FontId {
        let mut font = self.styled_font(bold, italic);
        let scale = scale.max(0.1);
        font.size = (font.size * scale).round().max(1.0) / scale;
        font
    }
    /// Ink bounds determine leading; fallback CJK font line boxes must not inflate rows.
    pub fn metrics(&self, ctx: &Context) -> (Size, f32) {
        #[cfg(windows)]
        if let Some(native) = &self.native {
            if let Some(metrics) = crate::native_text::metrics(ctx, native, self.font_type.size) {
                return metrics;
            }
        }
        let scale = ctx.pixels_per_point();
        let font = self.styled_font_at_scale(false, false, scale);
        ctx.fonts_mut(|fonts| {
            let sample =
                fonts.layout_no_wrap("Mgj国".into(), font.clone(), egui::Color32::PLACEHOLDER);
            let width = fonts
                .glyph_width(&font, 'm')
                .max(fonts.glyph_width(&font, '国') / 2.0);
            let width = (width * scale).round().max(1.0) / scale;
            let ink = sample.mesh_bounds;
            let height = ((font.size * 1.2).max(ink.height() + 1.0 / scale) * scale).ceil() / scale;
            let ascent = sample
                .rows
                .first()
                .and_then(|row| row.glyphs.first().map(|g| row.pos.y + g.pos.y))
                .unwrap_or(font.size);
            let baseline =
                ((ascent - ink.top() + (height - ink.height()) / 2.0) * scale).round() / scale;
            (Size::new(width, height), baseline)
        })
    }
    pub fn font_measure(&self, ctx: &Context) -> Size {
        self.metrics(ctx).0
    }
}

/// Own this on the UI thread and release in on_exit, before native window/COM teardown.
pub struct TextRendererSession {
    context: Context,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl TextRendererSession {
    pub fn new(ctx: &Context) -> Self {
        Self {
            context: ctx.clone(),
            _thread: std::marker::PhantomData,
        }
    }
    pub fn release(&mut self) {
        #[cfg(windows)]
        crate::native_text::release(&self.context);
    }
}
impl Drop for TextRendererSession {
    fn drop(&mut self) {
        self.release();
    }
}
