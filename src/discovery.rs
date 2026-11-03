//! Resolve public CLI entry points, not package-manager private directory layouts.
use crate::model::Agent;
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

pub struct CliSearch {
    directories: Vec<PathBuf>,
}

impl CliSearch {
    /// Read once per launch; never poll the registry or mutate the GUI environment.
    pub fn current() -> Self {
        let mut directories = Vec::new();
        if let Some(path) = std::env::var_os("PATH") {
            directories.extend(std::env::split_paths(&path));
        }
        // Explorer and an already-open GUI can have an older environment block.
        for path in persistent_paths() {
            directories.extend(std::env::split_paths(&path));
        }
        directories.extend(git_installations().into_iter().map(|root| root.join("bin")));
        directories.extend(common_directories(|name| std::env::var_os(name)));
        let search = Self::from_directories(directories);
        if let Some(inherited) = std::env::var_os("AI_TERMINAL_SHIM_DIR") {
            search.excluding(&PathBuf::from(inherited))
        } else {
            search
        }
    }

    pub fn excluding(mut self, directory: &std::path::Path) -> Self {
        self.directories
            .retain(|p| !p.as_os_str().eq_ignore_ascii_case(directory.as_os_str()));
        self
    }

    pub fn from_directories(directories: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut seen = HashSet::new();
        let directories = directories
            .into_iter()
            .filter(|path| {
                // Never implicitly search the selected project or an empty PATH element.
                path.is_absolute() && seen.insert(path.as_os_str().to_ascii_uppercase())
            })
            .collect();
        Self { directories }
    }

    pub fn find(&self, agent: Agent) -> Option<PathBuf> {
        self.resolve(agent).ok()
    }

    pub fn resolve(&self, agent: Agent) -> Result<PathBuf, String> {
        if agent == Agent::Bash {
            return self
                .directories
                .iter()
                .find_map(|directory| {
                    let direct = directory.join("bash.exe");
                    if direct.is_file() && git_bash_root(&direct).is_some() {
                        return Some(direct);
                    }
                    // Git normally adds cmd/, rather than bin/, to the Windows PATH.
                    directory.ancestors().take(3).find_map(|root| {
                        [root.join("bin/bash.exe"), root.join("usr/bin/bash.exe")]
                            .into_iter()
                            .find(|path| path.is_file() && git_bash_root(path).is_some())
                    })
                })
                .ok_or_else(|| {
                    "Git Bash was not found. Install Git for Windows or use PowerShell / CMD."
                        .to_owned()
                });
        }
        let stem = match agent {
            Agent::Codex => "codex",
            Agent::Claude => "claude",
            Agent::Shell => "powershell",
            Agent::Cmd => "cmd",
            Agent::Bash => "bash",
        };
        let mut stale = Vec::new();
        for directory in &self.directories {
            for extension in ["exe", "cmd", "bat"] {
                let candidate = directory.join(format!("{stem}.{extension}"));
                if candidate.is_file() {
                    if matches!(agent, Agent::Codex | Agent::Claude)
                        && let Some(target) = stale_npm_target(&candidate)
                    {
                        stale.push(format!("{} → {}", candidate.display(), target.display()));
                        continue;
                    }
                    return Ok(candidate);
                }
            }
        }
        if stale.is_empty() {
            Err(format!(
                "未找到 {}。已检查 PATH、Windows 用户／系统 PATH 和常见安装目录。请确认本机可运行对应 Windows CLI；自定义安装请将启动器目录加入用户 PATH。仅在 WSL 中安装的 CLI 暂不支持。",
                agent.label()
            ))
        } else {
            Err(format!(
                "{} 启动器存在，但其目标文件缺失：\n{}\n未找到其他可用入口，请修复对应安装或 PATH。",
                agent.label(),
                stale.join("\n")
            ))
        }
    }

    /// The same lookup path must reach Node and the tools used by a public shim.
    pub fn child_path(&self) -> Result<OsString, std::env::JoinPathsError> {
        std::env::join_paths(&self.directories)
    }

    pub fn bash_fallback(&self) -> Option<Agent> {
        [Agent::Shell, Agent::Cmd]
            .into_iter()
            .find(|&agent| self.find(agent).is_some())
    }
}

