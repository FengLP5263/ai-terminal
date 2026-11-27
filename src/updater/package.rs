use semver::Version;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
};

pub(super) const MAX_ARCHIVE_BYTES: usize = 96 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 128 * 1024 * 1024;
const MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_FILES: usize = 1024;
pub(super) const RUNTIME: [&str; 5] = [
    "ai-terminal.exe",
    "ai-terminal-cli.exe",
    "ai-terminal-hook.exe",
    "conpty.dll",
    "OpenConsole.exe",
];

#[derive(Deserialize)]
struct Manifest {
    format_version: u32,
    platform: String,
    version: String,
    dirty: bool,
    files: BTreeMap<String, FileRecord>,
}
#[derive(Deserialize)]
struct FileRecord {
    bytes: u64,
    sha256: String,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(super) fn digest(bytes: &[u8]) -> io::Result<String> {
    use std::fmt::Write as _;
    let hash = crate::control::actions::fingerprint(bytes)
        .map_err(|_| io::Error::other("SHA-256 unavailable"))?;
    let mut encoded = String::with_capacity(64);
    for byte in hash {
        write!(encoded, "{byte:02x}").expect("write to String");
    }
    Ok(encoded)
}

fn read_member(reader: impl Read, expected: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(expected + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != expected {
        return Err(invalid("Unexpected size in the update package"));
    }
    Ok(bytes)
}

fn member_path(name: &str) -> io::Result<PathBuf> {
    let mut path = PathBuf::new();
    for part in name.split('/') {
        if part.is_empty()
            || part.starts_with('.')
            || part.ends_with('.')
            || !part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'+'))
        {
            return Err(invalid("Invalid path in the update package"));
        }
        let base = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && matches!(&base[..3], "COM" | "LPT")
                && base.as_bytes()[3].is_ascii_digit())
        {
            return Err(invalid("Reserved path in the update package"));
        }
        path.push(part);
    }
    Ok(path)
}

fn manifest(bytes: &[u8], version: &Version, local: bool) -> io::Result<Manifest> {
    let manifest: Manifest = serde_json::from_slice(bytes)?;
    if manifest.format_version != 2
        || manifest.platform != "windows-x64"
        || Version::parse(&manifest.version).ok().as_ref() != Some(version)
        || manifest.dirty && !local
        || manifest.files.len() > MAX_FILES
        || !RUNTIME
            .iter()
            .all(|name| manifest.files.contains_key(*name))
    {
        return Err(invalid(
            "The update package does not match the requested Windows build",
        ));
    }
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for (name, record) in &manifest.files {
        member_path(name)?;
        total = total.saturating_add(record.bytes);
        if name.eq_ignore_ascii_case("manifest.json")
            || !seen.insert(name.to_ascii_lowercase())
            || total > MAX_EXPANDED_BYTES
        {
            return Err(invalid(
                "Duplicate path or expanded size limit in the update package",
            ));
        }
    }
    Ok(manifest)
}

fn check_digest(bytes: &[u8], record: &FileRecord) -> io::Result<()> {
    if digest(bytes)? != record.sha256 {
        return Err(invalid("File SHA-256 mismatch in the update package"));
    }
    Ok(())
}

pub(super) fn extract(
    bytes: Vec<u8>,
    version: &Version,
    directory: &Path,
    local: bool,
) -> io::Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    if archive.len() > MAX_FILES + 1 {
        return Err(invalid("Too many files in the update package"));
    }
    let member = archive.by_name("manifest.json")?;
    let size = member.size();
    if size > MANIFEST_BYTES {
        return Err(invalid("The update manifest is too large"));
    }
    let bytes = read_member(member, size)?;
    let manifest = manifest(&bytes, version, local)?;
    if manifest.files.len() + 1 != archive.len() {
        return Err(invalid("The update archive contains unexpected files"));
    }
    for (name, record) in manifest.files {
        let member = archive.by_name(&name)?;
        if !member.is_file() || member.is_symlink() || member.size() != record.bytes {
            return Err(invalid("An update file is missing or invalid"));
        }
        let bytes = read_member(member, record.bytes)?;
        check_digest(&bytes, &record)?;
        let path = directory.join(member_path(&name)?);
        fs::create_dir_all(path.parent().expect("validated package path"))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("manifest.json"))?;
    output.write_all(&bytes)?;
    output.sync_all()
}

pub(super) fn verify_runtime(directory: &Path, version: &Version, local: bool) -> io::Result<()> {
    let mut bytes = Vec::new();
    fs::File::open(directory.join("manifest.json"))?
        .take(MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MANIFEST_BYTES {
        return Err(invalid("The update manifest is too large"));
    }
    let manifest = manifest(&bytes, version, local)?;
    for name in RUNTIME {
        let record = &manifest.files[name];
        let bytes = read_member(fs::File::open(directory.join(name))?, record.bytes)?;
        check_digest(&bytes, record)?;
    }
    Ok(())
}
