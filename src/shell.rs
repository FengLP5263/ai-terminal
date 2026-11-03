//! Per-session shell integration. No user configuration edits or output guessing.
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
    io::IsTerminal,
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
            "Git Bash was not found. Install Git for Windows or use PowerShell / CMD.".to_owned()
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
        ("AI_TERMINAL_SHELL_INVOCATION".into(), String::new()),
    ]);
    for (agent, name, key) in [
        (Agent::Codex, "codex.exe", "AI_TERMINAL_REAL_CODEX"),
        (Agent::Claude, "claude.exe", "AI_TERMINAL_REAL_CLAUDE"),
        (
            Agent::Shell,
            "powershell.exe",
            "AI_TERMINAL_REAL_POWERSHELL",
        ),
        (Agent::Cmd, "cmd.exe", "AI_TERMINAL_REAL_CMD"),
        (Agent::Bash, "bash.exe", "AI_TERMINAL_REAL_BASH"),
    ] {
        if let Some(real) = search.find(agent) {
            environment.insert(key.into(), real.to_string_lossy().into_owned());
        } else if agent.is_shell() {
            continue;
        }
        let target = bin.join(name);
        // Session-specific directories prevent an old child from observing replaced shims.
        if !target.exists() && std::fs::hard_link(helper, &target).is_err() {
            std::fs::copy(helper, &target).map_err(|e| e.to_string())?;
        }
    }
    let mut paths = vec![bin];
    if spec.agent == Agent::Bash
        && let Some(root) = crate::discovery::git_bash_root(&binary)
    {
        paths.push(root.join("usr/bin"));
        paths.push(root.join("bin"));
        // Match the installed Git architecture, rather than the developer's drive/user.
        for (directory, msystem) in [
            ("clangarm64", "CLANGARM64"),
            ("mingw64", "MINGW64"),
            ("mingw32", "MINGW32"),
        ] {
            if root.join(directory).is_dir() {
                environment.insert("MSYSTEM".into(), msystem.into());
                break;
            }
        }
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
            std::fs::write(&rc, bash_startup(&generation)).map_err(|e| e.to_string())?;
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

/// Reproduce Git Bash's login configuration before adding the session integration.
/// --rcfile gives us an insertion point after user startup without editing those files.
fn bash_startup(generation: &str) -> String {
    format!(
        r#"if [[ -r /etc/profile ]]; then source /etc/profile; fi
for __ai_startup in "$HOME/.bash_profile" "$HOME/.bash_login" "$HOME/.profile"; do
    if [[ -r "$__ai_startup" ]]; then source "$__ai_startup"; break; fi
done
unset __ai_startup
# Git and user profiles may reorder PATH; restore only our session's entry directory.
__ai_terminal_bin="$(/usr/bin/cygpath -u "$AI_TERMINAL_SHIM_DIR")"
export PATH="$__ai_terminal_bin:$PATH"
unset __ai_terminal_bin
hash -r
__ai_prompt=0
__ai_terminal_prompt() {{ local __ai_exit=$?; __ai_prompt=$((__ai_prompt+1)); printf "\033]2;ai-terminal-cwd;{generation};%s;%s\007" "$__ai_prompt" "$(pwd -W)"; return "$__ai_exit"; }}
PROMPT_COMMAND=("${{PROMPT_COMMAND[@]}}" __ai_terminal_prompt)
"#
    )
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
    } else if name.eq_ignore_ascii_case(std::ffi::OsStr::new("powershell")) {
        Some(Agent::Shell)
    } else if name.eq_ignore_ascii_case(std::ffi::OsStr::new("cmd")) {
        Some(Agent::Cmd)
    } else if name.eq_ignore_ascii_case(std::ffi::OsStr::new("bash")) {
        Some(Agent::Bash)
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
        parent_invocation: std::env::var("AI_TERMINAL_SHELL_INVOCATION")
            .ok()
            .filter(|id| !id.is_empty()),
        console_processes: if matches!(name, "ShellStarted" | "ShellAgentStarted") {
            Some(console_processes().unwrap_or_default())
        } else {
            None
        },
        local_id,
        generation,
        name: name.into(),
        session_id: String::new(),
        turn_id: None,
        tool_name: None,
        tool_use_id: None,
        notification_type: None,
        source: Some(
            match agent {
                Agent::Codex => "codex",
                Agent::Claude => "claude",
                Agent::Shell => "powershell",
                Agent::Cmd => "cmd",
                Agent::Bash => "bash",
            }
            .into(),
        ),
        background_running: false,
        exit_code,
    };
    let _ = bridge::send(&endpoint, &token, event);
}

