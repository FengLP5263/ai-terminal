//! Bounded, literal Git ignore classification for one lazily loaded directory.
use super::{GitError, command::Runner};
use std::{collections::HashSet, path::Path};

pub(crate) fn ignored_names(
    directory: &Path,
    names: &[String],
    cancelled: impl Fn() -> bool,
) -> Result<HashSet<String>, GitError> {
    if cancelled() {
        return Err(GitError::Cancelled);
    }
    let mut input = Vec::new();
    for name in names {
        if name.contains('\0') || input.len() + name.len() + 1 > 1024 * 1024 {
            continue;
        }
        input.extend_from_slice(name.as_bytes());
        input.push(0);
    }
    if input.is_empty() {
        return Ok(HashSet::new());
    }
    // The index remains enabled so tracked files are never treated as ignored.
    // Git handles nested rules, negation, info/exclude and core.excludesFile itself.
    let output = Runner::new()?.run_with_cancel(
        directory,
        &["check-ignore", "--stdin", "-z"],
        Some(&input),
        cancelled,
    )?;
    if !output.success && !output.error.is_empty() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.error)
                .chars()
                .take(700)
                .collect(),
        ));
    }
    output
        .data
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| {
            std::str::from_utf8(p)
                .map(|s| s.trim_end_matches('/').to_owned())
                .map_err(|_| GitError::InvalidUtf8)
        })
        .collect()
}
