//! Shared file-type glyphs from the pinned Microsoft Codicons font.
use crate::chrome;
use egui::Color32;

pub fn icon(path: &str, binary: bool) -> (&'static str, Color32) {
    let path = path.replace('\\', "/");
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or(&path)
        .to_ascii_lowercase();
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    let blue = Color32::from_rgb(122, 180, 239);
    let amber = Color32::from_rgb(224, 188, 108);
    let purple = Color32::from_rgb(186, 154, 233);
    match ext {
        "md" | "mdx" | "markdown" => ("\u{eb1d}", blue),
        "json" | "jsonc" | "json5" => ("\u{eb0f}", amber),
        "ps1" | "psm1" | "psd1" | "sh" | "bash" | "cmd" | "bat" => ("\u{ea85}", chrome::COMPLETED),
        "png" | "jpg" | "jpeg" | "svg" | "gif" | "ico" | "bmp" | "webp" | "avif" | "mp3"
        | "mp4" | "wav" => ("\u{eaea}", purple),
        "pdf" => ("\u{eaeb}", chrome::FAILED),
        "zip" | "7z" | "gz" | "tar" | "rar" | "xz" | "bz2" => ("\u{eaef}", amber),
        "yaml" | "yml" | "toml" | "ini" | "env" | "config" | "conf" | "lock" => ("\u{eb52}", amber),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "java" | "kt" | "go" | "c"
        | "cpp" | "h" | "hpp" | "cs" | "rb" | "php" | "vue" | "svelte" | "html" | "css"
        | "scss" | "sql" | "xml" => ("\u{eae9}", blue),
        _ if matches!(
            name.as_str(),
            ".gitignore" | ".gitattributes" | ".editorconfig" | ".npmrc"
        ) || name.starts_with(".env.") =>
        {
            ("\u{eb52}", amber)
        }
        _ if binary => ("\u{eae8}", chrome::SECONDARY),
        _ => ("\u{ea7b}", chrome::SECONDARY),
    }
}
