use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Agent {
    Codex,
    Claude,
    QoderCn,
    Kimi,
    ZCode,
    Trae,
    #[default]
    Shell,
    Cmd,
    Bash,
}
impl Agent {
    pub fn is_shell(self) -> bool {
        matches!(self, Self::Shell | Self::Cmd | Self::Bash)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
            Self::QoderCn => "Qoder CN",
            Self::Kimi => "Kimi Code",
            Self::ZCode => "ZCode",
            Self::Trae => "TRAE",
            Self::Shell => "PowerShell",
            Self::Cmd => "CMD",
            Self::Bash => "Git Bash",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SessionColor {
    Blue,
    Green,
    Teal,
    Amber,
    Orange,
    Pink,
    Purple,
}
impl SessionColor {
    pub const ALL: [Self; 7] = [
        Self::Blue,
        Self::Green,
        Self::Teal,
        Self::Amber,
        Self::Orange,
        Self::Pink,
        Self::Purple,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Blue => "蓝色",
            Self::Green => "绿色",
            Self::Teal => "青色",
            Self::Amber => "黄色",
            Self::Orange => "橙色",
            Self::Pink => "粉色",
            Self::Purple => "紫色",
        }
    }
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Blue => [112, 181, 255],
            Self::Green => [115, 220, 153],
            Self::Teal => [93, 215, 211],
            Self::Amber => [246, 211, 112],
            Self::Orange => [255, 174, 128],
            Self::Pink => [239, 157, 200],
            Self::Purple => [189, 163, 255],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSpec {
    pub id: String,
    pub name: String,
    pub directory: PathBuf,
    pub agent: Agent,
    pub agent_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<SessionColor>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UiSettings {
    pub sidebar_collapsed: bool,
    pub sidebar_width: f32,
    pub terminal_font_size: f32,
    pub ui_scale: f32,
    pub language: crate::i18n::Language,
    pub floating_enabled: bool,
    pub floating_position: Option<[f32; 2]>,
    pub floating_position_pixels: Option<[f32; 2]>,
    pub desktop_shortcut: bool,
}
impl Default for UiSettings {
    fn default() -> Self {
        Self {
            sidebar_collapsed: false,
            sidebar_width: 238.0,
            terminal_font_size: 14.0,
            ui_scale: 1.0,
            language: crate::i18n::Language::System,
            floating_enabled: false,
            floating_position: None,
            floating_position_pixels: None,
            desktop_shortcut: false,
        }
    }
}
impl UiSettings {
    pub const MIN_SIDEBAR_WIDTH: f32 = 190.0;
    pub const MAX_SIDEBAR_WIDTH: f32 = 420.0;
    pub const MIN_ZOOM: f32 = 0.5;
    pub const MAX_ZOOM: f32 = 2.5;
    pub const MIN_FONT: f32 = 8.0;
    pub const MAX_FONT: f32 = 32.0;
    pub fn normalize(mut self) -> Self {
        self.ui_scale = if self.ui_scale.is_finite() {
            self.ui_scale.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM)
        } else {
            1.0
        };
        self.sidebar_width = if self.sidebar_width.is_finite() {
            self.sidebar_width
                .clamp(Self::MIN_SIDEBAR_WIDTH, Self::MAX_SIDEBAR_WIDTH)
        } else {
            238.0
        };
        self.terminal_font_size = if self.terminal_font_size.is_finite() {
            self.terminal_font_size
                .clamp(Self::MIN_FONT, Self::MAX_FONT)
        } else {
            14.0
        };
        self.floating_position = self
            .floating_position
            .filter(|p| p.iter().all(|v| v.is_finite()));
        self.floating_position_pixels = self
            .floating_position_pixels
            .filter(|p| p.iter().all(|v| v.is_finite()));
        self
    }
}

/// Move ownership, never clone/restart the live terminal or its Job Object.
pub fn reorder<T>(items: &mut Vec<T>, source: usize, target: usize, after: bool) -> bool {
    if source >= items.len() || target >= items.len() || source == target {
        return false;
    }
    let insertion = target + usize::from(after);
    let destination = insertion - usize::from(source < insertion);
    if source == destination {
        return false;
    }
    let item = items.remove(source);
    items.insert(destination, item);
    true
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub sessions: Vec<SessionSpec>,
    pub active: Option<String>,
    #[serde(default)]
    pub last_directory: Option<PathBuf>,
    #[serde(default)]
    pub ui: UiSettings,
}

pub fn data_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ai-terminal")
}
pub fn load(path: &Path) -> Result<Workspace, String> {
    if !path.exists() {
        return Ok(Workspace::default());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("无法读取会话记录：{e}"))
}
pub fn save(path: &Path, workspace: &Workspace) -> Result<(), String> {
    let parent = path.parent().ok_or("无效的保存路径")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, workspace).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_workspace_replace_keeps_restore_id_and_unicode_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        let mut w = Workspace::default();
        save(&path, &w).unwrap();
        w.sessions.push(SessionSpec {
            id: "one".into(),
            name: "中文项目".into(),
            directory: PathBuf::from("C:/项目/with space"),
            agent: Agent::Claude,
            agent_session_id: Some("actual-agent-id".into()),
            color: None,
        });
        save(&path, &w).unwrap();
        let read = load(&path).unwrap();
        assert_eq!(
            read.sessions[0].agent_session_id.as_deref(),
            Some("actual-agent-id")
        );
        assert_eq!(read.sessions[0].name, "中文项目");
    }
    #[test]
    fn old_workspace_keeps_sessions_and_defaults_new_preferences() {
        let old = r#"{"sessions":[],"active":null}"#;
        let workspace: Workspace = serde_json::from_str(old).unwrap();
        assert_eq!(workspace.ui, UiSettings::default());
        assert_eq!(workspace.last_directory, None);
        let mut workspace = workspace;
        workspace.ui.sidebar_collapsed = true;
        workspace.ui.terminal_font_size = 18.0;
        let read: Workspace =
            serde_json::from_slice(&serde_json::to_vec(&workspace).unwrap()).unwrap();
        assert_eq!(read.ui, workspace.ui);
        assert_eq!(
            UiSettings {
                terminal_font_size: 999.0,
                sidebar_width: 0.0,
                ..Default::default()
            }
            .normalize()
            .terminal_font_size,
            32.0
        );
    }
    #[test]
    fn moving_rows_preserves_identity_and_handles_both_directions() {
        let mut rows = vec!["a", "b", "c", "d"];
        assert!(reorder(&mut rows, 0, 2, true));
        assert_eq!(rows, ["b", "c", "a", "d"]);
        assert!(reorder(&mut rows, 3, 0, false));
        assert_eq!(rows, ["d", "b", "c", "a"]);
        assert!(!reorder(&mut rows, 0, 1, false));
        assert!(!reorder(&mut rows, 99, 0, false));
    }

    #[test]
    fn per_session_colors_roundtrip_and_old_sessions_default_to_uncolored() {
        let old = r#"{"sessions":[{"id":"one","name":"项目 👋🏽","directory":"C:/project","agent":"Shell","agent_session_id":null}],"active":"one"}"#;
        let mut workspace: Workspace = serde_json::from_str(old).unwrap();
        assert_eq!(workspace.sessions[0].color, None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        workspace.sessions[0].color = Some(SessionColor::Purple);
        save(&path, &workspace).unwrap();
        let read = load(&path).unwrap();
        assert_eq!(read.sessions[0].color, Some(SessionColor::Purple));
        assert_eq!(read.sessions[0].name, "项目 👋🏽");
        workspace.sessions[0].color = None;
        save(&path, &workspace).unwrap();
        assert_eq!(load(&path).unwrap().sessions[0].color, None);
    }
    #[test]
    fn malformed_workspace_is_reported_instead_of_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        std::fs::write(&path, b"{").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"{");
    }
}

/// Prefer the active terminal's latest directory, then the saved last directory.
pub fn default_directory(active: Option<&Path>, last: Option<&Path>) -> PathBuf {
    active
        .into_iter()
        .chain(last)
        .filter(|path| path.is_dir())
        .map(Path::to_path_buf)
        .chain(
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .filter(|p| p.is_dir()),
        )
        .chain(std::env::current_dir().ok())
        .next()
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
mod directory_tests {
    use super::*;
    #[test]
    fn defaults_prefer_active_cwd_then_saved_last_directory() {
        let root = tempfile::tempdir().unwrap();
        let active = root.path().join("active");
        let last = root.path().join("last");
        std::fs::create_dir(&active).unwrap();
        std::fs::create_dir(&last).unwrap();
        assert_eq!(default_directory(Some(&active), Some(&last)), active);
        assert_eq!(
            default_directory(Some(&root.path().join("deleted")), Some(&last)),
            last
        );
        let path = root.path().join("workspace.json");
        let workspace = Workspace {
            last_directory: Some(last.clone()),
            ..Default::default()
        };
        save(&path, &workspace).unwrap();
        assert_eq!(load(&path).unwrap().last_directory, Some(last));
    }
}
