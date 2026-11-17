//! Bounded image decoding and Markdown preparation, called only by the file worker.
use crate::{
    git::{FilePatch, PatchKind, PatchLine, syntax},
    project_files::{FileError, plain_content, resolve},
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_SOURCE_PIXELS: u64 = 32 * 1024 * 1024;
pub const MAX_INLINE_IMAGES: usize = 16;
pub const MAX_INLINE_PIXELS: usize = 16 * 1024 * 1024;
#[derive(Debug)]
pub struct PreviewImage {
    pub pixels: Arc<egui::ColorImage>,
    pub original: [u32; 2],
    pub format: String,
    pub first_frame: bool,
}
#[derive(Debug)]
pub struct Markdown {
    pub text: String,
    pub links: Vec<String>,
    pub images: BTreeMap<String, Result<Arc<PreviewImage>, FileError>>,
    pub code: Vec<FilePatch>,
    pub code_marker: String,
}
fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}
pub fn markdown_path(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "md" | "markdown" | "mdown" | "mkd" | "mdx"
    )
}
pub fn image_path(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "png" | "jpg" | "jpeg" | "jpe" | "webp" | "gif" | "bmp" | "ico" | "tif" | "tiff" | "svg"
    )
}
pub fn unsupported(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "doc"
            | "docx"
            | "docm"
            | "xls"
            | "xlsx"
            | "xlsm"
            | "ppt"
            | "pptx"
            | "pptm"
            | "odt"
            | "ods"
            | "odp"
    )
}
pub fn read_image(
    root: &Path,
    path: &Path,
    inline: bool,
    cancelled: &impl Fn() -> bool,
) -> Result<PreviewImage, FileError> {
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    let full = resolve(root, path)?;
    let file = fs::File::open(full)?;
    if !file.metadata()?.is_file() {
        return Err(FileError::Binary);
    }
    if file.metadata()?.len() > MAX_IMAGE_BYTES {
        return Err(FileError::TooLarge);
    }
    let mut bytes = vec![];
    file.take(MAX_IMAGE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(FileError::TooLarge);
    }
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    let max_edge = if inline { 2048 } else { 4096 };
    let image = if extension(path) == "svg" {
        decode_svg(&bytes, max_edge)?
    } else {
        decode_raster(&bytes, max_edge)?
    };
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    Ok(image)
}
fn decode_raster(bytes: &[u8], max_edge: u32) -> Result<PreviewImage, FileError> {
    let format = image::guess_format(bytes).map_err(|e| FileError::Image(e.to_string()))?;
    let (width, height) = image::ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| FileError::Image(e.to_string()))?;
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS
    {
        return Err(FileError::ImageLimit);
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| FileError::Image(e.to_string()))?;
    let image = if width > max_edge || height > max_edge {
        image.thumbnail(max_edge, max_edge)
    } else {
        image
    };
    let rgba = image.into_rgba8();
    Ok(PreviewImage {
        pixels: Arc::new(egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        )),
        original: [width, height],
        format: format!("{format:?}").to_uppercase(),
        first_frame: matches!(format, image::ImageFormat::Gif | image::ImageFormat::WebP),
    })
}
fn decode_svg(bytes: &[u8], max_edge: u32) -> Result<PreviewImage, FileError> {
    if bytes.len() > 4 * 1024 * 1024
        || bytes.starts_with(&[0x1f, 0x8b])
        || bytes.iter().filter(|&&b| b == b'<').count() > 10_000
    {
        return Err(FileError::ImageLimit);
    }
    let mut options = resvg::usvg::Options {
        font_family: "JetBrains Mono".into(),
        ..Default::default()
    };
    // User SVGs cannot read files, follow nested SVGs, fetch URLs, or expand data images.
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    options.image_href_resolver.resolve_data = Box::new(|_, _, _| None);
    options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").to_vec());
    #[cfg(windows)]
    if let Some(windows) = std::env::var_os("WINDIR") {
        let fonts = PathBuf::from(windows).join("Fonts");
        for name in ["segoeui.ttf", "msyh.ttc"] {
            if let Ok(bytes) = fs::read(fonts.join(name)) {
                options.fontdb_mut().load_font_data(bytes);
            }
        }
    }
    let tree = resvg::usvg::Tree::from_data(bytes, &options)
        .map_err(|e| FileError::Image(e.to_string()))?;
    let original = tree.size();
    let scale = (max_edge as f32 / original.width().max(original.height())).min(1.0);
    let width = (original.width() * scale).ceil().max(1.0) as u32;
    let height = (original.height() * scale).ceil().max(1.0) as u32;
    let mut bitmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or(FileError::ImageLimit)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut bitmap.as_mut(),
    );
    Ok(PreviewImage {
        pixels: Arc::new(egui::ColorImage::from_rgba_premultiplied(
            [width as usize, height as usize],
            bitmap.data(),
        )),
        original: [
            original.width().ceil() as u32,
            original.height().ceil() as u32,
        ],
        format: "SVG".into(),
        first_frame: false,
    })
}

