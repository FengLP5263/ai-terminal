use super::package;
use std::{
    fs,
    io::{Cursor, Write},
};
use zip::write::SimpleFileOptions;

fn fixture(version: &str, payload: &[u8], checksum: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut records = serde_json::Map::new();
    let licenses = "licenses/two-face-0.5.2+bat-0.26.1/LICENSE-MIT";
    for name in package::RUNTIME.into_iter().chain([licenses]) {
        archive
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(payload).unwrap();
        records.insert(
            name.into(),
            serde_json::json!({
                "bytes": payload.len(), "sha256": package::digest(checksum).unwrap()
            }),
        );
    }
    archive
        .start_file("manifest.json", SimpleFileOptions::default())
        .unwrap();
    serde_json::to_writer(
        &mut archive,
        &serde_json::json!({
            "version": version, "platform": "windows-x64", "format_version": 2,
            "dirty": false, "files": records,
        }),
    )
    .unwrap();
    archive.finish().unwrap().into_inner()
}

#[test]
fn verifies_package_before_activation_and_detects_later_runtime_damage() {
    let version = semver::Version::new(0, 17, 0);
    let root = tempfile::tempdir().unwrap();
    let stage = root.path().join("candidate");
    fs::create_dir(&stage).unwrap();
    let current = root.path().join("ai-terminal.exe");
    fs::write(&current, b"existing launcher").unwrap();

    let bad = fixture("0.17.0", b"bad", b"good");
    assert!(package::extract(bad, &version, &stage, false).is_err());
    assert!(!stage.join("manifest.json").exists());
    assert_eq!(fs::read(&current).unwrap(), b"existing launcher");

    let valid = fixture("0.17.0", b"new", b"new");
    package::extract(valid, &version, &stage, false).unwrap();
    assert!(
        stage
            .join("licenses/two-face-0.5.2+bat-0.26.1/LICENSE-MIT")
            .is_file()
    );
    assert!(package::verify_runtime(&stage, &version, false).is_ok());
    assert!(package::verify_runtime(&stage, &semver::Version::new(0, 18, 0), false).is_err());
    fs::write(stage.join("OpenConsole.exe"), b"old").unwrap();
    assert!(package::verify_runtime(&stage, &version, false).is_err());
    assert_eq!(fs::read(&current).unwrap(), b"existing launcher");
}
