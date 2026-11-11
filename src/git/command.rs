use super::GitError;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const CAP: usize = 2 * 1024 * 1024;
pub(super) struct Output {
    pub success: bool,
    pub data: Vec<u8>,
    pub error: Vec<u8>,
}
#[derive(Clone)]
pub(super) struct Runner {
    git: PathBuf,
    cancel: Option<(Arc<AtomicBool>, Arc<AtomicU64>, u64)>,
    filter_overrides: Vec<String>,
}
impl Runner {
    pub fn new() -> Result<Self, GitError> {
        Ok(Self {
            git: crate::discovery::CliSearch::current()
                .git()
                .ok_or(GitError::Missing)?,
            cancel: None,
            filter_overrides: Vec::new(),
        })
    }
    /// Diff/status may run clean/process filters even with --no-textconv.
    /// Disable configured drivers for this process without changing any config file.
    pub fn without_filters(&self, dir: &Path) -> Result<Self, GitError> {
        let output = self.run(
            dir,
            &[
                "config",
                "--null",
                "--name-only",
                "--get-regexp",
                r"^filter\..*\.(clean|smudge|process|required)$",
            ],
            None,
        )?;
        if !output.success && !output.error.is_empty() {
            return Err(GitError::Command(
                "Unable to inspect Git content-filter configuration.".into(),
            ));
        }
        let mut drivers = std::collections::BTreeSet::new();
        for key in output.data.split(|b| *b == 0).filter(|k| !k.is_empty()) {
            let key = String::from_utf8_lossy(key);
            if let Some((driver, _)) = key.rsplit_once('.') {
                drivers.insert(driver.to_owned());
            }
        }
        let mut runner = self.clone();
        for driver in drivers {
            for suffix in ["clean=", "smudge=", "process=", "required=false"] {
                runner.filter_overrides.push(format!("{driver}.{suffix}"));
            }
        }
        if runner
            .filter_overrides
            .iter()
            .map(|s| s.encode_utf16().count() + 4)
            .sum::<usize>()
            > 8192
        {
            return Err(GitError::Command(
                "Git content-filter configuration exceeds the query limit.".into(),
            ));
        }
        Ok(runner)
    }
    pub fn filters_disabled(&self) -> bool {
        !self.filter_overrides.is_empty()
    }
    pub fn cancellable(
        stop: Arc<AtomicBool>,
        generation: Arc<AtomicU64>,
        id: u64,
    ) -> Result<Self, GitError> {
        let mut r = Self::new()?;
        r.cancel = Some((stop, generation, id));
        Ok(r)
    }
    pub fn checked(&self, dir: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
        let out = self.run(dir, args, None)?;
        if out.success {
            Ok(out.data)
        } else {
            Err(GitError::Command(
                String::from_utf8_lossy(&out.error)
                    .chars()
                    .take(700)
                    .collect(),
            ))
        }
    }
    pub(super) fn cancelled(&self) -> bool {
        self.cancel.as_ref().is_some_and(|(stop, g, id)| {
            stop.load(Ordering::Acquire) || g.load(Ordering::Acquire) != *id
        })
    }
    pub fn run(&self, dir: &Path, args: &[&str], input: Option<&[u8]>) -> Result<Output, GitError> {
        self.run_with_cancel(dir, args, input, || false)
    }
    pub fn run_with_cancel(
        &self,
        dir: &Path,
        args: &[&str],
        input: Option<&[u8]>,
        cancelled: impl Fn() -> bool,
    ) -> Result<Output, GitError> {
        if self.cancelled() || cancelled() {
            return Err(GitError::Cancelled);
        }
        let mut cmd = Command::new(&self.git);
        cmd.args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.pager=cat",
            "-c",
            "core.quotepath=false",
            "-c",
            "log.showSignature=false",
            "-c",
            "core.hooksPath=",
        ]);
        for value in &self.filter_overrides {
            cmd.arg("-c").arg(value);
        }
        cmd.args(args)
            .current_dir(dir)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_ALLOW_PROTOCOL", "")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C");
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_EXTERNAL_DIFF",
            "GIT_DIFF_OPTS",
            "GIT_CONFIG_PARAMETERS",
            "GIT_CONFIG_COUNT",
            "GIT_LITERAL_PATHSPECS",
            "GIT_GLOB_PATHSPECS",
            "GIT_NOGLOB_PATHSPECS",
            "GIT_ICASE_PATHSPECS",
        ] {
            cmd.env_remove(key);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd.spawn().map_err(|e| GitError::Command(e.to_string()))?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let overflow = Arc::new(AtomicBool::new(false));
        let bounded = |mut pipe: Box<dyn Read + Send>, flag: Arc<AtomicBool>| {
            std::thread::spawn(move || {
                let mut out = Vec::new();
                let mut buf = [0; 8192];
                loop {
                    match pipe.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if out.len() + n > CAP {
                                flag.store(true, Ordering::Release);
                                break;
                            }
                            out.extend_from_slice(&buf[..n]);
                        }
                    }
                }
                out
            })
        };
        let a = bounded(Box::new(stdout), overflow.clone());
        let b = bounded(Box::new(stderr), overflow.clone());
        let writer = input.and_then(|input| {
            child.stdin.take().map(|mut pipe| {
                let bytes = input.to_vec();
                std::thread::spawn(move || {
                    let _ = pipe.write_all(&bytes);
                })
            })
        });
        let began = Instant::now();
        let mut error = None;
        let status = loop {
            if overflow.load(Ordering::Acquire) {
                error = Some(GitError::Command(
                    "Git result exceeds the 2 MiB limit.".into(),
                ));
            }
            if began.elapsed() > Duration::from_secs(6) {
                error = Some(GitError::Command("Git query timed out.".into()));
            }
            if self.cancelled() || cancelled() {
                error = Some(GitError::Cancelled);
            }
            if error.is_some() {
                let _ = child.kill();
                break child.wait();
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(15)),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(e);
                }
            }
        };
        let data = a.join().unwrap_or_default();
        let stderr = b.join().unwrap_or_default();
        if let Some(writer) = writer {
            let _ = writer.join();
        }
        if let Some(e) = error {
            return Err(e);
        }
        if overflow.load(Ordering::Acquire) {
            return Err(GitError::Command(
                "Git result exceeds the 2 MiB limit.".into(),
            ));
        }
        Ok(Output {
            success: status
                .map_err(|e| GitError::Command(e.to_string()))?
                .success(),
            data,
            error: stderr,
        })
    }
}
