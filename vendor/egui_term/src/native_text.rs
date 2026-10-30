//! Windows text rasterization. COM stays on the UI thread; only bounded GPU atlases persist.
use crate::{font::NativeFont, types::Size};
use egui::{Color32, Context, Pos2, Rect, Shape, TextureHandle, TextureOptions};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use windows::{
    core::{w, Interface, PCWSTR},
    Win32::{
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Imaging::*,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
    },
};

const ATLAS: usize = 1024;
const MAX_PAGES: usize = 8;
const SURFACE_W: u32 = 256;
const SURFACE_H: u32 = 160;

#[derive(Clone, Hash, PartialEq, Eq)]
struct Key {
    text: String,
    pixels: u16,
    style: u8,
    width: u16,
    height: u16,
    baseline: u16,
    fg: [u8; 4],
    bg: [u8; 4],
}
#[derive(Clone, Copy)]
struct Tile {
    texture: egui::TextureId,
    uv: Rect,
}
struct Page {
    texture: TextureHandle,
    x: usize,
    y: usize,
    row: usize,
}
struct Engine {
    factory: IDWriteFactory5,

    collection: Option<IDWriteFontCollection>,
    system_collection: IDWriteFontCollection,
    bitmap: IWICBitmap,
    target: ID2D1RenderTarget,
    brush: ID2D1SolidColorBrush,
    formats: HashMap<(u16, u8), IDWriteTextFormat>,
    metrics: HashMap<u16, (Size, f32)>,
    tiles: HashMap<Key, Tile>,
    pages: Vec<Page>,
    family: String,
    identity: usize,
    _registration: Option<RegisteredFonts>,
    _apartment: Apartment,
}
struct Apartment(bool);
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
struct RegisteredFonts {
    factory: IDWriteFactory5,
    loader: IDWriteInMemoryFontFileLoader,
}
impl Drop for RegisteredFonts {
    fn drop(&mut self) {
        unsafe {
            let _ = self.factory.UnregisterFontFileLoader(&self.loader);
        }
    }
}
thread_local! {static ENGINES:RefCell<VecDeque<(usize,Option<Engine>)>>=const {RefCell::new(VecDeque::new())};}

pub(super) fn release(ctx: &Context) {
    let manager = ctx.tex_manager();
    let id = Arc::as_ptr(&manager) as usize;
    ENGINES.with(|engines| engines.borrow_mut().retain(|(key, _)| *key != id));
}

fn with_engine<T>(
    ctx: &Context,
    font: &NativeFont,
    run: impl FnOnce(&mut Engine) -> windows::core::Result<T>,
) -> Option<T> {
    let manager = ctx.tex_manager();
    let id = Arc::as_ptr(&manager) as usize;
    ENGINES.with(|engines| {
        let mut engines = engines.borrow_mut();
        let index = engines.iter().position(|(key, _)| *key == id);
        let index = match index {
            Some(i) => i,
            None => {
                if engines.len() == 2 {
                    engines.pop_front();
                }
                engines.push_back((id, Engine::new(font).ok()));
                engines.len() - 1
            }
        };
        let engine = engines[index].1.as_mut()?;
        if engine.family != font.family || engine.identity != font.identity() {
            engines[index].1 = Engine::new(font).ok();
        }
        run(engines[index].1.as_mut()?).ok()
    })
}

