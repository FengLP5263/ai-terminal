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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub sessions: Vec<SessionSpec>,
    pub active: Option<String>,
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
    fn malformed_workspace_is_reported_instead_of_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        std::fs::write(&path, b"{").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"{");
    }
}
