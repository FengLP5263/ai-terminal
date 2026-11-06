use crate::font_choice::FontChoice;
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use egui_term::{FontSettings, TerminalFont};
use std::sync::{Arc, OnceLock};

const FACE_NAMES: [&str; 4] = [
    "terminal-regular",
    "terminal-bold",
    "terminal-italic",
    "terminal-bold-italic",
];
static CASCADIA: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/CascadiaMono-Regular.ttf"),
    include_bytes!("../assets/fonts/CascadiaMono-Bold.ttf"),
    include_bytes!("../assets/fonts/CascadiaMono-Italic.ttf"),
    include_bytes!("../assets/fonts/CascadiaMono-BoldItalic.ttf"),
];
static HACK: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Hack-Regular.ttf"),
    include_bytes!("../assets/fonts/Hack-Bold.ttf"),
    include_bytes!("../assets/fonts/Hack-Italic.ttf"),
    include_bytes!("../assets/fonts/Hack-BoldItalic.ttf"),
];
static JETBRAINS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf"),
];

fn system_font_directory() -> std::path::PathBuf {
    std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:/Windows"))
        .join("Fonts")
}

/// Inspect only known system faces, once. No disk scans or font copies into the package.
pub fn font_available(choice: FontChoice) -> bool {
    static AVAILABLE: OnceLock<[bool; 5]> = OnceLock::new();
    let available = AVAILABLE.get_or_init(|| {
        FontChoice::ALL.map(|font| {
            font.system_faces().is_none_or(|files| {
                cfg!(windows)
                    && files
                        .iter()
                        .all(|file| system_font_directory().join(file).is_file())
            })
        })
    });
    available[FontChoice::ALL
        .iter()
        .position(|font| *font == choice)
        .unwrap()]
}

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
    set_terminal_font(ctx, FontChoice::default());
}

/// Change the shared font definitions only on a preference change, never per frame.
pub fn set_terminal_font(ctx: &egui::Context, requested: FontChoice) -> FontChoice {
    let choice = if font_available(requested) {
        requested
    } else {
        FontChoice::default()
    };
    let mut fonts = FontDefinitions::default();
    let embedded = match choice {
        FontChoice::Hack => &HACK,
        FontChoice::JetBrainsMono => &JETBRAINS,
        _ => &CASCADIA,
    };
    for (index, name) in FACE_NAMES.into_iter().enumerate() {
        let mut face = match choice.system_faces() {
            Some(files) => match std::fs::read(system_font_directory().join(files[index])) {
                Ok(bytes) => FontData::from_owned(bytes),
                Err(_) => return set_terminal_font(ctx, FontChoice::default()),
            },
            None => FontData::from_static(embedded[index]),
        };
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
    for (name, path) in [("ui-latin", "segoeui.ttf"), ("ui-cjk", "msyh.ttc")] {
        if let Ok(bytes) = std::fs::read(system_font_directory().join(path)) {
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
    let system_fonts = system_font_directory();
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
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("terminal-font-choice"), choice));
    choice
}

pub fn terminal_font(size: f32) -> TerminalFont {
    terminal_font_for(FontChoice::default(), size)
}

/// UI emoji and the terminal must share a native family, keeping one bounded atlas.
pub fn current_terminal_font(ctx: &egui::Context, size: f32) -> TerminalFont {
    let choice = ctx
        .data(|data| data.get_temp::<FontChoice>(egui::Id::new("terminal-font-choice")))
        .unwrap_or_default();
    terminal_font_for(choice, size)
}

pub fn terminal_font_for(choice: FontChoice, size: f32) -> TerminalFont {
    let choice = if font_available(choice) {
        choice
    } else {
        FontChoice::default()
    };
    let faces: &'static [&'static [u8]] = match choice {
        FontChoice::CascadiaMono => &CASCADIA,
        FontChoice::Hack => &HACK,
        FontChoice::JetBrainsMono => &JETBRAINS,
        _ => &[],
    };
    TerminalFont::new(FontSettings {
        font_type: FontId::new(size, FontFamily::Name("terminal-regular".into())),
    })
    .with_variants(
        FontId::new(size, FontFamily::Name("terminal-bold".into())),
        FontId::new(size, FontFamily::Name("terminal-italic".into())),
        FontId::new(size, FontFamily::Name("terminal-bold-italic".into())),
    )
    .with_native_font(choice.family(), faces)
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
