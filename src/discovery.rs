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
        if agent == Agent::Bash {
            return self.directories.iter().find_map(|directory| {
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
            });
        }
        let stem = match agent {
            Agent::Codex => "codex",
            Agent::Claude => "claude",
            Agent::Shell => "powershell",
            Agent::Cmd => "cmd",
            Agent::Bash => "bash",
        };
        for directory in &self.directories {
            for extension in ["exe", "cmd", "bat"] {
                let candidate = directory.join(format!("{stem}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
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
