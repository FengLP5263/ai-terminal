//! Background-only syntax preparation. No filesystem lookup, code execution or formatting.
use super::patch::{PatchKind, PatchLine};
use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, StyleModifier, Theme, ThemeItem},
    parsing::SyntaxSet,
};

pub const TEXT: [u8; 3] = [225, 225, 225];
pub const KEY: [u8; 3] = [113, 211, 229];
pub const STRING: [u8; 3] = [206, 145, 189];
pub const NUMBER: [u8; 3] = [181, 206, 168];
pub const KEYWORD: [u8; 3] = [197, 134, 192];
const MAX_TOKENS: usize = 100_000;

#[derive(Debug, Clone)]
pub struct Token {
    pub end: u32,
    pub color: [u8; 3],
}
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub source: usize,
    pub indent: usize,
    pub tokens: Vec<Token>,
}
#[derive(Debug, Clone, Default)]
pub struct Presentation {
    pub rows: Vec<Row>,
    pub indent_width: usize,
}

fn theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let rgb = |c: [u8; 3]| Color {r:c[0],g:c[1],b:c[2],a:255};
        let mut theme = Theme::default();
        theme.settings.foreground = Some(rgb(TEXT));
        for (scope,color) in [
            ("comment", [128, 144, 128]),
            ("string", STRING),
            ("constant.numeric", NUMBER),
            ("constant.language, keyword, storage", KEYWORD),
            ("entity.name.function, support.function", [220,220,170]),
            ("entity.name.type, support.type, support.class", [78,201,176]),
            ("punctuation.section, punctuation.definition", [104,183,255]),
            ("entity.name.tag, entity.other.attribute-name, support.type.property-name", KEY),
            ("meta.mapping.key.json string, meta.mapping.key string, meta.structure.dictionary.key.json string", KEY),
            ("markup.heading, entity.name.section", [104,183,255]),
            ("markup.bold, markup.italic", [220,220,170]),
            ("markup.raw, markup.inline.raw, markup.fenced_code", STRING),
            ("markup.underline.link, markup.list punctuation", KEY),
        ] {
            theme.scopes.push(ThemeItem {scope:scope.parse().expect("fixed syntax scope"),style:StyleModifier {foreground:Some(rgb(color)),..Default::default()}});
        }
        theme
    })
}

pub fn is_file_line(line: &PatchLine) -> bool {
    matches!(
        line.kind,
        PatchKind::Context | PatchKind::Added | PatchKind::Deleted
    )
}

/// Expand visual tabs at tab stops; preserve all source spaces, without reformatting.
pub fn display_text(text: &str) -> String {
    let (mut output, mut column) = (String::with_capacity(text.len()), 0);
    for c in text.chars() {
        if c == '\t' {
            let spaces = 4 - column % 4;
            output.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            output.push(if c.is_control() { ' ' } else { c });
            column += if c.is_ascii() { 1 } else { 2 };
        }
    }
    output
}

