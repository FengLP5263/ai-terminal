//! Bounded, on-demand file diffs, shared by immutable commits and working-tree reads.
use super::{GitError, command::Runner};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
};

pub(super) const MAX_LINES: usize = 20_000;
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum PatchSource {
    Commit(String),
    WorkingTree,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PatchKind {
    Context,
    Added,
    Deleted,
    Hunk,
    Note,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchLine {
    pub kind: PatchKind,
    pub old: Option<u64>,
    pub new: Option<u64>,
    pub text: String,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct FilePatch {
    pub source: PatchSource,
    pub path: String,
    pub lines: Vec<PatchLine>,
    pub binary: bool,
    pub partial: bool,
    pub first_parent: bool,
    pub max_columns: usize,
    #[serde(skip)]
    pub presentation: super::syntax::Presentation,
}
#[derive(Clone)]
pub struct PatchResult {
    pub source: PatchSource,
    pub path: String,
    pub data: Option<Arc<FilePatch>>,
    pub error: Option<GitError>,
    pub loading: bool,
}
pub fn read_patch(directory: &Path, id: &str, path: &str) -> Result<FilePatch, GitError> {
    read_with(&Runner::new()?, directory, id, path)
}
fn read_with(r: &Runner, dir: &Path, id: &str, path: &str) -> Result<FilePatch, GitError> {
    if !super::commit::valid_id(id) || path.is_empty() || path.contains('\0') || path.len() > 16_384
    {
        return Err(GitError::Command("Invalid committed-file request".into()));
    }
    let parents = r.checked(
        dir,
        &[
            "show",
            "--no-show-signature",
            "--no-notes",
            "--no-patch",
            "--format=%P",
            id,
            "--",
        ],
    )?;
    let parents: Vec<_> = super::text(&parents)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    if parents.iter().any(|p| !super::commit::valid_id(p)) {
        return Err(GitError::Command("Invalid commit parent".into()));
    }
    let mut args = vec![
        "diff-tree",
        "--root",
        "--no-commit-id",
        "-r",
        "-p",
        "--no-renames",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--ignore-submodules=all",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        // Include unchanged content to both file boundaries within the display budget.
        "--unified=20000",
    ];
    if let Some(parent) = parents.first() {
        args.push(parent);
    }
    let literal = format!(":(literal){path}");
    args.extend([id, "--", &literal]);
    let bytes = r.checked(dir, &args)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| GitError::InvalidUtf8)?;
    let mut patch = parse(PatchSource::Commit(id.into()), path, &bytes)?;
    patch.first_parent = parents.len() > 1;
    if !patch.lines.iter().any(|l| l.kind == PatchKind::Hunk) && !patch.binary && !bytes.is_empty()
    {
        // Mode-only/empty-file changes have no text hunk. Still show this version's
        // complete contents rather than presenting metadata as the file body.
        let version = if text.lines().any(|l| l.starts_with("deleted file mode ")) {
            parents.first().map(String::as_str).unwrap_or(id)
        } else {
            id
        };
        let object = format!("{version}:{path}");
        let body = r.checked(dir, &["cat-file", "blob", &object])?;
        patch.binary = body.contains(&0);
        let body_text = if patch.binary {
            ""
        } else {
            std::str::from_utf8(&body).map_err(|_| GitError::InvalidUtf8)?
        };
        for (index, line) in
            body_text
                .lines()
                .enumerate()
                .take(if patch.binary { 0 } else { MAX_LINES + 1 })
        {
            if patch.lines.len() == MAX_LINES {
                patch.partial = true;
                break;
            }
            patch.lines.push(PatchLine {
                kind: PatchKind::Context,
                old: Some(index as u64 + 1),
                new: Some(index as u64 + 1),
                text: line.into(),
            });
        }
    }
    finish(r, patch)
}

pub(super) fn parse(source: PatchSource, path: &str, bytes: &[u8]) -> Result<FilePatch, GitError> {
    let text = std::str::from_utf8(bytes).map_err(|_| GitError::InvalidUtf8)?;
    let mut patch = FilePatch {
        source,
        path: path.into(),
        lines: vec![],
        binary: false,
        partial: false,
        first_parent: false,
        max_columns: 0,
        presentation: Default::default(),
    };
    let (mut old, mut new) = (0, 0);
    let mut in_hunk = false;
    for line in text.lines() {
        if patch.lines.len() == MAX_LINES {
            patch.partial = true;
            break;
        }
        if line.starts_with("@@ ") {
            let mut ranges = line.split_whitespace().skip(1);
            let number = |s: &str| {
                s.get(1..)
                    .and_then(|s| s.split(',').next())
                    .and_then(|n| n.parse::<u64>().ok())
            };
            (old, new) = (
                ranges
                    .next()
                    .and_then(number)
                    .ok_or_else(|| GitError::Command("Invalid diff hunk".into()))?,
                ranges
                    .next()
                    .and_then(number)
                    .ok_or_else(|| GitError::Command("Invalid diff hunk".into()))?,
            );
            in_hunk = true;
            patch.lines.push(PatchLine {
                kind: PatchKind::Hunk,
                old: None,
                new: None,
                text: line.into(),
            });
        } else if line.starts_with("Binary files ") || line == "GIT binary patch" {
            patch.binary = true;
        } else if in_hunk {
            let (kind, before, after) = match line.as_bytes().first() {
                Some(b'+') => {
                    let n = new;
                    new = new.saturating_add(1);
                    (PatchKind::Added, None, Some(n))
                }
                Some(b'-') => {
                    let n = old;
                    old = old.saturating_add(1);
                    (PatchKind::Deleted, Some(n), None)
                }
                Some(b' ') => {
                    let (o, n) = (old, new);
                    old = old.saturating_add(1);
                    new = new.saturating_add(1);
                    (PatchKind::Context, Some(o), Some(n))
                }
                _ => (PatchKind::Note, None, None),
            };
            let text = if kind == PatchKind::Note {
                line
            } else {
                &line[1..]
            };
            patch.lines.push(PatchLine {
                kind,
                old: before,
                new: after,
                text: text.into(),
            });
        } else if !["diff --git ", "index ", "--- ", "+++ "]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            // Keep mode, symlink, empty-file and submodule metadata visible.
            patch.lines.push(PatchLine {
                kind: PatchKind::Note,
                old: None,
                new: None,
                text: line.into(),
            });
        }
    }
    Ok(patch)
}

pub(super) fn finish(r: &Runner, mut patch: FilePatch) -> Result<FilePatch, GitError> {
    for line in &mut patch.lines {
        // The BOM identifies UTF-8 encoding; it is not a character in the first code line.
        if super::syntax::is_file_line(line) && (line.old == Some(1) || line.new == Some(1)) {
            line.text = line
                .text
                .strip_prefix('\u{feff}')
                .unwrap_or(&line.text)
                .into();
        }
        let mut chars = line.text.chars();
        let mut text: String = chars.by_ref().take(4000).collect();
        if chars.next().is_some() {
            text.push('…');
            patch.partial = true;
        }
        patch.max_columns = patch.max_columns.max(
            text.chars()
                .map(|c| {
                    if c == '\t' {
                        4
                    } else if c.is_ascii() {
                        1
                    } else {
                        2
                    }
                })
                .sum(),
        );
        line.text = text;
        if super::syntax::is_file_line(line) {
            line.text = super::syntax::display_text(&line.text);
        }
    }
    patch.presentation = super::syntax::prepare(&patch.path, &patch.lines, || r.cancelled());
    if r.cancelled() {
        return Err(GitError::Cancelled);
    }
    Ok(patch)
}

struct Request {
    root: PathBuf,
    source: PatchSource,
    path: String,
    generation: u64,
}
type Response = (u64, PatchSource, String, Result<FilePatch, GitError>);
pub(super) struct Reader {
    request: Arc<Mutex<Option<Request>>>,
    response: Arc<Mutex<Option<Response>>>,
    stop: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    signal: SyncSender<()>,
}
impl Reader {
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
            .name("git-file-diff".into())
            .spawn(move || {
                while rx.recv().is_ok() && !stop.load(Ordering::Acquire) {
                    let Some(q) = request.lock().unwrap().take() else {
                        continue;
                    };
                    let result =
                        Runner::cancellable(stop.clone(), generation.clone(), q.generation)
                            .and_then(|r| match &q.source {
                                PatchSource::Commit(id) => read_with(&r, &q.root, id, &q.path),
                                PatchSource::WorkingTree => {
                                    super::worktree::read_with(&r, &q.root, &q.path)
                                }
                            });
                    if generation.load(Ordering::Acquire) == q.generation
                        && !stop.load(Ordering::Acquire)
                        && !matches!(result, Err(GitError::Cancelled))
                    {
                        *response.lock().unwrap() = Some((q.generation, q.source, q.path, result));
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                }
            })
            .expect("Git file diff worker");
        this
    }
    pub fn clear(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.request.lock().unwrap() = None;
        *self.response.lock().unwrap() = None;
    }
    pub fn read(&self, root: &Path, source: PatchSource, path: &str) {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        *self.request.lock().unwrap() = Some(Request {
            root: root.into(),
            source,
            path: path.into(),
            generation,
        });
        let _ = self.signal.try_send(());
    }
    pub fn poll(&self) -> Option<PatchResult> {
        let (generation, source, path, result) = self.response.lock().unwrap().take()?;
        if generation != self.generation.load(Ordering::Acquire) {
            return None;
        }
        Some(match result {
            Ok(data) => PatchResult {
                source,
                path,
                data: Some(Arc::new(data)),
                error: None,
                loading: false,
            },
            Err(error) => PatchResult {
                source,
                path,
                data: None,
                error: Some(error),
                loading: false,
            },
        })
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.signal.try_send(());
    }
}