/// Inspect only the literal final target of the standard npm-generated public
/// batch wrapper. Do not execute it, guess package layouts, or interpret custom scripts.
/// Template source: https://github.com/npm/cmd-shim/blob/main/lib/index.js
pub fn stale_npm_target(path: &std::path::Path) -> Option<PathBuf> {
    use std::io::Read;
    if !is_batch(path) || std::fs::metadata(path).ok()?.len() > 65_536 {
        return None;
    }
    let mut script = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(65_537)
        .read_to_string(&mut script)
        .ok()?;
    let lines: Vec<_> = script
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let head = [
        "@echo off",
        "goto start",
        ":find_dp0",
        "set dp0=%~dp0",
        "exit /b",
        ":start",
        "setlocal",
        "call :find_dp0",
    ];
    if lines.len() <= head.len()
        || !lines
            .iter()
            .zip(head)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    {
        return None;
    }
    let body = &lines[head.len()..];
    // These are the native and Node templates emitted by npm/cmd-shim. A
    // modified wrapper (for example one that installs its target) stays opaque.
    let node = [
        "if exist \"%dp0%\\node.exe\" (",
        "set \"_prog=%dp0%\\node.exe\"",
        ") else (",
        "set \"_prog=node\"",
        ")",
    ];
    let old_node = [
        "if exist \"%dp0%\\node.exe\" (",
        "set \"_prog=%dp0%\\node.exe\"",
        ") else (",
        "set \"_prog=node\"",
        "set pathext=%pathext:;.js;=;%",
        ")",
    ];
    let native = body.len() == 1;
    let node = body.len() == node.len() + 1
        && body
            .iter()
            .zip(node)
            .all(|(a, b)| a.eq_ignore_ascii_case(b));
    let old_node = body.len() == old_node.len() + 1
        && body
            .iter()
            .zip(old_node)
            .all(|(a, b)| a.eq_ignore_ascii_case(b));
    if !native && !node && !old_node {
        return None;
    }
    let marker = "endlocal & goto #_undefined_# 2>nul || title %comspec% &";
    let line = *body.last()?;
    let lower = line.to_ascii_lowercase();
    if !native && !lower.starts_with(marker) {
        return None;
    }
    let command = if lower.starts_with(marker) {
        line[marker.len()..].trim_start()
    } else {
        line
    };
    let target = command.strip_suffix("%*")?.trim_end().strip_suffix('"')?;
    if native && (!target.starts_with('"') || target[1..].contains('"')) {
        return None;
    }
    let opening_quote = target.rfind('"')?;
    if !native
        && !["\"%_prog%\"", "set pathext=%pathext:;.js;=;% & \"%_prog%\""]
            .iter()
            .any(|expected| {
                target[..opening_quote]
                    .trim_end()
                    .eq_ignore_ascii_case(expected)
            })
    {
        return None;
    }
    let literal = &target[opening_quote + 1..];
    let prefix = "%dp0%\\";
    if !literal.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {
        return None;
    }
    let relative = &literal[prefix.len()..];
    if relative.is_empty() || relative.contains(['%', '!', '^', '\r', '\n']) {
        return None;
    }
    let target = path
        .parent()?
        .join(relative.replace('\\', std::path::MAIN_SEPARATOR_STR));
    match std::fs::metadata(&target) {
        Ok(info) if !info.is_file() => Some(target),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(target),
        _ => None, // Unknown, unreadable or custom launchers retain PATH precedence.
    }
}

/// Require the Git for Windows layout; never treat WSL aliases or Cygwin as Git Bash.
pub fn git_bash_root(binary: &std::path::Path) -> Option<PathBuf> {
    binary.parent()?.ancestors().take(3).find_map(|root| {
        (root.join("usr/bin/bash.exe").is_file()
            && root.join("etc/profile").is_file()
            && (root.join("cmd/git.exe").is_file() || root.join("bin/git.exe").is_file()))
        .then(|| root.to_path_buf())
    })
}

