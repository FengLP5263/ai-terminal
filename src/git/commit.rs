//! On-demand immutable commit inspection; never reads working-tree file bodies.
use super::{GitError, MAX_FILES, command::Runner};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitFile {
    pub path: String,
    pub status: char,
    pub added: u64,
    pub deleted: u64,
    pub binary: bool,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitDetails {
    pub id: String,
    pub message: String,
    pub parents: Vec<String>,
    pub files: Vec<CommitFile>,
    pub total_files: usize,
    pub added: u64,
    pub deleted: u64,
    pub partial: bool,
}
#[derive(Clone)]
pub struct Inspection {
    pub id: String,
    pub data: Option<Arc<CommitDetails>>,
    pub error: Option<GitError>,
    pub loading: bool,
}

pub fn read_commit(directory: &Path, id: &str) -> Result<CommitDetails, GitError> {
    read_with(&Runner::new()?, directory, id)
}
fn valid_id(id: &str) -> bool {
    matches!(id.len(), 40 | 64) && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn read_with(r: &Runner, directory: &Path, id: &str) -> Result<CommitDetails, GitError> {
    if !valid_id(id) {
        return Err(GitError::Command("Invalid commit ID".into()));
    }
    let metadata = r.checked(
        directory,
        &[
            "show",
            "--no-show-signature",
            "--no-notes",
            "--no-patch",
            "--format=%H%x00%P%x00%B",
            id,
            "--",
        ],
    )?;
    let fields: Vec<_> = metadata.splitn(3, |b| *b == 0).collect();
    if fields.len() != 3 || super::text(fields[0]) != id {
        return Err(GitError::Command("Invalid commit metadata".into()));
    }
    let parents: Vec<_> = super::text(fields[1])
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    if parents.iter().any(|p| !valid_id(p)) {
        return Err(GitError::Command("Invalid commit parent".into()));
    }
    // Explicit first-parent comparison for merges; --root includes initial files.
    let diff = |format| {
        let mut args = vec![
            "diff-tree",
            "--root",
            "--no-commit-id",
            "-r",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=all",
            format,
            "-z",
        ];
        if let Some(parent) = parents.first() {
            args.push(parent);
        }
        args.extend([id, "--"]);
        r.checked(directory, &args)
    };
    let names = diff("--name-status")?;
    let counts = super::parse_numstat(&diff("--numstat")?)?;
    let mut parts = names.split(|b| *b == 0).collect::<Vec<_>>();
    if parts.last() == Some(&&[][..]) {
        parts.pop();
    }
    if !parts.len().is_multiple_of(2) {
        return Err(GitError::Command("Invalid commit file list".into()));
    }
    let total_files = parts.len() / 2;
    let mut files = Vec::new();
    for pair in parts.chunks(2).take(MAX_FILES) {
        if pair[0].len() != 1 || !b"ADMT".contains(&pair[0][0]) || pair[1].is_empty() {
            return Err(GitError::Command("Invalid commit file status".into()));
        }
        let path = String::from_utf8_lossy(pair[1]).into_owned();
        let &(added, deleted, binary) = counts
            .get(&path)
            .ok_or_else(|| GitError::Command("Commit file statistics are incomplete".into()))?;
        files.push(CommitFile {
            path,
            status: pair[0][0] as char,
            added,
            deleted,
            binary,
        });
    }
    Ok(CommitDetails {
        id: id.into(),
        message: super::text(fields[2]),
        parents,
        files,
        total_files,
        added: counts.values().fold(0u64, |n, v| n.saturating_add(v.0)),
        deleted: counts.values().fold(0u64, |n, v| n.saturating_add(v.1)),
        partial: total_files > MAX_FILES,
    })
}

struct Request {
    root: PathBuf,
    commit: String,
    generation: u64,
}
type Response = (u64, String, Result<CommitDetails, GitError>);
/// One pending request/result, independent cancellation from workspace refreshes.
pub(super) struct Inspector {
    request: Arc<Mutex<Option<Request>>>,
    response: Arc<Mutex<Option<Response>>>,
    stop: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    signal: SyncSender<()>,
}
impl Inspector {
    pub fn new(ctx: egui::Context) -> Self {
        let (signal, rx) = mpsc::sync_channel(1);
        let this = Self {
            request: Arc::new(Mutex::new(None::<Request>)),
            response: Arc::new(Mutex::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            signal,
        };
        let (request, response, stop, generation) = (
            this.request.clone(),
            this.response.clone(),
            this.stop.clone(),
            this.generation.clone(),
        );
        std::thread::Builder::new()
            .name("git-commit".into())
            .spawn(move || {
                while rx.recv().is_ok() && !stop.load(Ordering::Acquire) {
                    let Some(request) = request.lock().unwrap().take() else {
                        continue;
                    };
                    let result =
                        Runner::cancellable(stop.clone(), generation.clone(), request.generation)
                            .and_then(|r| read_with(&r, &request.root, &request.commit));
                    if generation.load(Ordering::Acquire) == request.generation
                        && !stop.load(Ordering::Acquire)
                        && !matches!(result, Err(GitError::Cancelled))
                    {
                        *response.lock().unwrap() =
                            Some((request.generation, request.commit, result));
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                }
            })
            .expect("Git commit worker");
        this
    }
    pub fn clear(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.request.lock().unwrap() = None;
        *self.response.lock().unwrap() = None;
    }
    pub fn inspect(&self, root: &Path, commit: &str) {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        *self.request.lock().unwrap() = Some(Request {
            root: root.into(),
            commit: commit.into(),
            generation,
        });
        let _ = self.signal.try_send(());
    }
    pub fn poll(&self) -> Option<Inspection> {
        let (generation, id, result) = self.response.lock().unwrap().take()?;
        if generation != self.generation.load(Ordering::Acquire) {
            return None;
        }
        Some(match result {
            Ok(data) => Inspection {
                id,
                data: Some(Arc::new(data)),
                error: None,
                loading: false,
            },
            Err(error) => Inspection {
                id,
                data: None,
                error: Some(error),
                loading: false,
            },
        })
    }
}
impl Drop for Inspector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.signal.try_send(());
    }
}
