//! Read-only HEAD-to-working-file views. No index writes or external content drivers.
use super::{FilePatch, GitError, PatchKind, PatchLine, PatchSource, command::Runner, patch};
use std::{io::Read, path::Path};

const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub fn read_worktree_patch(directory: &Path, path: &str) -> Result<FilePatch, GitError> {
    read_with(&Runner::new()?, directory, path)
}

pub(super) fn read_with(r: &Runner, directory: &Path, path: &str) -> Result<FilePatch, GitError> {
    if path.is_empty()
        || path.len() > 16_384
        || path.contains('\0')
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(GitError::Command("Invalid working-file path".into()));
    }
    let root = std::path::PathBuf::from(super::text(
        &r.checked(directory, &["rev-parse", "--show-toplevel"])?,
    ));
    // Do not follow a symlink/junction to read files outside the repository.
    let mut file = root.clone();
    for component in Path::new(path).components() {
        file.push(component);
        match std::fs::symlink_metadata(&file) {
            Ok(meta) => {
                #[cfg(windows)]
                let reparse = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let reparse = false;
                if meta.file_type().is_symlink() || reparse {
                    return Err(GitError::Command(
                        "Linked working files are not supported".into(),
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(GitError::Command(e.to_string())),
        }
    }
    let isolated = r.without_filters(&root)?;
    let r = &isolated;
    let head = r.run(&root, &["rev-parse", "--verify", "-q", "HEAD"], None)?;
    let literal = format!(":(literal){path}");
    let tracked = r.run(
        &root,
        &["ls-files", "--error-unmatch", "--", &literal],
        None,
    )?;
    let mut view = if head.success && tracked.success {
        let bytes = r.checked(
            &root,
            &[
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--no-renames",
                "--ignore-submodules=all",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "--unified=20000",
                "HEAD",
                "--",
                &literal,
            ],
        )?;
        let mut view = patch::parse(PatchSource::WorkingTree, path, &bytes)?;
        if !view.binary && !view.lines.iter().any(|l| l.kind == PatchKind::Hunk) {
            append_body(&mut view, &read_body(&root.join(path))?, PatchKind::Context)?;
        }
        view
    } else {
        let mut view = patch::parse(PatchSource::WorkingTree, path, &[])?;
        let body = read_body(&root.join(path))?;
        // A staged deletion can leave the same path untracked. Compare both sides
        // rather than silently hiding the replacement file behind the index state.
        if head.success {
            let object = format!("{}:{path}", super::text(&head.data));
            let old = r.run(&root, &["cat-file", "blob", &object], None)?;
            if old.success {
                if old.data == body {
                    append_body(&mut view, &body, PatchKind::Context)?;
                } else {
                    append_body(&mut view, &old.data, PatchKind::Deleted)?;
                    append_body(&mut view, &body, PatchKind::Added)?;
                }
            } else {
                append_body(&mut view, &body, PatchKind::Added)?;
            }
        } else {
            append_body(&mut view, &body, PatchKind::Added)?;
        }
        view
    };
    let after = r.run(&root, &["rev-parse", "--verify", "-q", "HEAD"], None)?;
    if head.success != after.success || head.data != after.data {
        return Err(GitError::Command(
            "HEAD changed while reading file; refresh again".into(),
        ));
    }
    if r.cancelled() {
        return Err(GitError::Cancelled);
    }
    if view.binary {
        view.lines.clear();
    }
    patch::finish(r, view)
}

fn read_body(file: &Path) -> Result<Vec<u8>, GitError> {
    let mut input = match std::fs::File::open(file) {
        Ok(input) => input,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(GitError::Command(e.to_string())),
    };
    if !input
        .metadata()
        .map_err(|e| GitError::Command(e.to_string()))?
        .is_file()
    {
        return Err(GitError::Command("Not a regular working file".into()));
    }
    let mut bytes = Vec::new();
    (&mut input)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| GitError::Command(e.to_string()))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(GitError::Command("File exceeds the 2 MiB limit".into()));
    }
    Ok(bytes)
}

fn append_body(view: &mut FilePatch, bytes: &[u8], kind: PatchKind) -> Result<(), GitError> {
    if bytes.contains(&0) {
        view.binary = true;
        return Ok(());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| GitError::InvalidUtf8)?;
    for (index, text) in text.lines().enumerate() {
        if view.lines.len() >= patch::MAX_LINES {
            view.partial = true;
            break;
        }
        view.lines.push(PatchLine {
            kind,
            old: (kind != PatchKind::Added).then_some(index as u64 + 1),
            new: (kind != PatchKind::Deleted).then_some(index as u64 + 1),
            text: text.into(),
        });
    }
    Ok(())
}
