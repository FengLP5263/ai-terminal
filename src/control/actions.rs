//! Typed native controls shared by the console parser and the GUI dispatcher.
use super::{ApiError, ApiResult};
use crate::{font_choice::FontChoice, i18n::Language, model::UiSettings};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SettingsPatch {
    pub language: Option<String>,
    pub terminal_font: Option<String>,
    pub ui_scale_percent: Option<f32>,
    pub sidebar_collapsed: Option<bool>,
    pub sidebar_width: Option<f32>,
    pub floating_enabled: Option<bool>,
    pub floating_border: Option<bool>,
    pub floating_scale_percent: Option<f32>,
    pub floating_transparency_percent: Option<f32>,
    pub floating_position_pixels: Option<[f32; 2]>,
    pub floating_center: Option<bool>,
}
pub fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new("invalid_argument", message)
}
pub fn range(value: f32, min: f32, max: f32, field: &str) -> ApiResult<f32> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(invalid(format!("{field} must be between {min} and {max}")));
    }
    Ok(value)
}
impl SettingsPatch {
    pub fn apply(&self, current: UiSettings) -> ApiResult<UiSettings> {
        let mut p = current;
        if let Some(v) = &self.language {
            p.language = match v.as_str() {
                "system" => Language::System,
                "zh_cn" | "zh-CN" | "zh" => Language::ZhCn,
                "en" => Language::En,
                _ => return Err(invalid("language must be system, zh_cn or en")),
            };
        }
        if let Some(v) = &self.terminal_font {
            p.terminal_font = match v.as_str() {
                "jetbrains_mono" => FontChoice::JetBrainsMono,
                "cascadia_mono" => FontChoice::CascadiaMono,
                "hack" => FontChoice::Hack,
                "consolas" => FontChoice::Consolas,
                "courier_new" => FontChoice::CourierNew,
                _ => return Err(invalid("Unknown terminal font ID")),
            };
        }
        if let Some(v) = self.ui_scale_percent {
            p.ui_scale = range(v, 50., 250., "ui_scale_percent")? / 100.;
        }
        if let Some(v) = self.sidebar_collapsed {
            p.sidebar_collapsed = v;
        }
        if let Some(v) = self.sidebar_width {
            p.sidebar_width = range(
                v,
                UiSettings::MIN_SIDEBAR_WIDTH,
                UiSettings::MAX_SIDEBAR_WIDTH,
                "sidebar_width",
            )?;
        }
        if let Some(v) = self.floating_enabled {
            p.floating_enabled = v;
        }
        if let Some(v) = self.floating_border {
            p.floating_border = v;
        }
        if let Some(v) = self.floating_scale_percent {
            p.floating_scale = range(v, 50., 250., "floating_scale_percent")? / 100.;
        }
        if let Some(v) = self.floating_transparency_percent {
            p.floating_opacity = 1. - range(v, 0., 100., "floating_transparency_percent")? / 100.;
        }
        if let Some(v) = self.floating_position_pixels {
            for point in v {
                range(point, -100_000., 100_000., "floating_position_pixels")?;
            }
            if self.floating_center == Some(true) {
                return Err(invalid(
                    "Choose either floating_center or floating_position_pixels",
                ));
            }
            p.floating_position_pixels = Some(v);
            p.floating_position = None;
        }
        if self.floating_center == Some(true) {
            p.floating_position_pixels = None;
            p.floating_position = None;
        }
        Ok(p)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    SettingsShow,
    SettingsSet {
        patch: SettingsPatch,
    },
    SettingsReset,
    Fonts,
    WindowShow,
    WindowSet {
        width: Option<f32>,
        height: Option<f32>,
        x: Option<f32>,
        y: Option<f32>,
    },
    WindowState {
        state: WindowState,
    },
    Capture {
        output: PathBuf,
    },
    CaptureStatus,
    Panel {
        panel: Panel,
        open: bool,
    },
    ProjectShow,
    DirectoryList {
        path: PathBuf,
        offset: usize,
        limit: usize,
    },
    DirectoryExpand {
        path: PathBuf,
        expand: bool,
    },
    DirectoryCollapseAll,
    FilesRefresh,
    FileOpen {
        path: PathBuf,
    },
    FileClose,
    FileShow {
        content: bool,
        offset: usize,
        limit: usize,
    },
    FilePage {
        direction: PageDirection,
    },
    FileMode {
        source: bool,
    },
    ImageZoom {
        percent: Option<f32>,
    },
    Media {
        operation: MediaOperation,
        value: Option<f64>,
    },
    GitShow {
        section: GitSection,
        offset: usize,
        limit: usize,
    },
    GitRefresh,
    GitMore,
    GitInspect {
        id: String,
    },
    GitOpenDiff {
        path: String,
        commit: Option<String>,
    },
    GitCloseDiff,
    TerminalRead {
        id: String,
        offset: usize,
        limit: usize,
    },
    TerminalWrite {
        id: String,
        text: String,
        submit: bool,
    },
    TerminalKey {
        id: String,
        key: TerminalKey,
    },
    TerminalScroll {
        id: String,
        lines: Option<i32>,
        offset: Option<usize>,
    },
    SessionStop {
        id: String,
        force: bool,
        restart: bool,
    },
    SessionAcknowledge {
        id: String,
    },
    Quit {
        force: bool,
    },
}
macro_rules! enums {
    ($($name:ident { $($value:ident),+ $(,)? })*) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($value),+ }
    )* };
}
enums! {
    WindowState { Focus, Minimize, Maximize, Restore }
    Panel { Files, Git, History, Changes, Settings }
    PageDirection { Next, Previous, First, Reload }
    MediaOperation { Status, Play, Pause, Seek, Volume }
    GitSection { Status, Files, History, Commit, Diff }
    TerminalKey { Enter, Escape, Tab, Backspace, Up, Down, Left, Right, Home, End, PageUp, PageDown, CtrlC, CtrlD, CtrlL, CtrlZ }
}
impl TerminalKey {
    pub fn bytes(self, application_cursor: bool) -> &'static [u8] {
        match self {
            Self::Enter => b"\r",
            Self::Escape => b"\x1b",
            Self::Tab => b"\t",
            Self::Backspace => b"\x7f",
            Self::Up if application_cursor => b"\x1bOA",
            Self::Down if application_cursor => b"\x1bOB",
            Self::Right if application_cursor => b"\x1bOC",
            Self::Left if application_cursor => b"\x1bOD",
            Self::Up => b"\x1b[A",
            Self::Down => b"\x1b[B",
            Self::Right => b"\x1b[C",
            Self::Left => b"\x1b[D",
            Self::Home if application_cursor => b"\x1bOH",
            Self::End if application_cursor => b"\x1bOF",
            Self::Home => b"\x1b[H",
            Self::End => b"\x1b[F",
            Self::PageUp => b"\x1b[5~",
            Self::PageDown => b"\x1b[6~",
            Self::CtrlC => b"\x03",
            Self::CtrlD => b"\x04",
            Self::CtrlL => b"\x0c",
            Self::CtrlZ => b"\x1a",
        }
    }
}
impl Action {
    pub fn mutates(&self) -> bool {
        !matches!(
            self,
            Self::SettingsShow
                | Self::Fonts
                | Self::WindowShow
                | Self::CaptureStatus
                | Self::ProjectShow
                | Self::DirectoryList { .. }
                | Self::FileShow { .. }
                | Self::GitShow { .. }
                | Self::TerminalRead { .. }
                | Self::Media {
                    operation: MediaOperation::Status,
                    ..
                }
        )
    }
    /// Screenshot contents and read responses must never enter the receipt cache.
    pub fn cache_receipt(&self) -> bool {
        self.mutates() && !matches!(self, Self::Capture { .. })
    }
}

