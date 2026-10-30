use alacritty_terminal::vte::ansi::{self, NamedColor};
use egui::Color32;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub foreground: String,
    pub background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
    pub bright_foreground: Option<String>,
    pub dim_foreground: String,
    pub dim_black: String,
    pub dim_red: String,
    pub dim_green: String,
    pub dim_yellow: String,
    pub dim_blue: String,
    pub dim_magenta: String,
    pub dim_cyan: String,
    pub dim_white: String,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            foreground: String::from("#d7d7d7"),
            background: String::from("#000000"),
            black: String::from("#000000"),
            red: String::from("#f7768e"),
            green: String::from("#9ece6a"),
            yellow: String::from("#e0af68"),
            blue: String::from("#7aa2f7"),
            magenta: String::from("#bb9af7"),
            cyan: String::from("#7dcfff"),
            white: String::from("#d7d7d7"),
            bright_black: String::from("#808080"),
            bright_red: String::from("#ff9aad"),
            bright_green: String::from("#b6e880"),
            bright_yellow: String::from("#f2c879"),
            bright_blue: String::from("#9bbcff"),
            bright_magenta: String::from("#d3b4ff"),
            bright_cyan: String::from("#a4e7ff"),
            bright_white: String::from("#f0f0f0"),
            bright_foreground: None,
            dim_foreground: String::from("#909090"),
            dim_black: String::from("#0f0f0f"),
            dim_red: String::from("#712b2b"),
            dim_green: String::from("#5f6f3a"),
            dim_yellow: String::from("#a17e4d"),
            dim_blue: String::from("#456877"),
            dim_magenta: String::from("#704d68"),
            dim_cyan: String::from("#4d7770"),
            dim_white: String::from("#8e8e8e"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalTheme {
    colors: Arc<[Color32; 269]>,
}
impl Default for TerminalTheme {
    fn default() -> Self {
        static DEFAULT: OnceLock<TerminalTheme> = OnceLock::new();
        DEFAULT.get_or_init(|| Self::new(Box::default())).clone()
    }
}
impl TerminalTheme {
    pub fn new(palette: Box<ColorPalette>) -> Self {
        let parse = |value: &str| hex_to_color(value).expect("valid terminal palette");
        let mut colors = [parse(&palette.background); 269];
        for (name, value) in [
            (NamedColor::Black, &palette.black),
            (NamedColor::Red, &palette.red),
            (NamedColor::Green, &palette.green),
            (NamedColor::Yellow, &palette.yellow),
            (NamedColor::Blue, &palette.blue),
            (NamedColor::Magenta, &palette.magenta),
            (NamedColor::Cyan, &palette.cyan),
            (NamedColor::White, &palette.white),
            (NamedColor::BrightBlack, &palette.bright_black),
            (NamedColor::BrightRed, &palette.bright_red),
            (NamedColor::BrightGreen, &palette.bright_green),
            (NamedColor::BrightYellow, &palette.bright_yellow),
            (NamedColor::BrightBlue, &palette.bright_blue),
            (NamedColor::BrightMagenta, &palette.bright_magenta),
            (NamedColor::BrightCyan, &palette.bright_cyan),
            (NamedColor::BrightWhite, &palette.bright_white),
            (NamedColor::Foreground, &palette.foreground),
            (NamedColor::Background, &palette.background),
            (NamedColor::DimBlack, &palette.dim_black),
            (NamedColor::DimRed, &palette.dim_red),
            (NamedColor::DimGreen, &palette.dim_green),
            (NamedColor::DimYellow, &palette.dim_yellow),
            (NamedColor::DimBlue, &palette.dim_blue),
            (NamedColor::DimMagenta, &palette.dim_magenta),
            (NamedColor::DimCyan, &palette.dim_cyan),
            (NamedColor::DimWhite, &palette.dim_white),
            (NamedColor::DimForeground, &palette.dim_foreground),
        ] {
            colors[name as usize] = parse(value);
        }
        colors[NamedColor::BrightForeground as usize] = parse(
            palette
                .bright_foreground
                .as_deref()
                .unwrap_or(&palette.foreground),
        );
        for r in 0..6 {
            for g in 0..6 {
                for b in 0..6 {
                    colors[16 + r * 36 + g * 6 + b] = Color32::from_rgb(
                        if r == 0 { 0 } else { (r * 40 + 55) as u8 },
                        if g == 0 { 0 } else { (g * 40 + 55) as u8 },
                        if b == 0 { 0 } else { (b * 40 + 55) as u8 },
                    );
                }
            }
        }
        for i in 0..24 {
            let value = (i * 10 + 8) as u8;
            colors[232 + i] = Color32::from_rgb(value, value, value);
        }
        Self {
            colors: Arc::new(colors),
        }
    }
    pub fn get_color(&self, color: ansi::Color) -> Color32 {
        match color {
            ansi::Color::Spec(rgb) => Color32::from_rgb(rgb.r, rgb.g, rgb.b),
            ansi::Color::Indexed(index) => self.colors[index as usize],
            ansi::Color::Named(name) => self
                .colors
                .get(name as usize)
                .copied()
                .unwrap_or(self.colors[NamedColor::Background as usize]),
        }
    }
}
fn hex_to_color(hex: &str) -> anyhow::Result<Color32> {
    if hex.len() != 7 || !hex.is_ascii() || !hex.starts_with('#') {
        anyhow::bail!("invalid terminal color");
    }
    Ok(Color32::from_rgb(
        u8::from_str_radix(&hex[1..3], 16)?,
        u8::from_str_radix(&hex[3..5], 16)?,
        u8::from_str_radix(&hex[5..7], 16)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_palette_preserves_named_indexed_truecolor_and_shared_storage() {
        let first = TerminalTheme::default();
        let second = TerminalTheme::default();
        assert!(Arc::ptr_eq(&first.colors, &second.colors));
        assert_eq!(
            first.get_color(ansi::Color::Named(NamedColor::Red)),
            first.get_color(ansi::Color::Indexed(1))
        );
        assert_eq!(
            first.get_color(ansi::Color::Indexed(196)),
            Color32::from_rgb(255, 0, 0)
        );
        assert_eq!(
            first.get_color(ansi::Color::Spec(ansi::Rgb { r: 4, g: 8, b: 16 })),
            Color32::from_rgb(4, 8, 16)
        );
    }
}
