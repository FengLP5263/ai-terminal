use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
pub fn fixture() -> &'static Path {
    static BUILT: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &BUILT
        .get_or_init(|| {
            let root = tempfile::tempdir().unwrap();
            let output = root.path().join("cli-fixture.exe");
            let result = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "cli_args_fixture"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_args.rs"))
                .arg("-o")
                .arg(&output)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "fixture build: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            (root, output)
        })
        .1
}