/// Bounded presentation preparation for source lines, shared by native showcase fixtures.
pub fn prepare(path: &str, lines: &[PatchLine], cancelled: impl Fn() -> bool) -> Presentation {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    let began = Instant::now();
    let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_nonewlines);
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    let ext = name.rsplit_once('.').map_or(name.as_str(), |(_, ext)| ext);
    let ext = match ext {
        "jsonc" | "json5" => "json",
        "mjs" | "cjs" | "jsx" => "js",
        "tsx" | "ts" => "js",
        "mdx" => "md",
        other => other,
    };
    let syntax = syntaxes
        .find_syntax_by_extension(&name)
        .or_else(|| syntaxes.find_syntax_by_extension(ext))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let (mut old, mut new) = (
        HighlightLines::new(syntax, theme()),
        HighlightLines::new(syntax, theme()),
    );
    let mut presentation = Presentation::default();
    let (mut used, mut enabled, mut previous_old, mut previous_new) = (0, true, None, None);
    let mut indent_unit = 0;
    for (source, line) in lines.iter().enumerate().filter(|(_, l)| is_file_line(l)) {
        if cancelled() {
            break;
        }
        let indent = line.text.chars().take_while(|c| *c == ' ').count();
        if line.new.is_some() && (2..=8).contains(&indent) {
            indent_unit = if indent_unit == 0 {
                indent
            } else {
                gcd(indent_unit, indent)
            };
        }
        // Preserve separate old/new lexical states. Removed comments/quotes must
        // never affect the new version, including when jumping across a partial file.
        if line
            .old
            .is_some_and(|n| previous_old.is_some_and(|p| n != p + 1))
        {
            old = HighlightLines::new(syntax, theme());
        }
        if line
            .new
            .is_some_and(|n| previous_new.is_some_and(|p| n != p + 1))
        {
            new = HighlightLines::new(syntax, theme());
        }
        if began.elapsed() > Duration::from_secs(2) || used >= MAX_TOKENS {
            enabled = false;
        }
        let mut tokens = Vec::new();
        if enabled {
            let mut highlight = |h: &mut HighlightLines<'_>| {
                h.highlight_line(&line.text, syntaxes)
                    .ok()
                    .map(|parts| {
                        let (mut end, mut tokens) = (0, Vec::<Token>::new());
                        let limit = 256.min(MAX_TOKENS.saturating_sub(used));
                        for (style, text) in parts {
                            end += text.len();
                            let color =
                                [style.foreground.r, style.foreground.g, style.foreground.b];
                            if let Some(last) = tokens.last_mut().filter(|t| t.color == color) {
                                last.end = end as u32;
                            } else if tokens.len() < limit {
                                tokens.push(Token {
                                    end: end as u32,
                                    color,
                                });
                            } else {
                                tokens.clear();
                                break;
                            }
                        }
                        used += tokens.len();
                        tokens
                    })
                    .unwrap_or_default()
            };
            if line.old.is_some() {
                let old_tokens = highlight(&mut old);
                if line.new.is_none() {
                    tokens = old_tokens;
                }
            }
            if line.new.is_some() {
                tokens = highlight(&mut new);
            }
        }
        if let Some(n) = line.old {
            previous_old = Some(n);
        }
        if let Some(n) = line.new {
            previous_new = Some(n);
        }
        presentation.rows.push(Row {
            source,
            indent,
            tokens,
        });
    }
    presentation.indent_width = if indent_unit >= 2 {
        indent_unit
    } else if matches!(ext, "json" | "js" | "md" | "yaml" | "yml") {
        2
    } else {
        4
    };
    presentation
}
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(kind: PatchKind, old: Option<u64>, new: Option<u64>, text: &str) -> PatchLine {
        PatchLine {
            kind,
            old,
            new,
            text: text.into(),
        }
    }
    fn at(row: &Row, text: &str, needle: &str) -> [u8; 3] {
        let start = text.find(needle).unwrap();
        row.tokens
            .iter()
            .find(|t| t.end as usize > start)
            .map_or(TEXT, |t| t.color)
    }
    #[test]
    fn json_tokens_keep_the_same_colors_on_added_deleted_and_context_rows() {
        let text = "  \"name\": \"sample\", \"count\": 42, \"enabled\": true";
        let lines = vec![
            line(PatchKind::Context, Some(1), Some(1), "{"),
            line(PatchKind::Deleted, Some(2), None, text),
            line(PatchKind::Added, None, Some(2), text),
            line(PatchKind::Context, Some(3), Some(3), "}"),
        ];
        let p = prepare("config.json", &lines, || false);
        let old = &p.rows[1];
        let new = &p.rows[2];
        for token in ["name", "sample", "42", "true"] {
            assert_eq!(at(old, text, token), at(new, text, token));
        }
        assert_eq!(at(new, text, "name"), KEY);
        assert_eq!(at(new, text, "sample"), STRING);
        assert_eq!(at(new, text, "42"), NUMBER);
        assert_eq!(at(new, text, "true"), KEYWORD);
        assert_eq!(p.indent_width, 2);
    }
    #[test]
    fn markdown_code_and_unknown_files_use_language_rules_without_reformatting() {
        let lines = vec![
            line(PatchKind::Hunk, None, None, "@@ -1 +1 @@"),
            line(PatchKind::Note, None, None, "new file mode 100644"),
            line(PatchKind::Added, None, Some(1), "# Heading"),
            line(PatchKind::Added, None, Some(2), "  - **bold** and `code`"),
            line(PatchKind::Added, None, Some(3), "```json"),
            line(PatchKind::Added, None, Some(4), "{\"count\": 42}"),
            line(PatchKind::Added, None, Some(5), "```"),
        ];
        let p = prepare("readme.md", &lines, || false);
        assert_eq!(p.rows.len(), 5);
        assert_eq!(p.rows[0].source, 2);
        assert_ne!(at(&p.rows[0], &lines[2].text, "Heading"), TEXT);
        assert_eq!(display_text(" \tvalue\tend"), "    value   end");
        let plain = prepare("unknown.zzz", &lines[2..3], || false);
        assert_eq!(at(&plain.rows[0], &lines[2].text, "Heading"), TEXT);
        let code = vec![
            line(PatchKind::Deleted, Some(1), None, "/* removed open comment"),
            line(PatchKind::Added, None, Some(1), "const answer = 42;"),
        ];
        let p = prepare("file.js", &code, || false);
        assert_eq!(at(&p.rows[1], &code[1].text, "42"), NUMBER);
    }
}
