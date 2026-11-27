use super::{http, package};
use semver::Version;
use serde::Deserialize;
use std::io;

const REPOSITORY: &str = "FengLP5263/ai-terminal";
const UPDATE_MANIFEST: &str = "ai-terminal-windows-x64-update.json";
const RELEASE_LIMIT: usize = 64 * 1024;

#[derive(Clone, Default, Deserialize)]
pub(super) struct Notes {
    pub zh_cn: String,
    pub en: String,
}

#[derive(Clone)]
pub(super) struct Release {
    pub version: Version,
    pub notes: Notes,
    name: String,
    size: usize,
    sha256: String,
}

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    version: String,
    name: String,
    size: usize,
    sha256: String,
    dirty: bool,
    notes: Notes,
}

pub(super) fn check(current: &Version) -> io::Result<Option<Release>> {
    let path = format!("/{REPOSITORY}/releases/latest/download/{UPDATE_MANIFEST}");
    match http::get(&path, RELEASE_LIMIT, |_| {}) {
        Ok(metadata) => select(&metadata, current),
        // Releases before the in-app updater do not have this attachment.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

impl Release {
    pub(super) fn download(&self, mut report: impl FnMut(f32)) -> io::Result<Vec<u8>> {
        let path = format!(
            "/{REPOSITORY}/releases/download/app-v{}/{}",
            self.version, self.name
        );
        let bytes = http::get(&path, self.size, |received| {
            report(received as f32 / self.size as f32);
        })?;
        if bytes.len() != self.size || package::digest(&bytes)? != self.sha256 {
            return Err(io::Error::other(
                "The GitHub download does not match its SHA-256 digest",
            ));
        }
        Ok(bytes)
    }
}

fn select(bytes: &[u8], current: &Version) -> io::Result<Option<Release>> {
    let manifest: Manifest = serde_json::from_slice(bytes)?;
    let Ok(version) = Version::parse(&manifest.version) else {
        return Ok(None);
    };
    let prefix = format!("ai-terminal-v{version}-windows-x64-");
    let name_matches = manifest
        .name
        .strip_prefix(&prefix)
        .and_then(|suffix| suffix.strip_suffix(".zip"))
        .is_some_and(|stamp| {
            !stamp.is_empty()
                && stamp
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });
    if manifest.schema_version != 1
        || manifest.dirty
        || !version.pre.is_empty()
        || !version.build.is_empty()
        || &version <= current
        || !name_matches
        || !(1..=package::MAX_ARCHIVE_BYTES).contains(&manifest.size)
        || manifest.sha256.len() != 64
        || !manifest.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Ok(None);
    }
    Ok(Some(Release {
        version,
        notes: Notes {
            zh_cn: manifest.notes.zh_cn.chars().take(480).collect(),
            en: manifest.notes.en.chars().take(480).collect(),
        },
        name: manifest.name,
        size: manifest.size,
        sha256: manifest.sha256.to_ascii_lowercase(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[ignore = "uses the public GitHub Releases network"]
    fn verifies_a_published_windows_zip_over_https() {
        let release = check(&Version::new(0, 0, 0))
            .expect("read the GitHub latest release attachment through WinHTTP")
            .unwrap_or_else(|| Release {
                version: Version::new(0, 16, 0),
                notes: Notes::default(),
                name: "ai-terminal-v0.16.0-windows-x64-20261125T171542Z-d7ede8.zip".into(),
                size: 11_669_576,
                sha256: "21589e3ef3434cb14b4e653fb05e73a27212111683590d9dee371b78012d2878".into(),
            });
        let mut progress = 0.0;
        let bytes = release
            .download(|value| progress = value)
            .expect("download and verify GitHub asset");
        assert_eq!(progress, 1.0);
        let stage = tempfile::tempdir().unwrap();
        package::extract(bytes, &release.version, stage.path(), false).unwrap();
        package::verify_runtime(stage.path(), &release.version, false).unwrap();
        println!("Verified published Windows package {}", release.version);
    }

    #[test]
    fn selects_only_new_clean_stable_windows_updates() {
        let version = Version::new(0, 16, 0);
        let metadata = json!({
            "schema_version": 1, "version": "0.17.0", "dirty": false,
            "notes": {"zh_cn":"文件搜索", "en":"File search"},
            "name": "ai-terminal-v0.17.0-windows-x64-20261127T040000Z-abcdef.zip",
            "size": 1024, "sha256": "a".repeat(64)
        });
        let evaluate = |data: &serde_json::Value, current: &Version| {
            select(&serde_json::to_vec(data).unwrap(), current).unwrap()
        };
        let selected = evaluate(&metadata, &version).unwrap();
        assert_eq!(selected.version, Version::new(0, 17, 0));
        assert_eq!(selected.notes.en, "File search");
        assert!(evaluate(&metadata, &selected.version).is_none());
        for (key, value) in [
            ("dirty", json!(true)),
            ("name", json!("../../outside.zip")),
            ("schema_version", json!(2)),
            ("sha256", json!("missing")),
            ("size", json!(package::MAX_ARCHIVE_BYTES + 1)),
            ("version", json!("0.17.1-preview.1")),
        ] {
            let mut invalid = metadata.clone();
            invalid[key] = value;
            assert!(
                evaluate(&invalid, &version).is_none(),
                "accepted invalid {key}"
            );
        }
    }
}
