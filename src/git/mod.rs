//! Local read-only Git snapshots. No model access, credentials, writes, or network.
mod command;
mod ignore;
pub(crate) use ignore::ignored_names;
mod commit;
mod patch;
mod service;
pub mod syntax;
mod watch;
mod worktree;
use command::Runner;
pub use commit::{CommitDetails, CommitFile, Inspection, read_commit};
pub use patch::{FilePatch, PatchKind, PatchLine, PatchResult, PatchSource, read_patch};
pub use service::GitClient;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
pub use worktree::read_worktree_patch;

pub const PAGE: usize = 50;
pub const MAX_COMMITS: usize = 300;
pub const MAX_FILES: usize = 500;
pub const MAX_WORKTREE_FILES: usize = 10_000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileChange {
    pub path: String,
    pub index: char,
    pub worktree: char,
    pub added: u64,
    pub deleted: u64,
    pub binary: bool,
    pub omitted: bool,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct Commit {
    pub id: String,
    pub parents: Vec<String>,
    pub author: String,
    pub timestamp: i64,
    pub subject: String,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct Snapshot {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
    pub branch: String,
    pub detached: bool,
    pub unborn: bool,
    pub added: u64,
    pub deleted: u64,
    pub total_files: usize,
    pub files: Vec<FileChange>,
    pub partial: bool,
    pub omitted_files: usize,
    pub commits: Vec<Commit>,
    pub more: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    Missing,
    Command(String),
    InvalidUtf8,
    Cancelled,
}

pub fn read(directory: &Path, limit: usize) -> Result<Option<Snapshot>, GitError> {
    let runner = Runner::new()?;
    read_with(&runner, directory, limit)
}

fn text(data: &[u8]) -> String {
    String::from_utf8_lossy(data)
        .trim_end_matches(['\r', '\n'])
        .into()
}

fn read_with(r: &Runner, directory: &Path, limit: usize) -> Result<Option<Snapshot>, GitError> {
    let inside = r.run(directory, &["rev-parse", "--is-inside-work-tree"], None)?;
    if !inside.success {
        let error = text(&inside.error);
        if error.contains("not a git repository") {
            return Ok(None);
        }
        return Err(GitError::Command(error));
    }
    if text(&inside.data) != "true" {
        return Ok(None);
    }
    let root = PathBuf::from(text(
        &r.checked(directory, &["rev-parse", "--show-toplevel"])?,
    ));
    let git_dir = PathBuf::from(text(
        &r.checked(&root, &["rev-parse", "--absolute-git-dir"])?,
    ));
    let common = PathBuf::from(text(&r.checked(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?));
    let isolated = r.without_filters(&root)?;
    let r = &isolated;
    let head = r.run(&root, &["rev-parse", "--verify", "-q", "HEAD"], None)?;
    let symbolic = r.run(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"], None)?;
    let detached = !symbolic.success && head.success;
    let branch = if symbolic.success {
        text(&symbolic.data)
    } else if head.success {
        text(&head.data).chars().take(10).collect()
    } else {
        String::new()
    };
    let status = r.checked(
        &root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
            "--ignore-submodules=all",
        ],
    )?;
    let mut files = parse_status(&status)?;
    let filtered = if r.filters_disabled() && !files.is_empty() {
        let input: Vec<u8> = files
            .iter()
            .filter(|f| f.index != '?')
            .flat_map(|f| f.path.bytes().chain([0]))
            .collect();
        let output = r.run(
            &root,
            &["check-attr", "-z", "--stdin", "filter"],
            Some(&input),
        )?;
        if !output.success {
            return Err(GitError::Command(
                "Unable to inspect Git content-filter attributes.".into(),
            ));
        }
        let fields: Vec<_> = output
            .data
            .split(|b| *b == 0)
            .filter(|f| !f.is_empty())
            .collect();
        if !fields.len().is_multiple_of(3) {
            return Err(GitError::Command("Invalid Git attribute output".into()));
        }
        fields
            .chunks(3)
            .filter(|row| row[2] != b"unspecified" && row[2] != b"unset")
            .map(|row| String::from_utf8_lossy(row[0]).into_owned())
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };
    let total_files = files.len();
    // One diff against HEAD counts the net staged + unstaged changes without double counting.
    let diff = if head.success {
        r.checked(
            &root,
            &[
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--ignore-submodules=all",
                "--numstat",
                "-z",
                "HEAD",
                "--",
            ],
        )?
    } else {
        // Without HEAD, Git's cached diff compares the index against the empty tree.
        r.checked(
            &root,
            &[
                "diff",
                "--cached",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--ignore-submodules=all",
                "--numstat",
                "-z",
                "--",
            ],
        )?
    };
    let stats = parse_numstat(&diff)?;
    let (mut added, mut deleted) = (0u64, 0u64);
    for (path, value) in &stats {
        if filtered.contains(path) {
            continue;
        }
        added = added.saturating_add(value.0);
        deleted = deleted.saturating_add(value.1);
    }
    for file in &mut files {
        if r.cancelled() {
            return Err(GitError::Cancelled);
        }
        if file.index == '?' {
            continue;
        }
        if filtered.contains(&file.path) {
            file.omitted = true;
            continue;
        }
        if let Some(&(a, d, binary)) = stats.get(&file.path) {
            file.added = a;
            file.deleted = d;
            file.binary = binary;
        }
    }
    let omitted_files = files.iter().filter(|f| f.omitted).count();
    let partial = omitted_files > 0;
    // Display limits do not change Git's totals; untracked files contribute no lines.
    files.truncate(MAX_WORKTREE_FILES);
    let limit = limit.clamp(1, MAX_COMMITS);
    let mut commits = if head.success {
        parse_log(&r.checked(
            &root,
            &[
                "log",
                "--no-show-signature",
                "--no-color",
                "--topo-order",
                &format!("--max-count={}", limit + 1),
                "--format=%H%x00%P%x00%an%x00%at%x00%s",
                "-z",
                "HEAD",
                "--",
            ],
        )?)?
    } else {
        Vec::new()
    };
    let more = commits.len() > limit;
    commits.truncate(limit);
    // A changed HEAD invalidates the entire result rather than mixing two branches.
    let after = r.run(&root, &["rev-parse", "--verify", "-q", "HEAD"], None)?;
    if head.success != after.success || head.data != after.data {
        return Err(GitError::Command(
            "Repository changed during refresh; refresh again.".into(),
        ));
    }
    Ok(Some(Snapshot {
        root,
        git_dir,
        common_dir: common,
        branch,
        detached,
        unborn: !head.success,
        added,
        deleted,
        total_files,
        files,
        partial,
        omitted_files,
        commits,
        more,
    }))
}

fn parse_status(bytes: &[u8]) -> Result<Vec<FileChange>, GitError> {
    bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|entry| {
            if entry.len() < 4 || entry[2] != b' ' {
                return Err(GitError::Command("Invalid Git status output".into()));
            }
            Ok(FileChange {
                path: String::from_utf8_lossy(&entry[3..]).into(),
                index: entry[0] as char,
                worktree: entry[1] as char,
                added: 0,
                deleted: 0,
                binary: false,
                omitted: false,
            })
        })
        .collect()
}
fn parse_numstat(bytes: &[u8]) -> Result<HashMap<String, (u64, u64, bool)>, GitError> {
    let mut out = HashMap::new();
    for entry in bytes.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let mut parts = entry.splitn(3, |b| *b == b'\t');
        let a = parts.next().unwrap();
        let d = parts
            .next()
            .ok_or_else(|| GitError::Command("Invalid Git numstat output".into()))?;
        let path = parts
            .next()
            .ok_or_else(|| GitError::Command("Invalid Git numstat path".into()))?;
        let binary = a == b"-" || d == b"-";
        let number = |s: &[u8]| -> Result<u64, GitError> {
            if binary {
                Ok(0)
            } else {
                text(s)
                    .parse()
                    .map_err(|_| GitError::Command("Invalid Git line count".into()))
            }
        };
        out.insert(
            String::from_utf8_lossy(path).into(),
            (number(a)?, number(d)?, binary),
        );
    }
    Ok(out)
}
fn parse_log(bytes: &[u8]) -> Result<Vec<Commit>, GitError> {
    let parts: Vec<_> = bytes.split(|b| *b == 0).collect();
    let fields = if parts.last() == Some(&&[][..]) {
        &parts[..parts.len() - 1]
    } else {
        &parts[..]
    };
    if fields.len() % 5 != 0 {
        return Err(GitError::Command("Invalid Git history output".into()));
    }
    fields
        .chunks(5)
        .map(|row| {
            let id = text(row[0]);
            if !matches!(id.len(), 40 | 64) || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(GitError::Command("Invalid commit ID".into()));
            }
            Ok(Commit {
                id,
                parents: text(row[1]).split_whitespace().map(str::to_owned).collect(),
                author: text(row[2]),
                timestamp: text(row[3])
                    .parse()
                    .map_err(|_| GitError::Command("Invalid commit time".into()))?,
                subject: text(row[4]),
            })
        })
        .collect()
}
