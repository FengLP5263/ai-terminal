use std::{path::Path, path::PathBuf, process::Command, sync::OnceLock};

pub fn executable() -> &'static Path {
    static BUILT: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &BUILT
        .get_or_init(|| {
            let root = tempfile::tempdir().unwrap();
            let executable = root.path().join("text-console.exe");
            let result = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "text_console_fixture"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_console.rs"))
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            (root, executable)
        })
        .1
}
