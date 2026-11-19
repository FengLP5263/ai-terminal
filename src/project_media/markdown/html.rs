//! A small allowlist for document HTML, not a browser or CSS interpreter.
use super::*;
use std::ops::Range;

pub(super) struct ImageSpec {
    pub source: String,
    width: Option<f32>,
    height: Option<f32>,
}
impl ImageSpec {
    pub fn size(&self, original: [u32; 2]) -> egui::Vec2 {
        let aspect = original[0] as f32 / original[1].max(1) as f32;
        let width = self
            .width
            .unwrap_or_else(|| self.height.map_or(original[0] as f32, |h| h * aspect));
        let height = self.height.unwrap_or(width / aspect);
        egui::vec2(width, height)
    }
}
struct Token {
    span: Range<usize>,
    name: String,
    attrs: BTreeMap<String, String>,
    closing: bool,
}
pub(super) struct Normalizer<'a> {
    marker: &'a str,
    pub images: BTreeMap<String, ImageSpec>,
    pub centered: Vec<String>,
}
impl<'a> Normalizer<'a> {
    pub fn new(marker: &'a str) -> Self {
        Self {
            marker,
            images: BTreeMap::new(),
            centered: vec![],
        }
    }
    pub fn normalize(
        &mut self,
        source: &str,
        depth: usize,
        cancelled: &impl Fn() -> bool,
    ) -> Result<String, FileError> {
        let mut tokens = vec![];
        for (event, span) in Parser::new_ext(source, options()).into_offset_iter() {
            if cancelled() {
                return Err(FileError::Cancelled);
            }
            if matches!(event, Event::Html(_) | Event::InlineHtml(_)) {
                scan(source, span, &mut tokens);
            }
        }
        let mut output = String::new();
        let mut cursor = 0;
        let mut hrefs = vec![];
        for (index, token) in tokens.iter().enumerate() {
            if cancelled() {
                return Err(FileError::Cancelled);
            }
            if token.span.start < cursor {
                continue;
            }
            output.push_str(&source[cursor..token.span.start]);
            cursor = token.span.end;
            if token.name.starts_with(self.marker) {
                output.push_str(&source[token.span.clone()]);
                continue;
            }
            if depth < 16
                && token.name == "div"
                && !token.closing
                && token
                    .attrs
                    .get("align")
                    .is_some_and(|a| a.eq_ignore_ascii_case("center"))
            {
                let mut nested = 1;
                if let Some(end) = tokens[index + 1..].iter().find(|next| {
                    if next.name == "div" {
                        nested += if next.closing { -1 } else { 1 };
                    }
                    nested == 0
                }) {
                    let body = self.normalize(
                        &source[token.span.end..end.span.start],
                        depth + 1,
                        cancelled,
                    )?;
                    output.push_str(&format!(
                        "\n<{}-center-{} />\n",
                        self.marker,
                        self.centered.len()
                    ));
                    self.centered.push(body);
                    cursor = end.span.end;
                    continue;
                }
            }
            match token.name.as_str() {
                "script" | "style" | "iframe" | "object" | "template" if !token.closing => {
                    if let Some(end) = tokens[index + 1..]
                        .iter()
                        .find(|t| t.closing && t.name == token.name)
                    {
                        cursor = end.span.end;
                    } else {
                        cursor = source.len();
                    }
                }
                "img" if !token.closing && self.images.len() < 128 => {
                    if let Some(src) = token.attrs.get("src").filter(|s| !s.is_empty()) {
                        let alias = format!("{}-image-{}", self.marker, self.images.len());
                        let dim = |key: &str| {
                            token
                                .attrs
                                .get(key)
                                .and_then(|s| s.trim_end_matches("px").parse::<u32>().ok())
                                .filter(|n| (1..=4096).contains(n))
                                .map(|n| n as f32)
                        };
                        self.images.insert(
                            alias.clone(),
                            ImageSpec {
                                source: src.clone(),
                                width: dim("width"),
                                height: dim("height"),
                            },
                        );
                        let alt = token
                            .attrs
                            .get("alt")
                            .map(String::as_str)
                            .unwrap_or("image");
                        output.push_str(&format!("![{}](<{alias}>)", escape(alt)));
                    }
                }
                "a" if !token.closing => {
                    let href = token.attrs.get("href").cloned();
                    if href.is_some() {
                        output.push('[');
                    }
                    hrefs.push(href);
                }
                "a" => {
                    if let Some(Some(href)) = hrefs.pop() {
                        output.push_str(&format!("](<{}>)", destination(&href)));
                    }
                }
                "br" => output.push_str("  \n"),
                "hr" => output.push_str("\n\n---\n\n"),
                "p" | "div" | "section" | "article" | "details" | "summary" => {
                    output.push_str("\n\n")
                }
                "b" | "strong" => output.push_str("**"),
                "em" | "i" => output.push('*'),
                "s" | "del" => output.push_str("~~"),
                "code" | "kbd" => output.push('`'),
                _ => {} // Decorative/unsupported tags never execute or load a resource.
            }
        }
        output.push_str(&source[cursor..]);
        Ok(output)
    }
}
fn escape(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        if "\\`*_{}[]<>|".contains(ch) {
            result.push('\\');
        }
        if ch.is_control() {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }
    result
}
fn destination(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '<' => "%3C".into(),
            '>' => "%3E".into(),
            '\n' | '\r' => "%0A".into(),
            '\\' => "%5C".into(),
            ' ' => "%20".into(),
            _ => c.to_string(),
        })
        .collect()
}
fn scan(source: &str, span: Range<usize>, output: &mut Vec<Token>) {
    let bytes = source.as_bytes();
    let mut cursor = span.start;
    while cursor < span.end {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }
        let start = cursor;
        if source[start..].starts_with("<!--") {
            let end = source[start + 4..span.end]
                .find("-->")
                .map_or(span.end, |n| start + 4 + n + 3);
            output.push(Token {
                span: start..end,
                name: "!comment".into(),
                attrs: BTreeMap::new(),
                closing: false,
            });
            cursor = end;
            continue;
        }
        cursor += 1;
        let mut quote = None;
        while cursor < span.end {
            let ch = bytes[cursor];
            if let Some(q) = quote {
                if ch == q {
                    quote = None;
                }
            } else if matches!(ch, b'\'' | b'"') {
                quote = Some(ch);
            } else if ch == b'>' {
                break;
            }
            cursor += 1;
        }
        if cursor == span.end {
            break;
        }
        cursor += 1;
        let raw = &source[start + 1..cursor - 1];
        let closing = raw.starts_with('/');
        let raw = raw.trim_start_matches('/').trim();
        let name_end = raw
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .unwrap_or(raw.len());
        if name_end == 0 {
            continue;
        }
        output.push(Token {
            span: start..cursor,
            name: raw[..name_end].to_ascii_lowercase(),
            attrs: attributes(&raw[name_end..]),
            closing,
        });
    }
}
fn attributes(mut raw: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    while !raw.is_empty() {
        raw = raw.trim_start();
        let end = raw
            .find(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '-' | '_' | ':'))
            .unwrap_or(raw.len());
        if end == 0 {
            break;
        }
        let name = raw[..end].to_ascii_lowercase();
        raw = raw[end..].trim_start();
        if !raw.starts_with('=') {
            continue;
        }
        raw = raw[1..].trim_start();
        let value;
        if let Some(quote) = raw.chars().next().filter(|c| matches!(c, '\'' | '"')) {
            raw = &raw[1..];
            let end = raw.find(quote).unwrap_or(raw.len());
            value = &raw[..end];
            raw = raw.get(end + 1..).unwrap_or("");
        } else {
            let end = raw.find(char::is_whitespace).unwrap_or(raw.len());
            value = &raw[..end];
            raw = &raw[end..];
        }
        result.entry(name).or_insert_with(|| entities(value));
    }
    result
}
fn entities(value: &str) -> String {
    let mut result = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|&n| n <= 12) else {
            result.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|n| u32::from_str_radix(n, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|n| n.parse::<u32>().ok()))
                .and_then(char::from_u32),
        };
        if let Some(ch) = decoded {
            result.push(ch);
            rest = &rest[end + 1..];
        } else {
            result.push('&');
            rest = &rest[1..];
        }
    }
    result.push_str(rest);
    result
}
