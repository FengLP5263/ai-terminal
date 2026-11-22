//! Prepare bounded native Markdown blocks without changing the source document.
mod html;
use super::*;
use crate::{
    git::{PatchKind, PatchLine, syntax},
    project_files::plain_content,
};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::collections::BTreeSet;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct MarkdownTable {
    pub alignment: Vec<Alignment>,
    /// Header first, followed by body rows. Cells remain Markdown, not HTML.
    pub rows: Vec<Vec<String>>,
}
#[derive(Debug)]
pub struct MarkdownHeading {
    pub level: u8,
    pub text: String,
    pub anchor: String,
}

pub(super) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES
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
    if text.len() > 256 * 1024 {
        return Ok(None);
    }
    let marker = format!("ai-terminal-code-{}", uuid::Uuid::new_v4().simple());
    let mut md = Markdown {
        text: String::new(),
        has_emoji: text.graphemes(true).any(crate::ui_text::is_emoji),
        links: vec![],
        images: BTreeMap::new(),
        code: vec![],
        code_sources: vec![],
        code_marker: marker.clone(),
        tables: vec![],
        centered: vec![],
        headings: vec![],
    };
    let mut depth = 0usize;
    let mut replacements = vec![];
    let mut code: Option<(usize, String, String)> = None;
    let mut table: Option<(usize, MarkdownTable)> = None;
    let mut heading = None;
    let mut heading_counts = BTreeMap::new();
    for (index, (event, span)) in Parser::new_ext(text, options())
        .into_offset_iter()
        .enumerate()
    {
        if cancelled() {
            return Err(FileError::Cancelled);
        }
        if index > 16_000 {
            return Ok(None);
        }
        if !md.has_emoji
            && let Event::Text(value) = &event
        {
            md.has_emoji = value.graphemes(true).any(crate::ui_text::is_emoji);
        }
        match event {
            Event::Start(tag) => {
                depth += 1;
                if depth > 32 {
                    return Ok(None);
                }
                match tag {
                    Tag::CodeBlock(kind) => {
                        let info = match kind {
                            CodeBlockKind::Fenced(info) => info.to_string(),
                            CodeBlockKind::Indented => "txt".into(),
                        };
                        code = Some((span.start, info, String::new()));
                    }
                    Tag::Table(alignment) => {
                        if alignment.len() > 32 {
                            return Ok(None);
                        }
                        table = Some((
                            span.start,
                            MarkdownTable {
                                alignment,
                                rows: vec![],
                            },
                        ))
                    }
                    Tag::TableHead | Tag::TableRow => {
                        if let Some((_, table)) = &mut table {
                            table.rows.push(vec![]);
                        }
                    }
                    Tag::TableCell => {
                        if let Some((_, table)) = &mut table {
                            table
                                .rows
                                .last_mut()
                                .unwrap()
                                .push(text[span].trim().to_owned());
                        }
                    }
                    Tag::Heading { level, id, .. } => {
                        heading = Some((span.start, level as u8, id.map(|s| s.to_string())))
                    }
                    _ => {}
                }
            }
            Event::Text(part) if code.is_some() => code.as_mut().unwrap().2.push_str(&part),
            Event::End(end) => {
                depth = depth.saturating_sub(1);
                if end == TagEnd::CodeBlock
                    && let Some((start, info, body)) = code.take()
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
                    replacements.push((
                        start..span.end,
                        format!("\n<{marker}-{} />\n", md.code.len()),
                    ));
                    md.code.push(patch);
                    md.code_sources
                        .push(body.strip_suffix('\n').unwrap_or(&body).to_owned());
                } else if end == TagEnd::Table
                    && let Some((start, table)) = table.take()
                {
                    replacements.push((
                        start..span.end,
                        format!("\n<{marker}-table-{} />\n", md.tables.len()),
                    ));
                    md.tables.push(table);
                } else if matches!(end, TagEnd::Heading(_))
                    && let Some((start, level, explicit)) = heading.take()
                {
                    let raw = text[start..span.end].trim();
                    let body = if raw.starts_with('#') {
                        raw.trim_start_matches('#')
                            .trim()
                            .trim_end_matches('#')
                            .trim()
                    } else {
                        raw.rsplit_once('\n').map_or(raw, |(body, _)| body).trim()
                    };
                    let body = body.rsplit_once(" {#").map_or(body, |(body, _)| body);
                    let plain = Parser::new(body)
                        .filter_map(|e| match e {
                            Event::Text(s) | Event::Code(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect::<String>();
                    let base = explicit.unwrap_or_else(|| {
                        plain
                            .to_lowercase()
                            .chars()
                            .filter_map(|c| {
                                if c.is_whitespace() {
                                    Some('-')
                                } else if c.is_alphanumeric() || c == '-' || c == '_' {
                                    Some(c)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    });
                    let count = heading_counts.entry(base.clone()).or_insert(0);
                    let anchor = if *count == 0 {
                        base.clone()
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    replacements.push((
                        start..span.end,
                        format!("\n<{marker}-heading-{} />\n", md.headings.len()),
                    ));
                    md.headings.push(MarkdownHeading {
                        level,
                        text: body.into(),
                        anchor,
                    });
                }
            }
            _ => {}
        }
    }
    let mut rendered = text.to_owned();
    for (span, replacement) in replacements.into_iter().rev() {
        rendered.replace_range(span, &replacement);
    }
    let mut html = html::Normalizer::new(&marker);
    md.text = html.normalize(&rendered, 0, cancelled)?;
    for table in &mut md.tables {
        for row in &mut table.rows {
            for cell in row {
                *cell = html.normalize(cell, 0, cancelled)?;
            }
        }
    }
    for heading in &mut md.headings {
        heading.text = html.normalize(&heading.text, 0, cancelled)?;
    }
    md.centered = html.centered;
    let mut links = BTreeSet::new();
    let mut image_uris = vec![];
    let mut seen_images = BTreeSet::new();
    let mut event_count = 0;
    for fragment in std::iter::once(&md.text)
        .chain(md.centered.iter())
        .chain(md.tables.iter().flat_map(|t| t.rows.iter().flatten()))
        .chain(md.headings.iter().map(|h| &h.text))
    {
        let mut rendered_depth = 0usize;
        for event in Parser::new_ext(fragment, options()) {
            if cancelled() {
                return Err(FileError::Cancelled);
            }
            event_count += 1;
            if event_count > 16_000 {
                return Ok(None);
            }
            match &event {
                Event::Start(_) => {
                    rendered_depth += 1;
                    if rendered_depth > 32 {
                        return Ok(None);
                    }
                }
                Event::End(_) => rendered_depth = rendered_depth.saturating_sub(1),
                _ => {}
            }
            match event {
                Event::Start(Tag::Link { dest_url, .. }) => {
                    links.insert(dest_url.to_string());
                }
                Event::Start(Tag::Image { dest_url, .. })
                    if seen_images.insert(dest_url.to_string()) =>
                {
                    image_uris.push(dest_url.to_string());
                }
                _ => {}
            }
            if image_uris.len() > 128 {
                return Ok(None);
            }
        }
    }
    image_uris.sort_by_key(|uri| !html.images.contains_key(uri));
    let mut pixels = 0;
    let mut decoded_images = 0;
    for uri in image_uris {
        if cancelled() {
            return Err(FileError::Cancelled);
        }
        let spec = html.images.get(&uri);
        let source = spec.map_or(uri.as_str(), |spec| spec.source.as_str());
        let image = local_target(document, source)
            .and_then(|path| {
                if decoded_images >= MAX_INLINE_IMAGES {
                    return Err(FileError::ImageLimit);
                }
                decoded_images += 1;
                read_image(root, &path, true, cancelled)
            })
            .and_then(|mut image| {
                let count = image.pixels.pixels.len();
                if pixels + count > MAX_INLINE_PIXELS {
                    return Err(FileError::ImageLimit);
                }
                pixels += count;
                if let Some(spec) = spec {
                    let size = spec.size(image.original);
                    Arc::make_mut(&mut image.pixels).source_size = size;
                }
                Ok(Arc::new(image))
            });
        if matches!(image, Err(FileError::Cancelled)) {
            return Err(FileError::Cancelled);
        }
        md.images.insert(uri, image);
    }
    md.links = links.into_iter().collect();
    Ok(Some(md))
}
