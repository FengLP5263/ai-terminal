//! Shared compact file rows for working-tree changes and expanded commits.
use super::{chrome, display_text, header_font, i18n};
use egui::{Color32, FontFamily, FontId, RichText, Sense};

pub(super) struct FileRow<'a> {
    pub path: &'a str,
    pub status: &'a str,
    pub added: u64,
    pub deleted: u64,
    pub binary: bool,
    pub omitted: bool,
}

fn icon(path: &str, binary: bool) -> (&'static str, Color32) {
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

pub(super) fn show(ui: &mut egui::Ui, file: FileRow<'_>) {
    let font = header_font(ui);
    let height = ui.text_style_height(&egui::TextStyle::Body).max(18.0) + 4.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let (glyph, color) = icon(file.path, file.binary);
    let icon_font = FontId::new(font.size, FontFamily::Name("codicons".into()));
    let galley = ui.painter().layout_no_wrap(glyph.into(), icon_font, color);
    ui.painter().galley(
        egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5),
        galley,
        color,
    );
    let status = file.status;
    let status_color = if status.contains('D') {
        chrome::FAILED
    } else if status.contains(['A', '?']) {
        chrome::COMPLETED
    } else {
        chrome::WAITING
    };
    let status_galley = ui
        .painter()
        .layout_no_wrap(status.into(), font.clone(), status_color);
    let left = rect.left() + font.size + 8.0;
    let width = (rect.right() - left - status_galley.size().x - 10.0).max(1.0);
    let path = egui::WidgetText::from(RichText::new(display_text(file.path, 1000))).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width,
        font,
    );
    let status_x = (left + path.size().x + 10.0).min(rect.right() - status_galley.size().x);
    ui.painter().galley(
        egui::pos2(left, rect.center().y - path.size().y * 0.5),
        path,
        chrome::TEXT,
    );
    ui.painter().galley(
        egui::pos2(status_x, rect.center().y - status_galley.size().y * 0.5),
        status_galley,
        status_color,
    );
    response.on_hover_ui(|ui| {
        ui.style_mut().override_font_id = Some(header_font(ui));
        ui.set_max_width(420.0);
        ui.label(display_text(file.path, 4000));
        let locale = i18n::locale(ui.ctx());
        ui.weak(format!(
            "{}: {}",
            locale.text("状态", "Status"),
            file.status
        ));
        if file.binary {
            ui.weak(locale.text("二进制", "Binary"));
        } else if file.omitted {
            ui.weak(locale.text("未统计行数", "Line counts unavailable"));
        } else {
            ui.horizontal(|ui| {
                ui.colored_label(chrome::COMPLETED, format!("+{}", file.added));
                ui.colored_label(chrome::FAILED, format!("−{}", file.deleted));
            });
        }
    });
}
