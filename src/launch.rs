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
    crate::discovery::CliSearch::current().find(agent)
}

/// A native helper lets Rust handle batch-file quoting instead of interpolating
/// a cmd.exe command line through the terminal emulator's C-style argument quoting.
pub fn pty_command(
    binary: &Path,
    mut arguments: Vec<String>,
    helper: &Path,
) -> Result<(String, Vec<String>), String> {
    let program = if crate::discovery::is_batch(binary) {
        if !helper.is_file() {
            return Err("缺少 ai-terminal-hook.exe，请完整解压发行包".into());
        }
        arguments.insert(0, binary.to_string_lossy().into_owned());
        arguments.insert(0, "--launch-cli".into());
        helper
    } else {
        binary
    };
    let program = program.to_string_lossy();
    let executable = if program.contains([' ', '\t']) {
        format!("\"{program}\"")
    } else {
        program.into_owned()
    };
    Ok((executable, arguments))
}

/// Runs only in the lightweight helper, inside the owned ConPTY / Job Object.
/// Inherit interactive handles and propagate the CLI's actual exit status.
pub fn run_cli(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> i32 {
    let Some(program) = arguments.next() else {
        return 2;
    };
    match std::process::Command::new(program).args(arguments).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("无法启动 CLI：{error}");
            1
        }
    }
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
    if spec.agent.is_shell() {
        return crate::shell::build(spec, bridge, runtime, helper);
    }
    let search = crate::discovery::CliSearch::current();
    let binary = search.find(spec.agent).ok_or_else(|| format!(
        "未找到 {}。已检查 PATH、Windows 用户／系统 PATH 和常见安装目录。请确认本机可运行对应 CLI；自定义安装请将启动器目录加入用户 PATH，或从可运行该 CLI 的终端启动 ai-terminal。仅在 WSL 中安装的 CLI 暂不支持。", spec.agent.label()))?;
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
    environment.insert(
        "PATH".into(),
        search
            .child_path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned(),
    );
    // Avoid inheriting a parent Claude invocation's nested-session marker.
    environment.insert("CLAUDECODE".into(), String::new());
    arguments.extend(agent_arguments(spec, runtime, helper)?);
    let (executable, arguments) = pty_command(&binary, arguments, helper)?;
    Ok(Launch {
        executable,
        arguments,
        environment,
        generation,
    })
}

pub fn agent_arguments(
    spec: &SessionSpec,
    runtime: &Path,
    helper: &Path,
) -> Result<Vec<String>, String> {
    let mut arguments = Vec::new();
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
            let bytes = serde_json::to_vec(&serde_json::json!({"hooks":hooks}))
                .map_err(|e| e.to_string())?;
            if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
                std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            }
            arguments.extend(["--settings".into(), path.to_string_lossy().into_owned()]);
            if let Some(id) = &spec.agent_session_id {
                arguments.extend(["--resume".into(), id.clone()]);
            }
        }
        Agent::Codex => {
            // Keep Codex output in the terminal's normal scrollback grid.
            arguments.push("--no-alt-screen".into());
            for setting in codex_hook_overrides(helper) {
                arguments.extend(["-c".into(), setting]);
            }
            if let Some(id) = &spec.agent_session_id {
                arguments.extend(["resume".into(), id.clone()]);
            }
        }
        Agent::Shell | Agent::Cmd | Agent::Bash => {}
    }
    Ok(arguments)
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
            color: None,
        };
        if executable(Agent::Claude).is_none() {
            return;
        }
        let helper = dir.path().join("ai-terminal-hook.exe");
        std::fs::write(&helper, b"fixture").unwrap();
        let plan = build(&spec, &bridge, dir.path(), &helper).unwrap();
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
