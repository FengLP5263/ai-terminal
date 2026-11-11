//! Lazy, read-only project browsing. All filesystem and syntax work stays off the UI thread.
use crate::git::{FilePatch, PatchKind, PatchLine, PatchSource, syntax};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub const MAX_ENTRIES: usize = 1000;
pub const MAX_DIRECTORIES: usize = 64;
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_LINES: usize = 20_000;
pub const MAX_CHARS: usize = 4000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    Io(String),
    Link,
    OutsideProject,
    TooLarge,
    Binary,
    Encoding,
    Cancelled,
}
impl From<std::io::Error> for FileError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    Link,
    Other,
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: EntryKind,
    /// A positive Git check-ignore result; missing Git/query failures leave it unmarked.
    pub ignored: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub partial: bool,
}
#[derive(Debug)]
pub struct Document {
    pub content: FilePatch,
    pub encoding: &'static str,
    pub bytes: usize,
}
#[derive(Clone, Default)]
pub struct DirectoryState {
    request_id: u64,
    pub data: Option<Arc<Listing>>,
    pub error: Option<FileError>,
    pub loading: bool,
}
#[derive(Clone)]
pub struct FileState {
    pub path: PathBuf,
    pub data: Option<Arc<Document>>,
    pub error: Option<FileError>,
    pub loading: bool,
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}
fn relative(path: &Path) -> bool {
    path.components().all(|c| matches!(c, Component::Normal(_)))
}
fn resolve(root: &Path, path: &Path) -> Result<PathBuf, FileError> {
    if !relative(path) {
        return Err(FileError::OutsideProject);
    }
    let mut full = root.canonicalize()?;
    for component in path.components() {
        full.push(component);
        if linked(&fs::symlink_metadata(&full)?) {
            return Err(FileError::Link);
        }
    }
    Ok(full)
}
pub fn read_directory(
    root: &Path,
    path: &Path,
    cancelled: impl Fn() -> bool,
) -> Result<Listing, FileError> {
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    let full = resolve(root, path)?;
    let began = Instant::now();
    let mut listing = Listing::default();
    for entry in fs::read_dir(&full)? {
        if cancelled() {
            return Err(FileError::Cancelled);
        }
        if listing.entries.len() == MAX_ENTRIES || began.elapsed() > Duration::from_secs(2) {
            listing.partial = true;
            break;
        }
        let entry = entry?;
        // A broken/unreadable child remains visible instead of hiding the entire directory.
        let kind = match fs::symlink_metadata(entry.path()) {
            Ok(m) if linked(&m) => EntryKind::Link,
            Ok(m) if m.is_dir() => EntryKind::Directory,
            Ok(m) if m.is_file() => EntryKind::File,
            _ => EntryKind::Other,
        };
        listing.entries.push(Entry {
            path: path.join(entry.file_name()),
            name: entry.file_name().to_string_lossy().into_owned(),
            kind,
            ignored: false,
        });
    }
    let names = listing
        .entries
        .iter()
        .filter_map(|entry| {
            entry.path.file_name()?.to_str().map(|name| {
                if entry.kind == EntryKind::Directory {
                    format!("{name}/")
                } else {
                    name.to_owned()
                }
            })
        })
        .collect::<Vec<_>>();
    match crate::git::ignored_names(&full, &names, &cancelled) {
        Ok(ignored) => {
            for entry in &mut listing.entries {
                entry.ignored = entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| ignored.contains(name));
            }
        }
        Err(crate::git::GitError::Cancelled) => return Err(FileError::Cancelled),
        Err(_) => {}
    }
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    listing.entries.sort_by_cached_key(|entry| {
        (
            entry.kind != EntryKind::Directory,
            entry.name.to_lowercase(),
            entry.name.clone(),
        )
    });
    Ok(listing)
}
fn decode(bytes: &[u8]) -> Result<(String, &'static str), FileError> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if !(bytes.len() - 2).is_multiple_of(2) {
            return Err(FileError::Encoding);
        }
        let le = bytes[0] == 0xff;
        let units = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                if le {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect::<Vec<_>>();
        let text = String::from_utf16(&units).map_err(|_| FileError::Encoding)?;
        if text.contains('\0') {
            return Err(FileError::Binary);
        }
        Ok((text, if le { "UTF-16 LE" } else { "UTF-16 BE" }))
    } else {
        if bytes.contains(&0) {
            return Err(FileError::Binary);
        }
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        Ok((
            std::str::from_utf8(bytes)
                .map_err(|_| FileError::Encoding)?
                .into(),
            "UTF-8",
        ))
    }
}
pub fn read_file(
    root: &Path,
    path: &Path,
    cancelled: impl Fn() -> bool,
) -> Result<Document, FileError> {
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    let full = resolve(root, path)?;
    let metadata = fs::metadata(&full)?;
    if !metadata.is_file() {
        return Err(FileError::Binary);
    }
    if metadata.len() > MAX_BYTES {
        return Err(FileError::TooLarge);
    }
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    let mut bytes = Vec::new();
    fs::File::open(full)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(FileError::TooLarge);
    }
    let (text, encoding) = decode(&bytes)?;
    let mut content = FilePatch {
        source: PatchSource::WorkingTree,
        path: path.to_string_lossy().into_owned(),
        lines: vec![],
        binary: false,
        partial: false,
        first_parent: false,
        max_columns: 0,
        presentation: Default::default(),
    };
    if !text.is_empty() {
        for (index, raw) in text.split('\n').enumerate() {
            if cancelled() {
                return Err(FileError::Cancelled);
            }
            if index == MAX_LINES {
                content.partial = true;
                break;
            }
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            content.partial |= raw.chars().count() > MAX_CHARS;
            let text = syntax::display_text(&raw.chars().take(MAX_CHARS).collect::<String>());
            content.max_columns = content
                .max_columns
                .max(text.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum());
            content.lines.push(PatchLine {
                kind: PatchKind::Context,
                old: None,
                new: Some(index as u64 + 1),
                text,
            });
        }
    }
    content.presentation = syntax::prepare(&content.path, &content.lines, &cancelled);
    if cancelled() {
        return Err(FileError::Cancelled);
    }
    Ok(Document {
        content,
        encoding,
        bytes: bytes.len(),
    })
}