/// Keep request bodies (which may contain explicitly supplied terminal input)
/// out of the idempotency cache. The transport and reply still have bounded lifetimes.
#[cfg(windows)]
pub fn fingerprint(bytes: &[u8]) -> ApiResult<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::Security::Cryptography::*;
    let name: Vec<u16> = "SHA256\0".encode_utf16().collect();
    let mut algorithm = ptr::null_mut();
    let mut digest = vec![0u8; 32];
    unsafe {
        if BCryptOpenAlgorithmProvider(&mut algorithm, name.as_ptr(), ptr::null(), 0) < 0 {
            return Err(ApiError::new(
                "unavailable",
                "Cannot initialize request fingerprint",
            ));
        }
        let result = BCryptHash(
            algorithm,
            ptr::null(),
            0,
            bytes.as_ptr(),
            bytes.len() as u32,
            digest.as_mut_ptr(),
            digest.len() as u32,
        );
        BCryptCloseAlgorithmProvider(algorithm, 0);
        if result < 0 {
            return Err(ApiError::new("unavailable", "Cannot fingerprint request"));
        }
    }
    Ok(digest)
}
#[cfg(not(windows))]
pub fn fingerprint(_bytes: &[u8]) -> ApiResult<Vec<u8>> {
    Err(ApiError::new(
        "unsupported_platform",
        "Native controls require Windows",
    ))
}
