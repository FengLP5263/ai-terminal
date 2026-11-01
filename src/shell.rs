//! Per-session shell integration. No user profiles, global PATH edits, or output guessing.
use crate::{
    bridge::{self, Bridge},
    discovery::CliSearch,
    launch::{self, Launch},
    model::{Agent, SessionSpec},
    status::HookEvent,
};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub const CWD_PREFIX: &str = "ai-terminal-cwd;";

pub fn build(
    spec: &SessionSpec,
    bridge: &Bridge,
    runtime: &Path,
    helper: &Path,
) -> Result<Launch, String> {
    let search = CliSearch::current();
    let binary = search.find(spec.agent).ok_or_else(|| {
        if spec.agent == Agent::Bash {
            "未找到本机 Bash，请安装 Git for Windows 后重试。".to_owned()
        } else {
            format!("未找到 {}", spec.agent.label())
        }
    })?;
    let generation = uuid::Uuid::new_v4().to_string();
    let runtime =
        std::path::absolute(runtime.join(&spec.id).join(&generation)).map_err(|e| e.to_string())?;
    let bin = runtime.join("bin");
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let mut environment = HashMap::from([
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
        ("TERM_PROGRAM".into(), "ai-terminal".into()),
        ("CLAUDECODE".into(), String::new()),
        ("AI_TERMINAL_ENDPOINT".into(), bridge.endpoint.clone()),
        ("AI_TERMINAL_TOKEN".into(), bridge.token.clone()),
        ("AI_TERMINAL_SESSION".into(), spec.id.clone()),
        ("AI_TERMINAL_GENERATION".into(), generation.clone()),
        (
            "AI_TERMINAL_HELPER".into(),
            helper.to_string_lossy().into_owned(),
        ),
        (
            "AI_TERMINAL_SESSION_RUNTIME".into(),
            runtime.to_string_lossy().into_owned(),
        ),
        (
            "AI_TERMINAL_SHIM_DIR".into(),
            bin.to_string_lossy().into_owned(),
        ),
        ("AI_TERMINAL_INVOCATION".into(), String::new()),
    ]);
    for (agent, name, key) in [
        (Agent::Codex, "codex.exe", "AI_TERMINAL_REAL_CODEX"),
        (Agent::Claude, "claude.exe", "AI_TERMINAL_REAL_CLAUDE"),
    ] {
        if let Some(real) = search.find(agent) {
            environment.insert(key.into(), real.to_string_lossy().into_owned());
        }
        let target = bin.join(name);
        // Session-specific directories prevent an old child from observing replaced shims.
        if !target.exists() && std::fs::hard_link(helper, &target).is_err() {
            std::fs::copy(helper, &target).map_err(|e| e.to_string())?;
        }
    }
    let mut paths = vec![bin];
    if spec.agent == Agent::Bash
        && let Some(root) = binary.parent().and_then(Path::parent)
    {
        paths.push(root.join("usr/bin"));
        paths.push(root.join("bin"));
    }
    paths.extend(std::env::split_paths(
        &search.child_path().map_err(|e| e.to_string())?,
    ));
    environment.insert(
        "PATH".into(),
        std::env::join_paths(paths)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned(),
    );
    let arguments = match spec.agent {
        Agent::Shell => {
            let code = format!(
                r#"[Console]::InputEncoding=[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $OutputEncoding=[Console]::OutputEncoding; function global:prompt {{ $script:__ai_prompt+=1; [Console]::Write(([char]27).ToString()+']2;ai-terminal-cwd;{};'+$script:__ai_prompt+';'+$PWD.ProviderPath+[char]7); 'PS '+$PWD.Path+'> ' }}; Clear-Host"#,
                generation
            );
            vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-NoExit".into(),
                "-Command".into(),
                code,
            ]
        }
        Agent::Cmd => {
            environment.insert(
                "PROMPT".into(),
                format!("$E]2;ai-terminal-cwd;{generation};$T;$P$E\\$P$G"),
            );
            vec!["/D".into()]
        }
        Agent::Bash => {
            let rc = runtime.join("bashrc");
            std::fs::write(&rc,format!(r#"__ai_prompt=0
__ai_terminal_prompt() {{ local __ai_exit=$?; __ai_prompt=$((__ai_prompt+1)); printf "\033]2;ai-terminal-cwd;{generation};%s;%s\007" "$__ai_prompt" "$(pwd -W)"; return "$__ai_exit"; }}
PROMPT_COMMAND=__ai_terminal_prompt
PS1='\w \$ '
"#)).map_err(|e| e.to_string())?;
            environment.insert("CHERE_INVOKING".into(), "1".into());
            vec![
                "--noprofile".into(),
                "--rcfile".into(),
                rc.to_string_lossy().into_owned(),
                "-i".into(),
            ]
        }
        _ => return Err("不是终端类型".into()),
    };
    let (executable, arguments) = launch::pty_command(&binary, arguments, helper)?;
    Ok(Launch {
        executable,
        arguments,
        environment,
        generation,
    })
}

pub fn directory_from_title(title: &str, generation: &str) -> Option<PathBuf> {
    let rest = title.strip_prefix(CWD_PREFIX)?;
    let (observed, rest) = rest.split_once(';')?;
    let (sequence, path) = rest.split_once(';')?;
    if sequence.len() > 128 {
        return None;
    }
    if observed != generation || path.len() > 4096 || path.chars().any(char::is_control) {
        return None;
    }
    let converted;
    let path = if path.as_bytes().first() == Some(&b'/')
        && path.as_bytes().get(2) == Some(&b'/')
        && path.as_bytes().get(1).is_some_and(u8::is_ascii_alphabetic)
    {
        converted = format!("{}:/{}", &path[1..2], &path[3..]);
        &converted
    } else {
        path
    };
    let path = PathBuf::from(path);
    (path.is_absolute() && path.is_dir()).then_some(path)
}

pub fn shim_agent() -> Option<Agent> {
    let executable = std::env::current_exe().ok()?;
    let root = PathBuf::from(std::env::var_os("AI_TERMINAL_SHIM_DIR")?);
    if executable.parent()?.canonicalize().ok()? != root.canonicalize().ok()? {
        return None;
    }
    let name = executable.file_stem()?;
    if name.eq_ignore_ascii_case(std::ffi::OsStr::new("codex")) {
        Some(Agent::Codex)
    } else if name.eq_ignore_ascii_case(std::ffi::OsStr::new("claude")) {
        Some(Agent::Claude)
    } else {
        None
    }
}

fn notice(name: &str, agent: Agent, invocation: &str, exit_code: Option<i32>) {
    let Some((endpoint, token, local_id, generation)) = std::env::var("AI_TERMINAL_ENDPOINT")
        .ok()
        .zip(std::env::var("AI_TERMINAL_TOKEN").ok())
        .zip(std::env::var("AI_TERMINAL_SESSION").ok())
        .zip(std::env::var("AI_TERMINAL_GENERATION").ok())
        .map(|(((e, t), l), g)| (e, t, l, g))
    else {
        return;
    };
    let event = HookEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        invocation: Some(invocation.into()),
        process_id: Some(std::process::id()),
        local_id,
        generation,
        name: name.into(),
        session_id: String::new(),
        turn_id: None,
        tool_name: None,
        tool_use_id: None,
        notification_type: None,
        source: Some(
            if agent == Agent::Codex {
                "codex"
            } else {
                "claude"
            }
            .into(),
        ),
        background_running: false,
        exit_code,
    };
    let _ = bridge::send(&endpoint, &token, event);
}