impl Engine {
    fn new(font: &NativeFont) -> windows::core::Result<Self> {
        // eframe may have initialized a different COM apartment already; both support WIC.
        unsafe {
            let apartment = Apartment(CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok());
            let factory: IDWriteFactory5 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_ISOLATED)?;
            let (registration, collection) = if font.faces.is_empty() {
                (None, None)
            } else {
                let loader = factory.CreateInMemoryFontFileLoader()?;
                factory.RegisterFontFileLoader(&loader)?;
                let guard = RegisteredFonts {
                    factory: factory.clone(),
                    loader: loader.clone(),
                };
                let builder = factory.CreateFontSetBuilder()?;
                for bytes in font.faces {
                    let file = loader.CreateInMemoryFontFileReference(
                        &factory,
                        bytes.as_ptr().cast(),
                        bytes.len() as u32,
                        None::<&windows::core::IUnknown>,
                    )?;
                    builder.AddFontFile(&file)?;
                }
                let set = builder.CreateFontSet()?;
                let collection = factory
                    .CreateFontCollectionFromFontSet(&set)?
                    .cast::<IDWriteFontCollection>()?;
                (Some(guard), Some(collection))
            };
            let mut system = None;
            factory
                .cast::<IDWriteFactory>()?
                .GetSystemFontCollection(&mut system, false)?;
            let system_collection = system.expect("DirectWrite system collection");
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory2, None, CLSCTX_INPROC_SERVER)?;
            let bitmap = wic.CreateBitmap(
                SURFACE_W,
                SURFACE_H,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapCacheOnLoad,
            )?;
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let target = d2d.CreateWicBitmapRenderTarget(
                &bitmap,
                &D2D1_RENDER_TARGET_PROPERTIES {
                    r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                    ..Default::default()
                },
            )?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let params = factory.CreateRenderingParams()?;
            target.SetTextRenderingParams(&params);
            let brush = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                None,
            )?;
            Ok(Self {
                factory,
                collection,
                system_collection,
                bitmap,
                target,
                brush,
                formats: HashMap::new(),
                metrics: HashMap::new(),
                tiles: HashMap::new(),
                pages: Vec::new(),
                family: font.family.clone(),
                identity: font.identity(),
                _registration: registration,
                _apartment: apartment,
            })
        }
    }
    fn format(&mut self, pixels: u16, style: u8) -> windows::core::Result<IDWriteTextFormat> {
        if let Some(format) = self.formats.get(&(pixels, style)) {
            return Ok(format.clone());
        }
        unsafe {
            let name: Vec<u16> = self.family.encode_utf16().chain(Some(0)).collect();
            let format = self.factory.CreateTextFormat(
                PCWSTR(name.as_ptr()),
                self.collection.as_ref(),
                if style & 1 != 0 {
                    DWRITE_FONT_WEIGHT_BOLD
                } else {
                    DWRITE_FONT_WEIGHT_NORMAL
                },
                if style & 2 != 0 {
                    DWRITE_FONT_STYLE_ITALIC
                } else {
                    DWRITE_FONT_STYLE_NORMAL
                },
                DWRITE_FONT_STRETCH_NORMAL,
                pixels as f32,
                w!("en-US"),
            )?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            if self.formats.len() >= 64 {
                self.formats.clear();
            }
            self.formats.insert((pixels, style), format.clone());
            Ok(format)
        }
    }
    fn layout(
        &mut self,
        text: &str,
        pixels: u16,
        style: u8,
    ) -> windows::core::Result<(IDWriteTextLayout, f32, f32)> {
        let format = self.format(pixels, style)?;
        unsafe {
            let cjk=text.chars().any(|c|matches!(c as u32,0x2e80..=0x9fff|0xf900..=0xfaff|0xff00..=0xffef|0x20000..=0x3134f));
            let text: Vec<u16> = text.encode_utf16().collect();
            let layout = self
                .factory
                .CreateTextLayout(&text, &format, 4096.0, 512.0)?;
            if cjk {
                let range = DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: text.len() as u32,
                };
                layout.SetFontCollection(&self.system_collection, range)?;
                layout.SetFontFamilyName(w!("Microsoft YaHei UI"), range)?;
            }
            let fallback = self.factory.GetSystemFontFallback()?;
            layout
                .cast::<IDWriteTextLayout2>()?
                .SetFontFallback(&fallback)?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;
            let mut line = [DWRITE_LINE_METRICS::default(); 1];
            let mut count = 0;
            layout.GetLineMetrics(Some(&mut line), &mut count)?;
            Ok((
                layout,
                line[0].baseline,
                metrics.widthIncludingTrailingWhitespace,
            ))
        }
    }
    fn measure(&mut self, pixels: u16) -> windows::core::Result<(Size, f32)> {
        if let Some(metrics) = self.metrics.get(&pixels) {
            return Ok(*metrics);
        }
        let (_, baseline, width) = self.layout("MMMM", pixels, 0)?;
        let height = (pixels as f32 * 1.2).ceil();
        let value = (
            Size::new((width / 4.0).round().max(1.0), height),
            baseline.round(),
        );
        if self.metrics.len() >= 64 {
            self.metrics.clear();
        }
        self.metrics.insert(pixels, value);
        Ok(value)
    }
    fn raster(&mut self, key: &Key) -> windows::core::Result<Vec<u8>> {
        let (layout, baseline, natural_width) = self.layout(&key.text, key.pixels, key.style)?;
        let color = |rgba: [u8; 4]| D2D1_COLOR_F {
            r: rgba[0] as f32 / 255.0,
            g: rgba[1] as f32 / 255.0,
            b: rgba[2] as f32 / 255.0,
            a: rgba[3] as f32 / 255.0,
        };
        unsafe {
            let clip = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: key.width as f32,
                bottom: key.height as f32,
            };
            self.target.BeginDraw();
            self.target
                .PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_ALIASED);
            self.target.Clear(Some(&color(key.bg)));
            self.brush.SetColor(&color(key.fg));
            let left = if key.text.chars().any(|c| c as u32 >= 0x2E80) {
                ((key.width as f32 - natural_width) / 2.0).max(0.0).round()
            } else {
                0.0
            };
            self.target.DrawTextLayout(
                windows_numerics::Vector2 {
                    X: left,
                    Y: key.baseline as f32 - baseline,
                },
                &layout,
                &self.brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
            self.target.PopAxisAlignedClip();
            self.target.EndDraw(None, None)?;
            let mut bgra = vec![0; key.width as usize * key.height as usize * 4];
            self.bitmap.CopyPixels(
                &WICRect {
                    X: 0,
                    Y: 0,
                    Width: key.width as i32,
                    Height: key.height as i32,
                },
                key.width as u32 * 4,
                &mut bgra,
            )?;
            for pixel in bgra.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
                pixel[3] = 255;
            }
            Ok(bgra)
        }
    }
    fn tile(&mut self, ctx: &Context, key: Key) -> windows::core::Result<Tile> {
        if let Some(tile) = self.tiles.get(&key) {
            return Ok(*tile);
        }
        let pixels = self.raster(&key)?;
        let width = key.width as usize;
        let height = key.height as usize;
        let fits = |page: &Page| {
            page.y + page.row.max(height) < ATLAS
                && (page.x + width < ATLAS || page.y + page.row + height + 2 <= ATLAS)
        };
        if !self.pages.last().is_some_and(fits) {
            if self.pages.len() >= MAX_PAGES {
                self.pages.clear();
                self.tiles.clear();
            }
            let texture = ctx.load_texture(
                format!("native-text-{}", self.pages.len()),
                egui::ColorImage::filled([ATLAS, ATLAS], Color32::TRANSPARENT),
                TextureOptions::NEAREST,
            );
            self.pages.push(Page {
                texture,
                x: 0,
                y: 0,
                row: 0,
            });
        }
        let page = self.pages.last_mut().unwrap();
        if page.x + width + 1 > ATLAS {
            page.y += page.row + 1;
            page.x = 0;
            page.row = 0;
        }
        let pos = [page.x, page.y];
        page.texture.set_partial(
            pos,
            egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels),
            TextureOptions::NEAREST,
        );
        let uv = Rect::from_min_max(
            egui::pos2(pos[0] as f32 / ATLAS as f32, pos[1] as f32 / ATLAS as f32),
            egui::pos2(
                (pos[0] + width) as f32 / ATLAS as f32,
                (pos[1] + height) as f32 / ATLAS as f32,
            ),
        );
        let tile = Tile {
            texture: page.texture.id(),
            uv,
        };
        page.x += width + 1;
        page.row = page.row.max(height);
        self.tiles.insert(key, tile);
        Ok(tile)
    }
}

