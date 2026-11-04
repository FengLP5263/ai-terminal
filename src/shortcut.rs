//! First-run desktop shortcut. One attempt, tolerant failure, no user config edits.
use crate::model::UiSettings;
use std::path::Path;

pub const FILE_NAME: &str = "ai-terminal.lnk";

/// Only the installed app's default data directory may touch the user's Desktop.
/// Portable and test runs (`--data-dir`, smoke) stay isolated.
pub fn should_attempt(ui: &UiSettings, skip: bool, data_dir: &Path) -> bool {
    !ui.desktop_shortcut && !skip && data_dir == crate::model::data_directory()
}

/// Records the attempt before touching the Desktop so a transient failure is
/// not retried on every launch.
pub fn ensure_first_run(ui: &mut UiSettings, skip: bool, data_dir: &Path) -> bool {
    if !should_attempt(ui, skip, data_dir) {
        return false;
    }
    ui.desktop_shortcut = true;
    if let Ok(target) = std::env::current_exe() {
        std::thread::spawn(move || {
            if let Err(error) = create(&target) {
                eprintln!("desktop shortcut not created: {error}");
            }
        });
    }
    true
}

#[cfg(windows)]
fn create(target: &Path) -> Result<(), String> {
    let desktop = windows_impl::desktop_directory()?;
    windows_impl::create_link(&desktop.join(FILE_NAME), target)
}

#[cfg(not(windows))]
fn create(_target: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
mod windows_impl {
    use std::{
        os::windows::ffi::OsStrExt,
        path::{Path, PathBuf},
    };
    use windows::{
        Win32::{
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree, CoUninitialize, IPersistFile,
            },
            UI::Shell::{
                FOLDERID_Desktop, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellLink,
            },
        },
        core::{Interface, PCWSTR},
    };

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    pub fn desktop_directory() -> Result<PathBuf, String> {
        // SAFETY: reads the Desktop known folder; the returned string is freed once.
        unsafe {
            let path = SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None)
                .map_err(|error| error.to_string())?;
            let directory = path.to_string().map_err(|error| error.to_string());
            CoTaskMemFree(Some(path.0 as *const _));
            Ok(PathBuf::from(directory?))
        }
    }

    pub fn create_link(destination: &Path, target: &Path) -> Result<(), String> {
        let target_wide = wide(target);
        let working_wide = wide(target.parent().unwrap_or(target));
        let icon_wide = target_wide.clone();
        let description = wide(Path::new("ai-terminal"));
        let destination_wide = wide(destination);
        // SAFETY: all strings are NUL-terminated for the duration of the calls;
        // the shell link is released before CoUninitialize on this thread.
        unsafe {
            let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let result = (|| -> windows::core::Result<()> {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                link.SetPath(PCWSTR(target_wide.as_ptr()))?;
                link.SetWorkingDirectory(PCWSTR(working_wide.as_ptr()))?;
                link.SetDescription(PCWSTR(description.as_ptr()))?;
                link.SetIconLocation(PCWSTR(icon_wide.as_ptr()), 0)?;
                let file: IPersistFile = link.cast()?;
                file.Save(PCWSTR(destination_wide.as_ptr()), true)?;
                Ok(())
            })();
            if initialized {
                CoUninitialize();
            }
            result.map_err(|error| error.to_string())
        }
    }

    #[cfg(test)]
    pub fn description(path: &Path) -> Result<String, String> {
        use windows::Win32::System::Com::STGM_READ;
        let path = wide(path);
        // SAFETY: a private STA reads back the link created above.
        unsafe {
            let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let result = (|| -> windows::core::Result<String> {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                let file: IPersistFile = link.cast()?;
                file.Load(PCWSTR(path.as_ptr()), STGM_READ)?;
                let mut text = [0u16; 64];
                link.GetDescription(&mut text)?;
                let length = text.iter().position(|&c| c == 0).unwrap_or(text.len());
                Ok(String::from_utf16_lossy(&text[..length]))
            })();
            if initialized {
                CoUninitialize();
            }
            result.map_err(|error| error.to_string())
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::model::Workspace;

    #[test]
    fn shortcut_points_at_this_executable_and_records_one_attempt() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(FILE_NAME);
        let target = std::env::current_exe().unwrap();
        windows_impl::create_link(&destination, &target).unwrap();
        assert!(std::fs::metadata(&destination).unwrap().len() > 0);
        assert_eq!(
            windows_impl::description(&destination).unwrap(),
            "ai-terminal"
        );
    }

    #[test]
    fn isolated_or_non_first_runs_never_touch_the_desktop() {
        let root = tempfile::tempdir().unwrap();
        let mut ui = UiSettings::default();
        let installed = crate::model::data_directory();
        assert!(should_attempt(&ui, false, &installed));
        assert!(!should_attempt(&ui, true, &installed));
        assert!(!should_attempt(&ui, false, root.path()));
        assert!(!ensure_first_run(&mut ui, false, root.path()));
        assert!(!ui.desktop_shortcut);
        ui.desktop_shortcut = true;
        assert!(!should_attempt(&ui, false, &installed));
    }

    #[test]
    fn old_workspace_defaults_the_shortcut_flag_without_saving_it() {
        let workspace: Workspace =
            serde_json::from_str(r#"{"sessions":[],"active":null}"#).unwrap();
        assert!(!workspace.ui.desktop_shortcut);
    }
}