pub fn run_agent(agent: Agent, args: Vec<OsString>) -> i32 {
    let result = (|| -> Result<i32, String> {
        let root = PathBuf::from(std::env::var_os("AI_TERMINAL_SHIM_DIR").ok_or("缺少会话信息")?);
        let key = if agent == Agent::Codex {
            "AI_TERMINAL_REAL_CODEX"
        } else {
            "AI_TERMINAL_REAL_CLAUDE"
        };
        let binary = std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| CliSearch::current().excluding(&root).find(agent))
            .ok_or_else(|| format!("未找到 {}，请先安装对应 Windows CLI。", agent.label()))?;
        // Agent tools and nested agents must not replace the foreground session identity.
        if std::env::var_os("AI_TERMINAL_INVOCATION").is_some_and(|v| !v.is_empty()) {
            return std::process::Command::new(binary)
                .args(args)
                .status()
                .map(|s| s.code().unwrap_or(1))
                .map_err(|e| e.to_string());
        }
        let invocation = uuid::Uuid::new_v4().to_string();
        let helper = PathBuf::from(std::env::var_os("AI_TERMINAL_HELPER").ok_or("缺少 helper")?);
        let runtime =
            PathBuf::from(std::env::var_os("AI_TERMINAL_SESSION_RUNTIME").ok_or("缺少运行目录")?);
        let spec = SessionSpec {
            id: "shell-agent".into(),
            name: String::new(),
            directory: std::env::current_dir().map_err(|e| e.to_string())?,
            agent,
            agent_session_id: None,
            color: None,
        };
        let explicit_settings = args
            .iter()
            .any(|arg| arg == "--settings" || arg.to_string_lossy().starts_with("--settings="));
        let injected = if agent == Agent::Claude && explicit_settings {
            Vec::new()
        } else {
            launch::agent_arguments(&spec, &runtime, &helper)?
        };
        #[cfg(windows)]
        unsafe {
            unsafe extern "system" fn keep_launcher(event: u32) -> i32 {
                i32::from(event == 0 || event == 1)
            }
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(keep_launcher), 1);
        }
        notice("ShellAgentStarted", agent, &invocation, None);
        let status = std::process::Command::new(binary)
            .args(injected)
            .args(args)
            .env("AI_TERMINAL_INVOCATION", &invocation)
            .env("CLAUDECODE", "")
            .status();
        notice(
            "ShellAgentEnded",
            agent,
            &invocation,
            Some(status.as_ref().map(|s| s.code().unwrap_or(1)).unwrap_or(1)),
        );
        status
            .map(|s| s.code().unwrap_or(1))
            .map_err(|e| e.to_string())
    })();
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}