fn common_directories(mut env: impl FnMut(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for (name, suffix) in [
        ("NPM_CONFIG_PREFIX", ""),
        ("PNPM_HOME", ""),
        ("NVM_SYMLINK", ""),
        ("VOLTA_HOME", "bin"),
        ("BUN_INSTALL", "bin"),
        ("APPDATA", "npm"),
        ("LOCALAPPDATA", "pnpm"),
        ("USERPROFILE", ".local/bin"),
        ("USERPROFILE", ".bun/bin"),
        ("USERPROFILE", ".volta/bin"),
        ("USERPROFILE", "scoop/shims"),
        ("USERPROFILE", "scoop/apps/git/current/bin"),
        ("ProgramFiles", "nodejs"),
        ("ProgramFiles", "Git/bin"),
        ("ProgramFiles(x86)", "Git/bin"),
        ("ProgramW6432", "Git/bin"),
        ("LOCALAPPDATA", "Programs/Git/bin"),
        ("SystemRoot", "System32"),
        ("SystemRoot", "System32/WindowsPowerShell/v1.0"),
    ] {
        if let Some(root) = env(name).filter(|value| !value.is_empty()) {
            paths.push(PathBuf::from(root).join(suffix));
        }
    }
    paths
}

#[cfg(not(windows))]
fn persistent_paths() -> Vec<OsString> {
    Vec::new()
}

#[cfg(windows)]
fn persistent_paths() -> Vec<OsString> {
    use windows_sys::Win32::System::Registry::*;
    [
        registry_string(
            HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            "Path",
            0,
        ),
        registry_string(HKEY_CURRENT_USER, "Environment", "Path", 0),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(not(windows))]
fn git_installations() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn git_installations() -> Vec<PathBuf> {
    use windows_sys::Win32::System::Registry::*;
    [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
        .into_iter()
        .flat_map(|root| {
            [RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY]
                .into_iter()
                .filter_map(move |view| {
                    registry_string(root, r"SOFTWARE\GitForWindows", "InstallPath", view)
                        .map(PathBuf::from)
                        .filter(|path| path.is_absolute())
                })
        })
        .collect()
}

#[cfg(windows)]
fn registry_string(
    root: windows_sys::Win32::System::Registry::HKEY,
    key: &str,
    name: &str,
    view: u32,
) -> Option<OsString> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Registry::*;
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | view;
    let mut bytes = 0;
    // SAFETY: predefined registry roots; checked bounded UTF-16 buffers. Read only.
    unsafe {
        if RegGetValueW(
            root,
            key.as_ptr(),
            name.as_ptr(),
            flags,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        ) != 0
            || bytes == 0
            || bytes > 131_072
        {
            return None;
        }
        let mut value = vec![0u16; (bytes as usize).div_ceil(2)];
        if RegGetValueW(
            root,
            key.as_ptr(),
            name.as_ptr(),
            flags,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        ) != 0
        {
            return None;
        }
        let length = value.iter().position(|&c| c == 0).unwrap_or(value.len());
        Some(OsString::from_wide(&value[..length]))
    }
}

pub fn is_batch(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case(OsStr::new("cmd"))
            || extension.eq_ignore_ascii_case(OsStr::new("bat"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn npm_fixture(root: &std::path::Path, target: &str, node: bool) -> PathBuf {
        std::fs::create_dir_all(root).unwrap();
        let file = root.join("claude.cmd");
        let head = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n";
        let body = if node {
            format!(
                "IF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n)\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & set PATHEXT=%PATHEXT:;.JS;=;% & \"%_prog%\" \"%dp0%\\{target}\" %*\r\n"
            )
        } else {
            format!("\"%dp0%\\{target}\"  %*\r\n")
        };
        std::fs::write(&file, format!("{head}{body}")).unwrap();
        file
    }

    #[test]
    fn stale_npm_launcher_falls_through_without_changing_path_or_running_scripts() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("old prefix");
        let good = root.path().join("healthy prefix");
        let stale = npm_fixture(&old, "arbitrary\\native.exe", false);
        let healthy = npm_fixture(&good, "custom\\entry.js", true);
        std::fs::create_dir_all(good.join("custom")).unwrap();
        std::fs::write(good.join("custom/entry.js"), "fixture").unwrap();
        let search = CliSearch::from_directories([old.clone(), good.clone()]);
        assert_eq!(search.resolve(Agent::Claude), Ok(healthy));
        assert_eq!(
            stale_npm_target(&stale),
            Some(old.join("arbitrary/native.exe"))
        );
        assert_eq!(
            std::env::split_paths(&search.child_path().unwrap()).collect::<Vec<_>>(),
            [old, good]
        );
    }

    #[test]
    fn every_stale_candidate_is_reported_and_custom_installers_remain_opaque() {
        let root = tempfile::tempdir().unwrap();
        let first = npm_fixture(&root.path().join("first"), "missing.exe", false);
        let second = npm_fixture(&root.path().join("second"), "missing.js", true);
        let search = CliSearch::from_directories([
            first.parent().unwrap().to_owned(),
            second.parent().unwrap().to_owned(),
        ]);
        let error = search.resolve(Agent::Claude).unwrap_err();
        assert!(
            error.contains(&first.display().to_string())
                && error.contains(&second.display().to_string())
        );
        assert!(error.contains("missing.exe") && error.contains("missing.js"));
        let custom = std::fs::read_to_string(&second).unwrap().replace(
            "& set PATHEXT",
            "& call install-if-needed.cmd & set PATHEXT",
        );
        std::fs::write(&second, custom).unwrap();
        assert_eq!(stale_npm_target(&second), None);
        let modified = std::fs::read_to_string(&first).unwrap().replace(
            "CALL :find_dp0",
            "CALL :find_dp0\r\ncall install-if-needed.cmd",
        );
        std::fs::write(&first, modified).unwrap();
        assert_eq!(stale_npm_target(&first), None);
        assert_eq!(search.find(Agent::Claude), Some(first));
    }
    fn git_fixture(root: &std::path::Path) {
        for file in [
            "bin/bash.exe",
            "usr/bin/bash.exe",
            "cmd/git.exe",
            "etc/profile",
        ] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "fixture").unwrap();
        }
    }

    #[test]
    fn git_bash_discovers_portable_git_from_cmd_and_usr_paths() {
        let root = tempfile::tempdir().unwrap();
        let portable = root.path().join("自定义 磁盘/PortableGit");
        git_fixture(&portable);
        let expected = portable.join("bin/bash.exe");
        assert_eq!(
            CliSearch::from_directories([portable.join("cmd")]).find(Agent::Bash),
            Some(expected.clone())
        );
        assert_eq!(
            CliSearch::from_directories([portable.join("usr/bin")]).find(Agent::Bash),
            Some(portable.join("usr/bin/bash.exe"))
        );
        assert_eq!(git_bash_root(&expected), Some(portable));
    }

    #[test]
    fn git_bash_rejects_unrelated_bash_and_prefers_path_order() {
        let root = tempfile::tempdir().unwrap();
        let unrelated = root.path().join("WindowsApps");
        std::fs::create_dir(&unrelated).unwrap();
        std::fs::write(unrelated.join("bash.exe"), "WSL or unrelated Bash").unwrap();
        let first = root.path().join("first-git");
        let second = root.path().join("second-git");
        git_fixture(&first);
        git_fixture(&second);
        assert!(
            CliSearch::from_directories([unrelated.clone()])
                .find(Agent::Bash)
                .is_none()
        );
        assert_eq!(
            CliSearch::from_directories([unrelated, first.join("cmd"), second.join("bin")])
                .find(Agent::Bash),
            Some(first.join("bin/bash.exe"))
        );
    }

    #[test]
    fn missing_git_bash_offers_powershell_then_cmd_and_no_fake_bash() {
        let root = tempfile::tempdir().unwrap();
        let search = CliSearch::from_directories([root.path().to_path_buf()]);
        assert!(search.find(Agent::Bash).is_none());
        assert_eq!(search.bash_fallback(), None);
        std::fs::write(root.path().join("cmd.exe"), "fixture").unwrap();
        assert_eq!(search.bash_fallback(), Some(Agent::Cmd));
        std::fs::write(root.path().join("powershell.exe"), "fixture").unwrap();
        assert_eq!(search.bash_fallback(), Some(Agent::Shell));
    }
    #[test]
    fn public_shims_custom_prefix_and_path_precedence_do_not_depend_on_package_layout() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("自定义 npm prefix");
        let second = root.path().join("other version");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("codex.cmd"), "@echo off").unwrap();
        std::fs::write(second.join("codex.exe"), "fixture").unwrap();
        let search = CliSearch::from_directories([first.clone(), second]);
        assert_eq!(search.find(Agent::Codex), Some(first.join("codex.cmd")));
        std::fs::write(first.join("codex.exe"), "fixture").unwrap();
        assert_eq!(search.find(Agent::Codex), Some(first.join("codex.exe")));
    }
    #[test]
    fn fresh_persistent_path_reaches_child_without_changing_parent_environment() {
        let root = tempfile::tempdir().unwrap();
        let stale = root.path().join("old-path");
        let fresh = root.path().join("new user path");
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(fresh.join("claude.bat"), "fixture").unwrap();
        let original = std::env::var_os("PATH");
        let search = CliSearch::from_directories([
            stale.clone(),
            fresh.clone(),
            fresh.clone(),
            PathBuf::new(),
            PathBuf::from("."),
        ]);
        assert_eq!(search.find(Agent::Claude), Some(fresh.join("claude.bat")));
        assert_eq!(
            std::env::split_paths(&search.child_path().unwrap()).collect::<Vec<_>>(),
            [stale, fresh]
        );
        assert_eq!(std::env::var_os("PATH"), original);
    }
    #[test]
    fn fallback_roots_come_from_the_users_environment() {
        let root = tempfile::tempdir().unwrap();
        let paths = common_directories(|name| match name {
            "USERPROFILE" => Some(root.path().join("different-user").into_os_string()),
            "APPDATA" => Some(root.path().join("roaming-moved").into_os_string()),
            "NPM_CONFIG_PREFIX" => Some(root.path().join("custom-prefix").into_os_string()),
            _ => None,
        });
        assert!(paths.contains(&root.path().join("different-user/.local/bin")));
        assert!(paths.contains(&root.path().join("roaming-moved/npm")));
        assert!(paths.contains(&root.path().join("custom-prefix")));
        assert!(
            !paths
                .iter()
                .any(|path| path.to_string_lossy().contains("node_modules"))
        );
    }
}
