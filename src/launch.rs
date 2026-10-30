use crate::{
    bridge::Bridge,
    model::{Agent, SessionSpec},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct Launch {
    pub executable: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub generation: String,
}

pub fn executable(agent: Agent) -> Option<PathBuf> {
    let name = match agent {
        Agent::Codex => "codex.exe",
        Agent::Claude => "claude.exe",
        Agent::Shell => "powershell.exe",
    };
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let profile = std::env::var_os("USERPROFILE").map(PathBuf::from)?;
    let roaming = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| profile.join("AppData/Roaming"));
    let candidates:Vec<PathBuf>=match agent {
        Agent::Codex=>vec![roaming.join("npm/node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe"),profile.join(".local/bin/codex.exe")],
        Agent::Claude=>vec![profile.join(".local/bin/claude.exe"),roaming.join("npm/node_modules/@anthropic-ai/claude-code/bin/claude.exe"),roaming.join("npm/node_modules/@anthropic-ai/claude-code/node_modules/@anthropic-ai/claude-code-win32-x64/claude.exe")],
        Agent::Shell=>vec![PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(||"C:/Windows".into())).join("System32/WindowsPowerShell/v1.0/powershell.exe")],
    };
    candidates.into_iter().find(|p| p.is_file())
}

pub fn build(
    spec: &SessionSpec,
    bridge: &Bridge,
    runtime: &Path,
    helper: &Path,
) -> Result<Launch, String> {
    if !spec.directory.is_dir() {
        return Err("项目目录不存在".into());
    }
    let binary = executable(spec.agent)
        .ok_or_else(|| format!("未找到 {}，请先安装对应 CLI", spec.agent.label()))?;
    let generation = uuid::Uuid::new_v4().to_string();
    let mut arguments = vec![];
    let mut environment = HashMap::from([
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
        ("TERM_PROGRAM".into(), "ai-terminal".into()),
        ("AI_TERMINAL_ENDPOINT".into(), bridge.endpoint.clone()),
        ("AI_TERMINAL_TOKEN".into(), bridge.token.clone()),
        ("AI_TERMINAL_SESSION".into(), spec.id.clone()),
        ("AI_TERMINAL_GENERATION".into(), generation.clone()),
    ]);
    // Avoid inheriting a parent Claude invocation's nested-session marker.
    environment.insert("CLAUDECODE".into(), String::new());
    match spec.agent {
        Agent::Claude => {
            let mut hooks = serde_json::Map::new();
            for event in [
                "SessionStart",
                "UserPromptSubmit",
                "PreToolUse",
                "PermissionRequest",
                "PostToolUse",
                "PostToolUseFailure",
                "Notification",
                "Stop",
                "StopFailure",
                "SessionEnd",
                "Elicitation",
                "ElicitationResult",
            ] {
                hooks.insert(event.into(),serde_json::json!([{"hooks":[{"type":"command","command":helper,"args":["--hook"],"timeout":2}]}]));
            }
            std::fs::create_dir_all(runtime).map_err(|e| e.to_string())?;
            let path = runtime.join(format!("{}-claude.json", spec.id));
            std::fs::write(
                &path,
                serde_json::to_vec(&serde_json::json!({"hooks":hooks}))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            arguments.extend(["--settings".into(), path.to_string_lossy().into_owned()]);
            if let Some(id) = &spec.agent_session_id {
                arguments.extend(["--resume".into(), id.clone()]);
            }
        }
        Agent::Codex => {
            for setting in codex_hook_overrides(helper) {
                arguments.extend(["-c".into(), setting]);
            }
            if let Some(id) = &spec.agent_session_id {
                arguments.extend(["resume".into(), id.clone()]);
            }
        }
        Agent::Shell => {
            arguments.extend(["-NoLogo".into(),"-NoProfile".into(),"-NoExit".into(),"-Command".into(),"[Console]::InputEncoding=[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $OutputEncoding=[Console]::OutputEncoding; Clear-Host".into()]);
        }
    }
    // The Alacritty Windows backend quotes argument values; executable needs its own quotes.
    let program = binary.to_string_lossy();
    let executable = if program.contains(' ') {
        format!("\"{program}\"")
    } else {
        program.into_owned()
    };
    Ok(Launch {
        executable,
        arguments,
        environment,
        generation,
    })
}

pub fn codex_hook_overrides(helper: &Path) -> Vec<String> {
    let shell_command = format!(
        "& '{}' --hook",
        helper.to_string_lossy().replace('\'', "''")
    );
    let command = serde_json::to_string(&shell_command).expect("valid string");
    ["SessionStart","UserPromptSubmit","PreToolUse","PermissionRequest","PostToolUse","Stop","Interrupt","SessionEnd"].into_iter().map(|event|format!("hooks.{event}=[{{hooks=[{{type=\"command\",command={command},command_windows={command},timeout=2}}]}}]")).collect()
}

/// Called at process entry, before our GUI or session threads exist. This is
/// process-local: it never changes Windows user/machine environment settings.
pub fn prepare_gui_environment() {
    // The coding-agent launcher injects NO_COLOR=1/TERM=dumb for command output;
    // those flags do not describe a newly created, full-color interactive PTY.
    // SAFETY: run before application-created threads; Windows environment
    // mutation is supported by std on this platform.
    unsafe {
        std::env::remove_var("NO_COLOR");
        std::env::remove_var("FORCE_COLOR");
        std::env::remove_var("CLICOLOR_FORCE");
        std::env::set_var("TERM", "xterm-256color");
        std::env::set_var("COLORTERM", "truecolor");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn hook_config_quotes_paths_and_never_auto_trusts() {
        let args = codex_hook_overrides(Path::new("C:/测试/it's a folder/ai-terminal.exe"));
        assert!(args.iter().all(|s| s.contains("it''s a folder")));
        assert!(
            !args
                .iter()
                .any(|s| s.contains("trusted") || s.contains("bypass"))
        );
    }
    #[test]
    fn launch_is_scoped_and_keeps_actual_resume_id() {
        let dir = tempfile::tempdir().unwrap();
        let bridge = Bridge::start(Arc::new(|| {})).unwrap();
        let spec = SessionSpec {
            id: "local".into(),
            name: "test".into(),
            directory: dir.path().into(),
            agent: Agent::Claude,
            agent_session_id: Some("actual-id".into()),
        };
        if executable(Agent::Claude).is_none() {
            return;
        }
        let plan = build(
            &spec,
            &bridge,
            dir.path(),
            Path::new("C:/tools/ai-terminal.exe"),
        )
        .unwrap();
        assert!(
            plan.arguments
                .windows(2)
                .any(|s| s == ["--resume", "actual-id"])
        );
        assert!(
            !plan
                .environment
                .keys()
                .any(|k| k.contains("ANTHROPIC") || k.contains("OPENAI"))
        );
        let config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("local-claude.json")).unwrap())
                .unwrap();
        assert_eq!(config.as_object().unwrap().len(), 1);
        assert!(config.get("hooks").is_some());
    }
}
