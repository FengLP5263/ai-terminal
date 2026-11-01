use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use egui_term::{FontSettings, TerminalFont};
use std::sync::{Arc, OnceLock};

pub const TERMINAL_BG: Color32 = Color32::BLACK;
pub const SIDEBAR_BG: Color32 = Color32::from_gray(18);

/// Fonts are embedded so the terminal's Latin metrics do not depend on the PC.
pub fn setup(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = SIDEBAR_BG;
    visuals.window_fill = Color32::from_gray(24);
    visuals.override_text_color = Some(Color32::from_gray(220));
    visuals.selection.bg_fill = Color32::from_gray(64);
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_gray(44);
    ctx.set_visuals(visuals);
    ctx.set_theme(egui::Theme::Dark);
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.animation_time = 0.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    ctx.set_style_of(egui::Theme::Dark, style);
    let mut fonts = FontDefinitions::default();
    let faces: [(&str, &[u8]); 4] = [
        (
            "terminal-regular",
            include_bytes!("../assets/fonts/Hack-Regular.ttf"),
        ),
        (
            "terminal-bold",
            include_bytes!("../assets/fonts/Hack-Bold.ttf"),
        ),
        (
            "terminal-italic",
            include_bytes!("../assets/fonts/Hack-Italic.ttf"),
        ),
        (
            "terminal-bold-italic",
            include_bytes!("../assets/fonts/Hack-BoldItalic.ttf"),
        ),
    ];
    for (name, bytes) in faces {
        let mut face = FontData::from_static(bytes);
        face.tweak.hinting = Some(true);
        face.tweak.subpixel_binning = Some(false);
        face.tweak.hinting_target =
            egui::epaint::text::HintingTarget::Smooth(egui::epaint::text::SmoothHinting {
                light: true,
                symmetric_rendering: false,
                preserve_linear_metrics: true,
            });
        fonts.font_data.insert(name.into(), Arc::new(face));
    }
    for (name, path) in [
        ("ui-latin", "C:/Windows/Fonts/segoeui.ttf"),
        ("ui-cjk", "C:/Windows/Fonts/msyh.ttc"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.into(), Arc::new(FontData::from_owned(bytes)));
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, name.into());
        }
    }
    if fonts.font_data.contains_key("ui-latin") {
        let ui_family = fonts.families.get_mut(&FontFamily::Proportional).unwrap();
        ui_family.retain(|name| name != "ui-latin");
        ui_family.insert(0, "ui-latin".into());
    }
    let mut fallback = fonts.families[&FontFamily::Monospace].clone();
    if fonts.font_data.contains_key("ui-cjk") {
        fallback.insert(0, "ui-cjk".into());
    }
    for name in [
        "terminal-regular",
        "terminal-bold",
        "terminal-italic",
        "terminal-bold-italic",
    ] {
        let mut family = vec![name.into()];
        family.extend(fallback.clone());
        fonts.families.insert(FontFamily::Name(name.into()), family);
    }
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "terminal-regular".into());
    if fonts.font_data.contains_key("ui-cjk") {
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("ui-cjk".into());
    }
    // Use the system emoji face for complete shaped clusters in UI labels/editors.
    let system_fonts = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:/Windows"))
        .join("Fonts");
    if let Ok(bytes) = std::fs::read(system_fonts.join("seguiemj.ttf")) {
        fonts
            .font_data
            .insert("ui-emoji".into(), Arc::new(FontData::from_owned(bytes)));
        let mut family = vec!["ui-emoji".into()];
        family.extend(fonts.families[&FontFamily::Proportional].clone());
        fonts
            .families
            .insert(FontFamily::Name("ui-emoji".into()), family);
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(2, "ui-emoji".into());
    }
    ctx.set_fonts(fonts);
}

pub fn terminal_font(size: f32) -> TerminalFont {
    static FACES: [&[u8]; 4] = [
        include_bytes!("../assets/fonts/Hack-Regular.ttf"),
        include_bytes!("../assets/fonts/Hack-Bold.ttf"),
        include_bytes!("../assets/fonts/Hack-Italic.ttf"),
        include_bytes!("../assets/fonts/Hack-BoldItalic.ttf"),
    ];
    TerminalFont::new(FontSettings {
        font_type: FontId::new(size, FontFamily::Name("terminal-regular".into())),
    })
    .with_variants(
        FontId::new(size, FontFamily::Name("terminal-bold".into())),
        FontId::new(size, FontFamily::Name("terminal-italic".into())),
        FontId::new(size, FontFamily::Name("terminal-bold-italic".into())),
    )
    .with_native_font("Hack", &FACES)
}

/// The approved B artwork is embedded; startup performs no asset-file I/O.
pub fn app_icon() -> egui::IconData {
    let mut reader = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "../assets/branding/ai-terminal.png"
    )))
    .read_info()
    .expect("embedded app icon header");
    let mut rgba = vec![0; reader.output_buffer_size().expect("bounded app icon")];
    let info = reader
        .next_frame(&mut rgba)
        .expect("embedded app icon pixels");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    rgba.truncate(info.buffer_size());
    egui::IconData {
        rgba,
        width: info.width,
        height: info.height,
    }
}

/// Reuse the approved icon for application, floating, and developer windows.
pub fn window_icon() -> Arc<egui::IconData> {
    static ICON: OnceLock<Arc<egui::IconData>> = OnceLock::new();
    ICON.get_or_init(|| Arc::new(app_icon())).clone()
}
