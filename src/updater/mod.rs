mod http;
mod install;
mod package;
mod release;
#[cfg(test)]
mod tests;

pub use install::{Install, Staged};
use semver::Version;
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    time::{Duration, Instant},
};

const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_RETRY: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub enum Source {
    GitHub,
    #[cfg(feature = "update-preview")]
    Preview {
        archive: std::path::PathBuf,
        version: Version,
        apply: bool,
    },
}

impl Source {
    pub fn from_args(_args: &[String], _isolated: bool) -> io::Result<Self> {
        #[cfg(feature = "update-preview")]
        if let Some(pair) = _args
            .windows(2)
            .find(|pair| pair[0] == "--update-preview-archive")
        {
            if !_isolated {
                return Err(io::Error::other(
                    "Update preview requires an isolated --data-dir",
                ));
            }
            let version = _args
                .windows(2)
                .find(|pair| pair[0] == "--update-preview-version")
                .and_then(|pair| Version::parse(&pair[1]).ok())
                .ok_or_else(|| io::Error::other("Update preview requires a package version"))?;
            return Ok(Self::Preview {
                archive: std::path::Path::new(&pair[1]).canonicalize()?,
                version,
                apply: _args.iter().any(|arg| arg == "--apply-update-preview"),
            });
        }
        Ok(Self::GitHub)
    }
}

#[derive(Clone)]
enum Payload {
    Remote(release::Release),
    #[cfg(feature = "update-preview")]
    Local(std::path::PathBuf),
}
#[derive(Clone)]
pub struct Offer {
    pub version: Version,
    notes: release::Notes,
    payload: Payload,
}
impl Offer {
    pub fn notes(&self, locale: crate::i18n::Locale) -> &str {
        match locale {
            crate::i18n::Locale::ZhCn => &self.notes.zh_cn,
            crate::i18n::Locale::En => &self.notes.en,
        }
    }

    fn discover(source: Source, current: &Version) -> io::Result<Option<Self>> {
        match source {
            Source::GitHub => release::check(current).map(|offer| {
                offer.map(|release| Self {
                    version: release.version.clone(),
                    notes: release.notes.clone(),
                    payload: Payload::Remote(release),
                })
            }),
            #[cfg(feature = "update-preview")]
            Source::Preview {
                archive, version, ..
            } => Ok((&version > current).then(|| Self {
                version,
                notes: release::Notes {
                    zh_cn: "右侧文件面板支持按文件名和路径搜索。".into(),
                    en: "Search the Files tab by name or path.".into(),
                },
                payload: Payload::Local(archive),
            })),
        }
    }
    fn prepare(self, install: Install, progress: &AtomicU32) -> io::Result<Staged> {
        let bytes = match &self.payload {
            Payload::Remote(release) => release.download(|fraction| {
                progress.store((fraction * 100.0) as u32, Ordering::Relaxed);
            })?,
            #[cfg(feature = "update-preview")]
            Payload::Local(archive) => {
                use std::io::Read as _;
                let mut bytes = Vec::new();
                std::fs::File::open(archive)?
                    .take(package::MAX_ARCHIVE_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > package::MAX_ARCHIVE_BYTES {
                    return Err(io::Error::other(
                        "The preview package exceeds its size limit",
                    ));
                }
                bytes
            }
        };
        progress.store(100, Ordering::Relaxed);
        install.stage(self.version, bytes)
    }
}

enum Task {
    Check(Receiver<io::Result<Option<Offer>>>),
    Prepare(Receiver<io::Result<Staged>>, Arc<AtomicU32>),
}

pub struct Updater {
    install: Install,
    source: Source,
    pub current: Version,
    pub offer: Option<Offer>,
    pub error: Option<String>,
    pub open: bool,
    next_check: Instant,
    task: Option<Task>,
    staged: Option<Staged>,
    activate: bool,
    restart: SyncSender<Staged>,
}

impl Updater {
    pub fn new(
        install: Install,
        source: Source,
        current: Version,
        activate: bool,
        restart: SyncSender<Staged>,
    ) -> Self {
        Self {
            install,
            source,
            current,
            activate,
            restart,
            offer: None,
            error: None,
            open: false,
            next_check: Instant::now(),
            task: None,
            staged: None,
        }
    }

    /// Files are fully verified before the app begins saving and closing sessions.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        match self.task.as_ref() {
            Some(Task::Check(receiver)) => match receiver.try_recv() {
                Ok(offer) => {
                    match offer {
                        Ok(offer) => self.offer = offer,
                        Err(_) => self.next_check = Instant::now() + CHECK_RETRY,
                    }
                    self.task = None;
                    #[cfg(feature = "update-preview")]
                    if let Source::Preview { apply, .. } = self.source
                        && self.offer.is_some()
                    {
                        self.open = true;
                        if apply {
                            self.start(ctx);
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.task = None;
                    self.next_check = Instant::now() + CHECK_RETRY;
                }
                Err(TryRecvError::Empty) => {}
            },
            Some(Task::Prepare(receiver, _)) => match receiver.try_recv() {
                Ok(Ok(staged)) => {
                    self.task = None;
                    self.staged = Some(staged);
                    return true;
                }
                Ok(Err(error)) => {
                    self.task = None;
                    self.error = Some(error.to_string());
                    self.open = true;
                }
                Err(TryRecvError::Disconnected) => {
                    self.task = None;
                    self.error = Some("Update worker stopped".into());
                }
                Err(TryRecvError::Empty) => {}
            },
            None if Instant::now() >= self.next_check && self.offer.is_none() => {
                self.next_check = Instant::now() + CHECK_INTERVAL;
                let current = self.current.clone();
                let source = self.source.clone();
                if let Ok(receiver) = background(ctx, move || Offer::discover(source, &current)) {
                    self.task = Some(Task::Check(receiver));
                } else {
                    self.next_check = Instant::now() + CHECK_RETRY;
                }
            }
            None => {}
        }
        if self.task.is_none() && self.offer.is_none() {
            ctx.request_repaint_after(self.next_check.saturating_duration_since(Instant::now()));
        }
        false
    }

    pub fn start(&mut self, ctx: &egui::Context) {
        let Some(offer) = self.offer.clone().filter(|_| self.task.is_none()) else {
            return;
        };
        let install = self.install.clone();
        let progress = Arc::new(AtomicU32::new(0));
        let report = progress.clone();
        self.error = None;
        match background(ctx, move || offer.prepare(install, &report)) {
            Ok(receiver) => self.task = Some(Task::Prepare(receiver, progress)),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub fn progress(&self) -> Option<u32> {
        match &self.task {
            Some(Task::Prepare(_, progress)) => Some(progress.load(Ordering::Relaxed)),
            _ => None,
        }
    }

    pub fn complete_shutdown(&mut self) {
        if let Some(staged) = self.staged.take() {
            let _ = self.restart.try_send(staged);
        }
    }

    pub fn cancel_shutdown(&mut self) {
        self.staged = None;
    }

    pub fn first_frame(&mut self, ctx: &egui::Context, frame: u64) -> io::Result<()> {
        if self.activate && frame < 2 {
            ctx.request_repaint();
            return Ok(());
        }
        if std::mem::take(&mut self.activate) {
            self.install.activate(&self.current)?;
        }
        Ok(())
    }
}

fn background<T: Send + 'static>(
    ctx: &egui::Context,
    work: impl FnOnce() -> T + Send + 'static,
) -> io::Result<Receiver<T>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("app-update".into())
        .spawn(move || {
            let _ = sender.send(work());
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        })?;
    Ok(receiver)
}