pub fn metrics(ctx: &Context, font: &NativeFont, size: f32) -> Option<(Size, f32)> {
    let scale = ctx.pixels_per_point();
    let pixels = (size * scale).round().clamp(1.0, 128.0) as u16;
    with_engine(ctx, font, |engine| engine.measure(pixels)).map(|(size, baseline)| {
        (
            Size::new(size.width / scale, size.height / scale),
            baseline / scale,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub fn shape(
    ctx: &Context,
    font: &NativeFont,
    size: f32,
    text: &str,
    rect: Rect,
    baseline: f32,
    bold: bool,
    italic: bool,
    fg: Color32,
    bg: Color32,
) -> Option<Shape> {
    let scale = ctx.pixels_per_point();
    let width = (rect.width() * scale).round() as u16;
    let height = (rect.height() * scale).round() as u16;
    if width == 0 || height == 0 || width as u32 > SURFACE_W || height as u32 > SURFACE_H {
        return None;
    }
    let key = Key {
        text: text.into(),
        pixels: (size * scale).round().clamp(1.0, 128.0) as u16,
        style: u8::from(bold) | u8::from(italic) << 1,
        width,
        height,
        baseline: (baseline * scale).round() as u16,
        fg: fg.to_srgba_unmultiplied(),
        bg: bg.to_srgba_unmultiplied(),
    };
    with_engine(ctx, font, |engine| engine.tile(ctx, key)).map(|tile| {
        let mut mesh = egui::Mesh::with_texture(tile.texture);
        let min = Pos2::new(
            (rect.min.x * scale).round() / scale,
            (rect.min.y * scale).round() / scale,
        );
        mesh.add_rect_with_uv(
            Rect::from_min_size(min, egui::vec2(width as f32 / scale, height as f32 / scale)),
            tile.uv,
            Color32::WHITE,
        );
        Shape::mesh(mesh)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn native() -> NativeFont {
        NativeFont {
            family: "Consolas".into(),
            faces: &[],
        }
    }
    #[test]
    fn windows_color_emoji_is_multicolor_and_styles_are_distinct() {
        let mut engine = Engine::new(&native()).expect("DirectWrite/Direct2D must initialize");
        let base = Key {
            text: "👋".into(),
            pixels: 28,
            style: 0,
            width: 64,
            height: 42,
            baseline: 30,
            fg: [215, 215, 215, 255],
            bg: [0, 0, 0, 255],
        };
        let rgba = engine.raster(&base).unwrap();
        assert!(
            rgba.as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[0] > 180 && p[1] > 130 && p[2] < 90)
                .count()
                > 50,
            "emoji was rendered as a monochrome fallback"
        );
        let regular = Key {
            text: "Hack".into(),
            width: 84,
            ..base.clone()
        };
        let bold = Key {
            style: 1,
            ..regular.clone()
        };
        assert_ne!(
            engine.raster(&regular).unwrap(),
            engine.raster(&bold).unwrap()
        );
    }
}
