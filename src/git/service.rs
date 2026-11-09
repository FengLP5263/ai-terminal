use super::{
    GitError, Inspection, MAX_COMMITS, PAGE, Snapshot, command::Runner, commit::Inspector,
    watch::Watcher,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Request {
    directory: PathBuf,
    limit: usize,
    id: u64,
}
type Response = (u64, Result<Option<Snapshot>, GitError>, bool);
#[derive(Default)]
pub(super) struct Changes {
    pub metadata: bool,
    pub paths: HashSet<PathBuf>,
}
pub(super) struct Shared {
    request: Mutex<Option<Request>>,
    response: Mutex<Option<Response>>,
    pub changes: Mutex<Changes>,
    stop: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    manual: AtomicBool,
    pub signal: SyncSender<()>,
}
impl Shared {
    pub fn changed(&self, path: Option<PathBuf>) {
        let mut changes = self.changes.lock().unwrap();
        if let Some(path) = path {
            if changes.paths.len() < 64 {
                changes.paths.insert(path);
            } else {
                changes.metadata = true;
            }
        } else {
            changes.metadata = true;
        }
        drop(changes);
        let _ = self.signal.try_send(());
    }
}

/// One active repository, one coalesced request/result slot, event-driven updates.
pub struct GitClient {
    inspector: Inspector,
    pub inspection: Option<Inspection>,
    shared: Arc<Shared>,
    directory: Option<PathBuf>,
    pub snapshot: Option<Arc<Snapshot>>,
    pub error: Option<GitError>,
    pub loading: bool,
    pub not_repository: bool,
    pub watching: bool,
    pub limit: usize,
    last_refresh: Option<Instant>,
}
impl GitClient {
    pub fn new(ctx: egui::Context) -> Self {
        let inspector = Inspector::new(ctx.clone());
        let (tx, rx) = mpsc::sync_channel(1);
        let shared = Arc::new(Shared {
            request: Mutex::new(None),
            response: Mutex::new(None),
            changes: Mutex::new(Changes::default()),
            stop: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            manual: AtomicBool::new(false),
            signal: tx,
        });
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("git-snapshots".into())
            .spawn(move || {
                let mut watchers: Vec<Watcher> = Vec::new();
                let mut watched: Vec<PathBuf> = Vec::new();
                while rx.recv().is_ok() {
                    if worker.stop.load(Ordering::Acquire) {
                        break;
                    }
                    // Wait for a quiet burst, bounded to one second under sustained edits.
                    // The outer recv blocks indefinitely when idle; this is not polling.
                    let burst = Instant::now();
                    while burst.elapsed() < Duration::from_secs(1) {
                        let wait = Duration::from_millis(250)
                            .min(Duration::from_secs(1).saturating_sub(burst.elapsed()));
                        if rx.recv_timeout(wait).is_err() {
                            break;
                        }
                    }
                    if worker.stop.load(Ordering::Acquire) {
                        break;
                    }
                    let request = worker.request.lock().unwrap().clone();
                    let Some(request) = request else {
                        watchers.clear();
                        watched.clear();
                        continue;
                    };
                    let manual = worker.manual.swap(false, Ordering::AcqRel);
                    let changes = std::mem::take(&mut *worker.changes.lock().unwrap());
                    let runner = Runner::cancellable(
                        worker.stop.clone(),
                        worker.generation.clone(),
                        request.id,
                    );
                    let mut result = runner.and_then(|runner| {
                        if !manual && !changes.metadata && !changes.paths.is_empty() {
                            // Git itself identifies ignored build/cache paths; tracked ignored
                            // files remain visible because check-ignore preserves tracked paths.
                            let input: Vec<u8> = changes
                                .paths
                                .iter()
                                .flat_map(|p| {
                                    p.to_string_lossy()
                                        .as_bytes()
                                        .iter()
                                        .copied()
                                        .chain([0])
                                        .collect::<Vec<_>>()
                                })
                                .collect();
                            if let Ok(ignored) = runner.run(
                                watched.first().unwrap_or(&request.directory),
                                &["check-ignore", "-z", "--stdin"],
                                Some(&input),
                            ) {
                                let count = ignored
                                    .data
                                    .split(|b| *b == 0)
                                    .filter(|p| !p.is_empty())
                                    .count();
                                if ignored.success && count == changes.paths.len() {
                                    return Err(GitError::Cancelled);
                                }
                            }
                        }
                        super::read_with(&runner, &request.directory, request.limit)
                    });
                    if worker.generation.load(Ordering::Acquire) != request.id
                        || matches!(result, Err(GitError::Cancelled))
                    {
                        continue;
                    }
                    let mut roots = Vec::new();
                    if let Ok(Some(snapshot)) = &result {
                        for path in [&snapshot.root, &snapshot.git_dir, &snapshot.common_dir] {
                            if !roots.contains(path) {
                                roots.push(path.clone());
                            }
                        }
                    }
                    if roots != watched {
                        watchers.clear();
                        for (index, path) in roots.iter().enumerate() {
                            if let Ok(watcher) = Watcher::start(path, index != 0, worker.clone()) {
                                watchers.push(watcher);
                            }
                        }
                        watched = roots;
                        if !watchers.is_empty() {
                            // Publish only after watching has started, so edits between the
                            // first discovery query and monitor attachment cannot be lost.
                            result = Runner::cancellable(
                                worker.stop.clone(),
                                worker.generation.clone(),
                                request.id,
                            )
                            .and_then(|runner| {
                                super::read_with(&runner, &request.directory, request.limit)
                            });
                        }
                    }
                    if worker.generation.load(Ordering::Acquire) != request.id {
                        continue;
                    }
                    *worker.response.lock().unwrap() = Some((
                        request.id,
                        result,
                        !watched.is_empty()
                            && watchers.len() == watched.len()
                            && watchers.iter().all(Watcher::is_alive),
                    ));
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                }
            })
            .expect("Git worker thread");
        Self {
            inspector,
            inspection: None,
            shared,
            directory: None,
            snapshot: None,
            error: None,
            loading: false,
            not_repository: false,
            watching: false,
            limit: PAGE,
            last_refresh: None,
        }
    }
    pub fn set_directory(&mut self, directory: Option<&Path>) -> bool {
        if self.directory.as_deref() == directory {
            return false;
        }
        self.directory = directory.map(Path::to_owned);
        self.inspector.clear();
        self.inspection = None;
        self.snapshot = None;
        self.error = None;
        self.not_repository = false;
        self.limit = PAGE;
        self.refresh();
        true
    }
    pub fn refresh(&mut self) {
        self.last_refresh = Some(Instant::now());
        let id = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.loading = self.directory.is_some();
        *self.shared.request.lock().unwrap() = self.directory.clone().map(|directory| Request {
            directory,
            limit: self.limit,
            id,
        });
        self.shared.manual.store(true, Ordering::Release);
        let _ = self.shared.signal.try_send(());
    }
    /// Prompts, focus and opening a panel are fallback opportunities, not change signals.
    /// Healthy filesystem monitoring already keeps the snapshot current.
    pub fn refresh_if_needed(&mut self) -> bool {
        if self.directory.is_none()
            || self.loading
            || (self.watching && self.error.is_none())
            || self
                .last_refresh
                .is_some_and(|last| last.elapsed() < Duration::from_secs(2))
        {
            return false;
        }
        self.refresh();
        true
    }
    pub fn more(&mut self) {
        if self.limit < MAX_COMMITS {
            self.limit = (self.limit + PAGE).min(MAX_COMMITS);
            self.refresh();
        }
    }
    pub fn inspect(&mut self, id: &str) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        if !snapshot.commits.iter().any(|c| c.id == id)
            || self
                .inspection
                .as_ref()
                .is_some_and(|i| i.id == id && i.error.is_none())
        {
            return;
        }
        self.inspector.inspect(&snapshot.root, id);
        self.inspection = Some(Inspection {
            id: id.into(),
            data: None,
            error: None,
            loading: true,
        });
    }
    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        if let Some(inspection) = self.inspector.poll() {
            self.inspection = Some(inspection);
            updated = true;
        }
        let response = self.shared.response.lock().unwrap().take();
        if let Some((id, result, watching)) = response {
            if id != self.shared.generation.load(Ordering::Acquire) {
                return false;
            }
            self.loading = false;
            self.watching = watching;
            match result {
                Ok(snapshot) => {
                    if self.inspection.as_ref().is_some_and(|i| {
                        !snapshot
                            .as_ref()
                            .is_some_and(|s| s.commits.iter().any(|c| c.id == i.id))
                    }) {
                        self.inspector.clear();
                        self.inspection = None;
                    }
                    self.not_repository = snapshot.is_none();
                    self.snapshot = snapshot.map(Arc::new);
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            }
            return true;
        }
        updated
    }
}
impl Drop for GitClient {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        let _ = self.shared.signal.try_send(());
    }
}
