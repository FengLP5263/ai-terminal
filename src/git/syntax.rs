//! Background-only syntax preparation. No filesystem lookup, code execution or formatting.
use super::patch::{PatchKind, PatchLine};
use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, StyleModifier, Theme, ThemeItem},
    parsing::{SyntaxReference, SyntaxSet},
};

pub const TEXT: [u8; 3] = [225, 225, 225];
pub const KEY: [u8; 3] = [113, 211, 229];
pub const STRING: [u8; 3] = [206, 145, 189];
pub const NUMBER: [u8; 3] = [181, 206, 168];
pub const KEYWORD: [u8; 3] = [197, 134, 192];
pub const VARIABLE: [u8; 3] = [156, 220, 254];
pub const COMMENT: [u8; 3] = [128, 144, 128];
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
            ("comment", COMMENT),
            ("string", STRING),
            ("constant.numeric", NUMBER),
            ("constant.language, keyword, storage", KEYWORD),
            ("variable", VARIABLE),
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
    let syntaxes = SYNTAXES.get_or_init(two_face::syntax::extra_no_newlines);
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    let ext = name.rsplit_once('.').map_or(name.as_str(), |(_, ext)| ext);
    let syntax = syntax_for_path(syntaxes, &name, ext, lines);
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
fn syntax_for_path<'a>(
    syntaxes: &'a SyntaxSet,
    name: &str,
    ext: &str,
    lines: &[PatchLine],
) -> &'a SyntaxReference {
    let alias = if name.starts_with(".env.") {
        "env"
    } else if name.starts_with("dockerfile.") {
        "Dockerfile"
    } else {
        match name {
            ".bashrc" | ".bash_profile" | ".profile" | ".zshrc" | ".zprofile" | ".envrc" => "sh",
            _ => match ext {
                "psm1" | "psd1" => "ps1",
                "cmd" => "bat",
                "mjs" | "cjs" | "jsx" => "js",
                "jsonc" | "json5" => "json",
                "mdx" => "md",
                _ => ext,
            },
        }
    };
    syntaxes
        .find_syntax_by_extension(name)
        .or_else(|| syntaxes.find_syntax_by_extension(ext))
        .or_else(|| syntaxes.find_syntax_by_extension(alias))
        .or_else(|| {
            // Shebang recognition is in memory only. Never open another file,
            // consult project syntax settings or execute the interpreter.
            lines
                .iter()
                .find(|l| is_file_line(l) && l.new == Some(1))
                .or_else(|| lines.iter().find(|l| is_file_line(l) && l.old == Some(1)))
                .and_then(|l| {
                    syntaxes.find_syntax_by_first_line(l.text.trim_start_matches('\u{feff}'))
                })
        })
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
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

    #[test]
    fn common_scripts_and_configuration_files_select_real_language_grammars() {
        let set = two_face::syntax::extra_no_newlines();
        for (path, extension) in [
            ("tasks.PS1", "ps1"),
            ("module.psm1", "psm1"),
            ("module.psd1", "psd1"),
            ("setup.cmd", "cmd"),
            ("setup.bat", "bat"),
            ("build.sh", "sh"),
            (".bashrc", "bashrc"),
            (".zshrc", "zshrc"),
            ("task.py", "py"),
            ("app.js", "js"),
            ("app.mjs", "mjs"),
            ("app.cjs", "cjs"),
            ("types.ts", "ts"),
            ("App.tsx", "tsx"),
            ("App.jsx", "jsx"),
            ("Dockerfile", "dockerfile"),
            ("Dockerfile.dev", "dev"),
            ("Cargo.toml", "toml"),
            ("settings.ini", "ini"),
            (".env", "env"),
            (".env.production", "production"),
            ("build.yml", "yml"),
            ("settings.jsonc", "jsonc"),
            ("Makefile", "makefile"),
            ("CMakeLists.txt", "txt"),
        ] {
            let syntax = syntax_for_path(&set, &path.to_ascii_lowercase(), extension, &[]);
            assert_ne!(syntax.name, "Plain Text", "No grammar for {path}");
            if matches!(extension, "ts" | "tsx") {
                assert!(
                    syntax.name.contains("TypeScript"),
                    "{path}: {}",
                    syntax.name
                );
            }
        }
    }

    #[test]
    fn powershell_distinguishes_keywords_variables_types_commands_strings_and_comments() {
        let texts = [
            "# 注释只作为文字显示，不运行脚本",
            "param([switch]$List, [string]$Item)",
            "if ($List -and $env:USERPROFILE) {",
            "    Write-Host \"Hello $Item\"; $count = 42 # trailing comment",
            "    $literal = 'literal $Item'; $enabled = $true",
            "}",
        ];
        for path in ["test.ps1", "module.psm1", "module.psd1", "TEST.PS1"] {
            let lines: Vec<_> = texts
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    line(
                        PatchKind::Context,
                        Some(i as u64 + 1),
                        Some(i as u64 + 1),
                        t,
                    )
                })
                .collect();
            let p = prepare(path, &lines, || false);
            for (i, token, expected) in [
                (0, "注释", COMMENT),
                (1, "param", KEYWORD),
                (1, "List", VARIABLE),
                (2, "if", KEYWORD),
                (2, "USERPROFILE", VARIABLE),
                (3, "Write-Host", [220, 220, 170]),
                (3, "Hello", STRING),
                (3, "Item", VARIABLE),
                (3, "42", NUMBER),
                (3, "trailing", COMMENT),
                (4, "literal $Item", STRING),
            ] {
                assert_eq!(at(&p.rows[i], texts[i], token), expected, "{path}: {token}");
            }
        }
    }

    #[test]
    fn powershell_multiline_states_and_diff_versions_stay_independent() {
        let texts = [
            "<# block comment",
            "comment $var",
            "#>",
            "$text = @\"",
            "hello $name",
            "\"@",
            "if ($true) { $answer = 42 }",
        ];
        let mut lines: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                line(
                    PatchKind::Context,
                    Some(i as u64 + 1),
                    Some(i as u64 + 1),
                    t,
                )
            })
            .collect();
        lines.push(line(
            PatchKind::Deleted,
            Some(8),
            None,
            "<# deleted unclosed comment",
        ));
        lines.push(line(PatchKind::Added, None, Some(8), "$next = 7 # comment"));
        let p = prepare("test.ps1", &lines, || false);
        assert_eq!(at(&p.rows[1], texts[1], "comment"), COMMENT);
        assert_eq!(at(&p.rows[4], texts[4], "hello"), STRING);
        assert_eq!(at(&p.rows[4], texts[4], "name"), VARIABLE);
        assert_eq!(at(&p.rows[6], texts[6], "if"), KEYWORD);
        assert_eq!(at(&p.rows[8], &lines[8].text, "7"), NUMBER);
        assert_eq!(at(&p.rows[8], &lines[8].text, "comment"), COMMENT);
        assert_eq!(p.rows[8].indent, 0);
    }

    #[test]
    fn other_scripts_shebangs_and_unknown_files_keep_source_and_cancel_promptly() {
        for (path, text, word) in [
            ("setup.cmd", "if defined NAME echo hello", "if"),
            ("setup.bat", "rem comment", "comment"),
            ("build.sh", "if [ -n \"$HOME\" ]; then", "if"),
            ("task.py", "def hello(name): # comment", "def"),
            ("app.ts", "interface User { name: string }", "interface"),
            (
                "App.tsx",
                "export const App = () => <div>Hello</div>;",
                "export",
            ),
            ("Dockerfile.dev", "FROM alpine:3.20", "FROM"),
            ("Cargo.toml", "name = \"demo\"", "demo"),
        ] {
            let lines = vec![line(PatchKind::Added, None, Some(1), text)];
            let p = prepare(path, &lines, || false);
            assert_ne!(at(&p.rows[0], text, word), TEXT, "{path}: {word}");
            assert_eq!(lines[0].text, text);
        }
        let lines = vec![
            line(
                PatchKind::Context,
                Some(1),
                Some(1),
                "#!/usr/bin/env python3",
            ),
            line(PatchKind::Context, Some(2), Some(2), "def hello():"),
        ];
        let p = prepare("scripts/runner", &lines, || false);
        assert_ne!(at(&p.rows[1], &lines[1].text, "def"), TEXT);
        let p = prepare("file.unknown-type", &lines[1..], || false);
        assert_eq!(at(&p.rows[0], &lines[1].text, "def"), TEXT);
        assert!(prepare("file.ps1", &lines, || true).rows.is_empty());
    }
}
