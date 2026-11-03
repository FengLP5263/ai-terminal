//! Required, application-local console companions. No system installation or updates.
use std::path::Path;

pub const FILES: [&str; 2] = ["conpty.dll", "OpenConsole.exe"];

pub fn validate_directory(directory: &Path) -> Result<(), String> {
    let missing: Vec<_> = FILES
        .into_iter()
        .filter(|name| !directory.join(name).is_file())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "缺少原生控制台组件：{}。请完整解压发行包，将它们与 ai-terminal.exe 放在同一目录。",
            missing.join(", ")
        ))
    }
}

pub fn validate_installation() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        validate_directory(exe.parent().ok_or("无法确定程序目录")?)
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_native_console_is_reported_before_creating_a_session() {
        let root = tempfile::tempdir().unwrap();
        let error = validate_directory(root.path()).unwrap_err();
        assert!(error.contains("conpty.dll") && error.contains("OpenConsole.exe"));
        std::fs::write(root.path().join(FILES[0]), "fixture").unwrap();
        assert!(
            validate_directory(root.path())
                .unwrap_err()
                .contains(FILES[1])
        );
        std::fs::write(root.path().join(FILES[1]), "fixture").unwrap();
        assert!(validate_directory(root.path()).is_ok());
    }
}