/// URI policy is shared by Markdown images and internal file links. No file/network I/O here.
pub fn local_target(document: &Path, uri: &str) -> Result<PathBuf, FileError> {
    if uri.starts_with(['/', '\\']) || uri.contains(':') || uri.contains('\0') {
        return Err(FileError::OutsideProject);
    }
    let uri = uri.split(['#', '?']).next().unwrap_or("");
    let bytes = uri.as_bytes();
    let mut decoded = vec![];
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or(FileError::OutsideProject)?;
            let text = std::str::from_utf8(hex).map_err(|_| FileError::OutsideProject)?;
            decoded.push(u8::from_str_radix(text, 16).map_err(|_| FileError::OutsideProject)?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(decoded)
        .map_err(|_| FileError::OutsideProject)?
        .replace('\\', "/");
    if decoded.contains(['\0', ':']) {
        return Err(FileError::OutsideProject);
    }
    let mut result = document.parent().unwrap_or(Path::new("")).to_path_buf();
    for part in Path::new(&decoded).components() {
        match part {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir if result.pop() => {}
            _ => return Err(FileError::OutsideProject),
        }
    }
    if result.as_os_str().is_empty() {
        return Err(FileError::OutsideProject);
    }
    Ok(result)
}
pub fn external_link(uri: &str) -> bool {
    let lower = uri.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:"))
        && !uri.chars().any(char::is_control)
}
fn code_path(info: &str) -> String {
    let language = info.split_whitespace().next().unwrap_or("txt");
    let ext = match language {
        "rust" => "rs",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        "powershell" => "ps1",
        "bash" | "shell" => "sh",
        "c++" => "cpp",
        other => other,
    };
    format!(
        "code.{}",
        ext.chars()
            .filter(char::is_ascii_alphanumeric)
            .take(24)
            .collect::<String>()
    )
}
pub fn prepare_markdown(
    root: &Path,
    document: &Path,
    text: &str,
    cancelled: &impl Fn() -> bool,
) -> Result<Option<Markdown>, FileError> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut depth = 0usize;
    let mut links = BTreeSet::new();
    let mut image_uris = BTreeSet::new();
    let mut code = vec![];
    let mut replacements = vec![];
    let mut block: Option<(usize, String, String)> = None;
    let marker = format!("ai-terminal-code-{}", uuid::Uuid::new_v4().simple());
    for (index, (event, span)) in Parser::new_ext(text, options)
        .into_offset_iter()
        .enumerate()
    {
        if cancelled() {
            return Err(FileError::Cancelled);
        }
        if index > 16_000 {
            return Ok(None);
        }
        match event {
            Event::Start(tag) => {
                depth += 1;
                if depth > 32 {
                    return Ok(None);
                }
                match tag {
                    Tag::Link { dest_url, .. } => {
                        links.insert(dest_url.to_string());
                    }
                    Tag::Image { dest_url, .. } => {
                        image_uris.insert(dest_url.to_string());
                        if image_uris.len() > 128 {
                            return Ok(None);
                        }
                    }
                    Tag::CodeBlock(kind) => {
                        let info = match kind {
                            CodeBlockKind::Fenced(info) => info.to_string(),
                            CodeBlockKind::Indented => "txt".into(),
                        };
                        block = Some((span.start, info, String::new()));
                    }
                    _ => {}
                }
            }
            Event::Text(part) if block.is_some() => {
                block.as_mut().unwrap().2.push_str(&part);
            }
            Event::End(end) => {
                depth = depth.saturating_sub(1);
                if end == TagEnd::CodeBlock
                    && let Some((start, info, body)) = block.take()
                {
                    let mut patch = plain_content(&code_path(&info));
                    for (index, line) in body
                        .strip_suffix('\n')
                        .unwrap_or(&body)
                        .split('\n')
                        .enumerate()
                    {
                        patch.lines.push(PatchLine {
                            kind: PatchKind::Context,
                            old: None,
                            new: Some(index as u64 + 1),
                            text: syntax::display_text(line),
                        });
                    }
                    patch.presentation = syntax::prepare(&patch.path, &patch.lines, cancelled);
                    replacements
                        .push((start..span.end, format!("\n<{marker}-{} />\n", code.len())));
                    code.push(patch);
                }
            }
            _ => {}
        }
    }
    let mut rendered = text.to_owned();
    for (span, replacement) in replacements.into_iter().rev() {
        rendered.replace_range(span, &replacement);
    }
    let mut images = BTreeMap::new();
    let mut pixels = 0;
    for (index, uri) in image_uris.into_iter().enumerate() {
        if cancelled() {
            return Err(FileError::Cancelled);
        }
        let image = if index >= MAX_INLINE_IMAGES {
            Err(FileError::ImageLimit)
        } else {
            local_target(document, &uri).and_then(|path| read_image(root, &path, true, cancelled))
        }
        .and_then(|image| {
            let count = image.pixels.pixels.len();
            if pixels + count > MAX_INLINE_PIXELS {
                return Err(FileError::ImageLimit);
            }
            pixels += count;
            Ok(Arc::new(image))
        });
        if matches!(image, Err(FileError::Cancelled)) {
            return Err(FileError::Cancelled);
        }
        images.insert(uri, image);
    }
    Ok(Some(Markdown {
        text: rendered,
        links: links.into_iter().collect(),
        images,
        code,
        code_marker: marker,
    }))
}
