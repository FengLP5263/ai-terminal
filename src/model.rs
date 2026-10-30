use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Agent {
    #[default]
    Codex,
    Claude,
    Shell,
}
impl Agent {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
            Self::Shell => "PowerShell",
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UiSettings {
    pub sidebar_collapsed: bool,
    pub sidebar_width: f32,
    pub terminal_font_size: f32,
    pub ui_scale: f32,
}
impl Default for UiSettings {
    fn default() -> Self {
        Self {
            sidebar_collapsed: false,
            sidebar_width: 238.0,
            terminal_font_size: 14.0,
            ui_scale: 1.0,
        }
    }
}
impl UiSettings {
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
            self.sidebar_width.clamp(190.0, 420.0)
        } else {
            238.0
        };
        self.terminal_font_size = if self.terminal_font_size.is_finite() {
            self.terminal_font_size
                .clamp(Self::MIN_FONT, Self::MAX_FONT)
        } else {
            14.0
        };
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
    fn malformed_workspace_is_reported_instead_of_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        std::fs::write(&path, b"{").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"{");
    }
}
