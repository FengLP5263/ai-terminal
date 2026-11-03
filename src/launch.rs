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
    let binary = search.resolve(spec.agent)?;
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
        (
            "AI_TERMINAL_HELPER".into(),
            helper.to_string_lossy().into_owned(),
        ),
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
                "PostToolBatch",
                "PermissionDenied",
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
            // Keep this managed terminal's engine, environment and tasks in its own job.
            arguments.push("--no-daemon".into());
            for setting in codex_hook_overrides()
                .into_iter()
                .chain(codex_notification_overrides())
            {
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

/// Enable explicit user-attention notifications only within this managed Codex invocation.
pub fn codex_notification_overrides() -> Vec<String> {
    vec![
        "tui.notifications=['approval-requested','plan-mode-prompt']".into(),
        "tui.notification_method='osc9'".into(),
        "tui.notification_condition='always'".into(),
    ]
}

pub fn codex_hook_overrides() -> Vec<String> {
    // Stable definition across install/preview paths; trust is reviewed by Codex.
    // The actual executable is supplied only in this managed session's environment.
    let shell_command = "& $env:AI_TERMINAL_HELPER --hook";
    let command = serde_json::to_string(shell_command).expect("valid string");
    ["SessionStart","UserPromptSubmit","PreToolUse","PermissionRequest","PostToolUse","Stop","Interrupt","SessionEnd"].into_iter().map(|event|format!("hooks.{event}=[{{hooks=[{{type=\"command\",command={command},command_windows={command},timeout=5}}]}}]")).collect()
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
    fn hook_config_has_stable_identity_and_never_auto_trusts() {
        let args = codex_hook_overrides();
        assert!(args.iter().all(|s| s.contains("$env:AI_TERMINAL_HELPER")));
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
