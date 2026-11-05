//! App chrome language. Terminal output and user-authored names are never translated.
use serde::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    ZhCn,
    En,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    #[default]
    ZhCn,
    En,
}
impl Locale {
    pub fn from_tag(tag: &str) -> Self {
        if tag
            .split(['-', '_', '.'])
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case("zh"))
        {
            Self::ZhCn
        } else {
            Self::En
        }
    }
    pub fn text(self, zh: &'static str, en: &'static str) -> &'static str {
        match self {
            Self::ZhCn => zh,
            Self::En => en,
        }
    }
}
impl Language {
    pub fn resolve(self) -> Locale {
        match self {
            Self::System => system_locale(),
            Self::ZhCn => Locale::ZhCn,
            Self::En => Locale::En,
        }
    }
    pub fn label(self, locale: Locale) -> &'static str {
        match self {
            Self::System => locale.text("跟随系统", "System default"),
            Self::ZhCn => "简体中文",
            Self::En => "English",
        }
    }
}
pub fn system_locale() -> Locale {
    static SYSTEM: std::sync::OnceLock<Locale> = std::sync::OnceLock::new();
    *SYSTEM.get_or_init(|| {
        #[cfg(windows)]
        {
            let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
            if id & 0x3ff == 0x04 {
                Locale::ZhCn
            } else {
                Locale::En
            }
        }
        #[cfg(not(windows))]
        {
            Locale::from_tag(
                &std::env::var("LC_ALL")
                    .or_else(|_| std::env::var("LANG"))
                    .unwrap_or_else(|_| "en".into()),
            )
        }
    })
}
pub fn apply(ctx: &egui::Context, language: Language) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("app-locale"), language.resolve()));
}
pub fn locale(ctx: &egui::Context) -> Locale {
    ctx.data(|data| {
        data.get_temp(egui::Id::new("app-locale"))
            .unwrap_or_default()
    })
}
pub fn tr(ctx: &egui::Context, zh: &'static str, en: &'static str) -> &'static str {
    locale(ctx).text(zh, en)
}
pub fn activity(locale: Locale, state: crate::status::Activity) -> &'static str {
    use crate::status::Activity::*;
    match state {
        Unknown => locale.text("等待任务通知", "Awaiting task events"),
        Idle => locale.text("就绪", "Ready"),
        Working => locale.text("进行中", "Working"),
        Unconfirmed => locale.text("状态待确认", "Status unconfirmed"),
        Completed => locale.text("已完成", "Completed"),
        Waiting => locale.text("需要处理", "Needs attention"),
        Failed => locale.text("失败", "Failed"),
        Exited => locale.text("已退出", "Exited"),
    }
}

pub fn color(locale: Locale, color: crate::model::SessionColor) -> &'static str {
    use crate::model::SessionColor::*;
    match color {
        Blue => locale.text("蓝色", "Blue"),
        Green => locale.text("绿色", "Green"),
        Teal => locale.text("青色", "Teal"),
        Amber => locale.text("黄色", "Yellow"),
        Orange => locale.text("橙色", "Orange"),
        Pink => locale.text("粉色", "Pink"),
        Purple => locale.text("紫色", "Purple"),
    }
}