/// Inspect only this helper's attached console at launch, never scan system processes.
#[cfg(windows)]
fn console_processes() -> Option<Vec<u32>> {
    let mut processes = vec![0; 256];
    // SAFETY: valid bounded output buffer; this API only reads our attached console.
    let count = unsafe {
        windows_sys::Win32::System::Console::GetConsoleProcessList(
            processes.as_mut_ptr(),
            processes.len() as u32,
        )
    } as usize;
    if count == 0 || count > processes.len() {
        return None;
    }
    processes.truncate(count);
    Some(processes)
}

#[cfg(not(windows))]
fn console_processes() -> Option<Vec<u32>> {
    None
}

fn interactive_shell(agent: Agent, args: &[OsString]) -> bool {
    args.iter().all(|arg| {
        let arg = arg.to_string_lossy().to_ascii_lowercase();
        match agent {
            Agent::Shell => matches!(arg.as_str(), "-nologo" | "-noprofile" | "-noexit"),
            Agent::Cmd => matches!(arg.as_str(), "/d" | "/q" | "/k"),
            Agent::Bash => matches!(
                arg.as_str(),
                "-i" | "-l" | "--login" | "--noprofile" | "--norc"
            ),
            _ => false,
        }
    })
}

pub fn run_nested_shell(agent: Agent, args: Vec<OsString>) -> i32 {
    let result = (|| -> Result<i32, String> {
        let key = match agent {
            Agent::Shell => "AI_TERMINAL_REAL_POWERSHELL",
            Agent::Cmd => "AI_TERMINAL_REAL_CMD",
            Agent::Bash => "AI_TERMINAL_REAL_BASH",
            _ => return Err("Not a shell entry".into()),
        };
        let binary = std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| CliSearch::current().find(agent))
            .ok_or_else(|| format!("{} was not found", agent.label()))?;
        let tracked = interactive_shell(agent, &args)
            && std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal()
            && !std::env::var_os("AI_TERMINAL_INVOCATION").is_some_and(|v| !v.is_empty());
        let mut command = std::process::Command::new(binary);
        command.args(args);
        let invocation = uuid::Uuid::new_v4().to_string();
        if tracked {
            keep_launcher_alive();
            notice("ShellStarted", agent, &invocation, None);
            command.env("AI_TERMINAL_SHELL_INVOCATION", &invocation);
        }
        let status = command.status();
        let exit_code = status.as_ref().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
        if tracked {
            notice("ShellEnded", agent, &invocation, Some(exit_code));
        }
        status.map(|_| exit_code).map_err(|error| error.to_string())
    })();
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn keep_launcher_alive() {
    #[cfg(windows)]
    // SAFETY: install a static handler only in this short-lived console launcher.
    unsafe {
        unsafe extern "system" fn keep_launcher(event: u32) -> i32 {
            i32::from(event == 0 || event == 1)
        }
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(keep_launcher), 1);
    }
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
            .filter(|p| p.is_file() && crate::discovery::stale_npm_target(p).is_none())
            .map(Ok)
            .unwrap_or_else(|| CliSearch::current().excluding(&root).resolve(agent))?;
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
        keep_launcher_alive();
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

#[cfg(test)]
mod nested_shell_tests {
    use super::*;
    #[test]
    fn interactive_entries_are_distinct_from_commands_and_scripts() {
        for (agent, args, expected) in [
            (Agent::Shell, vec![], true),
            (Agent::Shell, vec!["-NoProfile", "-NoLogo"], true),
            (Agent::Shell, vec!["-Command", "Write-Output ok"], false),
            (Agent::Shell, vec!["-c", "echo ok"], false),
            (Agent::Shell, vec!["-File", "example.ps1"], false),
            (Agent::Cmd, vec!["/d"], true),
            (Agent::Cmd, vec!["/k"], true),
            (Agent::Cmd, vec!["/k", "echo ok"], false),
            (Agent::Cmd, vec!["/c", "echo ok"], false),
            (Agent::Bash, vec!["--noprofile", "--norc", "-i"], true),
            (Agent::Bash, vec!["-lc", "echo ok"], false),
            (Agent::Bash, vec!["example.sh"], false),
        ] {
            assert_eq!(
                interactive_shell(
                    agent,
                    &args.into_iter().map(OsString::from).collect::<Vec<_>>()
                ),
                expected
            );
        }
    }
}