#[derive(Clone, Copy)]
enum Kind {
    Directory(u64),
    File(u64),
}
struct Request {
    epoch: u64,
    root: PathBuf,
    path: PathBuf,
    kind: Kind,
}
enum ResultData {
    Directory(u64, Result<Arc<Listing>, FileError>),
    File(u64, Result<Arc<Document>, FileError>),
}
struct Response {
    epoch: u64,
    path: PathBuf,
    data: ResultData,
}
#[derive(Default)]
struct Queue {
    requests: VecDeque<Request>,
    responses: VecDeque<Response>,
}
struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    stop: AtomicBool,
    epoch: AtomicU64,
    ticket: AtomicU64,
}

/// One worker and bounded queues/cache; no recursive indexing, timer or file-body persistence.
pub struct FileClient {
    pub root: Option<PathBuf>,
    pub directories: BTreeMap<PathBuf, DirectoryState>,
    pub file: Option<FileState>,
    pub revision: u64,
    pub limit_reached: bool,
    shared: Arc<Shared>,
    next_directory: u64,
}
impl FileClient {
    pub fn new(ctx: egui::Context) -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::default(),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
            ticket: AtomicU64::new(0),
        });
        let state = shared.clone();
        std::thread::Builder::new().name("project-files".into()).spawn(move || loop {
            let request = {
                let mut queue=state.queue.lock().unwrap();
                while queue.requests.is_empty() && !state.stop.load(Ordering::Relaxed) { queue=state.wake.wait(queue).unwrap(); }
                if state.stop.load(Ordering::Relaxed) { break; }
                queue.requests.pop_front().unwrap()
            };
            let cancelled=|| state.stop.load(Ordering::Relaxed) || state.epoch.load(Ordering::Relaxed)!=request.epoch || matches!(request.kind,Kind::File(ticket) if state.ticket.load(Ordering::Relaxed)!=ticket);
            if cancelled() { continue; }
            let data=match request.kind {
                Kind::Directory(ticket)=>ResultData::Directory(ticket,read_directory(&request.root,&request.path,cancelled).map(Arc::new)),
                Kind::File(ticket)=>ResultData::File(ticket,read_file(&request.root,&request.path,cancelled).map(Arc::new)),
            };
            if cancelled() { continue; }
            let mut queue=state.queue.lock().unwrap();
            // At most 64 directory requests and one coalesced selected-file request.
            if queue.responses.len()<128 { queue.responses.push_back(Response {epoch:request.epoch,path:request.path,data}); }
            drop(queue);
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }).expect("project file worker");
        Self {
            root: None,
            directories: BTreeMap::new(),
            file: None,
            revision: 0,
            limit_reached: false,
            next_directory: 0,
            shared,
        }
    }
    pub fn set_root(&mut self, root: Option<&Path>) -> bool {
        if self.root.as_deref() == root {
            return false;
        }
        self.shared.epoch.fetch_add(1, Ordering::Relaxed);
        self.close_file();
        self.root = root.map(Path::to_owned);
        self.directories.clear();
        self.limit_reached = false;
        self.revision += 1;
        let mut queue = self.shared.queue.lock().unwrap();
        queue.requests.clear();
        queue.responses.clear();
        true
    }
    fn request(&self, path: &Path, kind: Kind) {
        if let Some(root) = &self.root {
            let mut queue = self.shared.queue.lock().unwrap();
            if matches!(kind, Kind::File(_)) {
                queue.requests.retain(|r| !matches!(r.kind, Kind::File(_)));
                queue
                    .responses
                    .retain(|r| !matches!(r.data, ResultData::File(..)));
            }
            queue.requests.push_back(Request {
                epoch: self.shared.epoch.load(Ordering::Relaxed),
                root: root.clone(),
                path: path.into(),
                kind,
            });
            self.shared.wake.notify_one();
        }
    }
    pub fn ensure_directory(&mut self, path: &Path) {
        if self.root.is_none() || self.directories.contains_key(path) {
            return;
        }
        if self.directories.len() == MAX_DIRECTORIES {
            self.limit_reached = true;
            return;
        }
        self.next_directory = self.next_directory.wrapping_add(1);
        self.directories.insert(
            path.into(),
            DirectoryState {
                request_id: self.next_directory,
                loading: true,
                ..Default::default()
            },
        );
        self.request(path, Kind::Directory(self.next_directory));
        self.revision += 1;
    }
    pub fn open_file(&mut self, path: &Path) {
        if self.root.is_none() || self.file.as_ref().is_some_and(|f| f.path == path) {
            return;
        }
        let ticket = self.shared.ticket.fetch_add(1, Ordering::Relaxed) + 1;
        self.file = Some(FileState {
            path: path.into(),
            data: None,
            error: None,
            loading: true,
        });
        self.request(path, Kind::File(ticket));
        self.revision += 1;
    }
    pub fn close_file(&mut self) {
        self.shared.ticket.fetch_add(1, Ordering::Relaxed);
        self.file = None;
        self.revision += 1;
    }
    pub fn refresh(&mut self) {
        let directories = self.directories.keys().cloned().collect::<Vec<_>>();
        self.shared.epoch.fetch_add(1, Ordering::Relaxed);
        {
            let mut queue = self.shared.queue.lock().unwrap();
            queue.requests.clear();
            queue.responses.clear();
        }
        self.limit_reached = false;
        for path in directories {
            self.next_directory = self.next_directory.wrapping_add(1);
            if let Some(state) = self.directories.get_mut(&path) {
                state.request_id = self.next_directory;
                state.loading = true;
                state.error = None;
            }
            self.request(&path, Kind::Directory(self.next_directory));
        }
        self.reload_file();
        self.revision += 1;
    }
    pub fn reload_file(&mut self) {
        if let Some(state) = self.file.as_mut() {
            state.loading = true;
            state.error = None;
            let path = state.path.clone();
            let ticket = self.shared.ticket.fetch_add(1, Ordering::Relaxed) + 1;
            self.request(&path, Kind::File(ticket));
            self.revision += 1;
        }
    }
    pub fn collapse(&mut self, path: &Path) {
        self.directories
            .retain(|key, _| key.as_os_str().is_empty() || !key.starts_with(path));
        let mut queue = self.shared.queue.lock().unwrap();
        queue.requests.retain(|r| {
            !matches!(r.kind, Kind::Directory(_))
                || r.path.as_os_str().is_empty()
                || !r.path.starts_with(path)
        });
        queue.responses.retain(|r| {
            !matches!(r.data, ResultData::Directory(..))
                || r.path.as_os_str().is_empty()
                || !r.path.starts_with(path)
        });
        self.limit_reached = false;
        self.revision += 1;
    }
    pub fn poll(&mut self) {
        let responses = std::mem::take(&mut self.shared.queue.lock().unwrap().responses);
        for response in responses {
            if response.epoch != self.shared.epoch.load(Ordering::Relaxed) {
                continue;
            }
            match response.data {
                ResultData::Directory(ticket, result) => {
                    if let Some(state) = self
                        .directories
                        .get_mut(&response.path)
                        .filter(|s| s.request_id == ticket)
                    {
                        state.loading = false;
                        match result {
                            Ok(data) => {
                                state.data = Some(data);
                                state.error = None;
                            }
                            Err(error) => {
                                state.data = None;
                                state.error = Some(error);
                            }
                        }
                    }
                }
                ResultData::File(ticket, result)
                    if self.shared.ticket.load(Ordering::Relaxed) == ticket =>
                {
                    if let Some(state) = self.file.as_mut().filter(|f| f.path == response.path) {
                        state.loading = false;
                        match result {
                            Ok(data) => {
                                state.data = Some(data);
                                state.error = None;
                            }
                            Err(error) => {
                                state.data = None;
                                state.error = Some(error);
                            }
                        }
                    }
                }
                _ => {}
            }
            self.revision += 1;
        }
    }
}
impl Drop for FileClient {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        self.shared.wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_directory_results_cannot_replace_a_reopened_folder() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("folder")).unwrap();
        let mut client = FileClient::new(egui::Context::default());
        client.set_root(Some(root.path()));
        let path = Path::new("folder");
        client.ensure_directory(path);
        let old = client.directories[path].request_id;
        client.collapse(path);
        client.ensure_directory(path);
        assert_ne!(old, client.directories[path].request_id);
        let stale = Listing {
            entries: vec![Entry {
                path: "folder/stale.txt".into(),
                name: "stale.txt".into(),
                kind: EntryKind::File,
                ignored: false,
            }],
            partial: false,
        };
        let epoch = client.shared.epoch.load(Ordering::Relaxed);
        {
            let mut queue = client.shared.queue.lock().unwrap();
            queue.responses.push_back(Response {
                epoch,
                path: path.into(),
                data: ResultData::Directory(old, Ok(Arc::new(stale))),
            });
            queue.responses.push_back(Response {
                epoch,
                path: path.into(),
                data: ResultData::Directory(old, Err(FileError::Io("late failure".into()))),
            });
        }
        client.poll();
        let state = &client.directories[path];
        assert!(state.error.is_none());
        assert!(
            state
                .data
                .as_ref()
                .is_none_or(|d| d.entries.iter().all(|e| e.name != "stale.txt"))
        );
    }
}
