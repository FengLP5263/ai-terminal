use super::package;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

const ORIGIN: &str = "--update-origin";
const ACTIVATE: &str = "--activate-update";
const DIRECT: &str = "--update-direct";

#[derive(Clone)]
pub struct Install {
    origin: PathBuf,
    running: PathBuf,
    directory: PathBuf,
    config: PathBuf,
    local: bool,
}

#[derive(Clone, Deserialize, Serialize)]
struct Current {
    slot: String,
    version: String,
}

pub struct Staged {
    install: Install,
    version: Version,
    directory: tempfile::TempDir,
}

impl Install {
    pub fn resolve(config: &Path, args: &[String], local: bool) -> io::Result<Self> {
        let running = std::env::current_exe()?.canonicalize()?;
        let config = config.canonicalize()?;
        let origin_arg = args.windows(2).find(|pair| pair[0] == ORIGIN);
        let origin = match origin_arg {
            Some(pair) => Path::new(&pair[1]).canonicalize()?,
            None => running.clone(),
        };
        let identity = package::digest(origin.to_string_lossy().to_lowercase().as_bytes())?;
        let directory = config.join("updates").join(&identity[..32]);
        if origin_arg.is_some() {
            let parent = running
                .parent()
                .ok_or_else(|| io::Error::other("Missing update path"))?;
            if parent.parent() != Some(directory.as_path())
                || !parent
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(valid_slot)
                || !origin.is_file()
            {
                return Err(io::Error::other("Invalid update origin"));
            }
        }
        Ok(Self {
            origin,
            running,
            directory,
            config,
            local,
        })
    }

    fn pointer(&self) -> PathBuf {
        self.directory.join("current.json")
    }

    fn read_current(&self) -> Option<Current> {
        let file = fs::File::open(self.pointer()).ok()?;
        let mut bytes = Vec::new();
        file.take(4097).read_to_end(&mut bytes).ok()?;
        if bytes.len() > 4096 {
            return None;
        }
        let current: Current = serde_json::from_slice(&bytes).ok()?;
        valid_slot(&current.slot).then_some(current)
    }

    pub fn forward(&self, args: &[String], version: &Version) -> bool {
        if self.running != self.origin || args.iter().any(|arg| arg == DIRECT) {
            return false;
        }
        let Some(current) = self.read_current() else {
            return false;
        };
        let Ok(newer) = Version::parse(&current.version) else {
            return false;
        };
        if &newer <= version {
            return false;
        }
        let slot = self.directory.join(&current.slot);
        if package::verify_runtime(&slot, &newer, self.local).is_err() {
            return false;
        }
        Command::new(slot.join(package::RUNTIME[0]))
            .args(&args[1..])
            .arg(ORIGIN)
            .arg(&self.origin)
            .spawn()
            .is_ok()
    }

    pub fn is_activation(args: &[String]) -> bool {
        args.iter().any(|arg| arg == ACTIVATE)
    }

    /// Called from the first usable GUI frame of the candidate.
    pub fn activate(&self, version: &Version) -> io::Result<()> {
        let slot = self
            .running
            .parent()
            .filter(|dir| dir.parent() == Some(&self.directory))
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .filter(|name| valid_slot(name))
            .ok_or_else(|| io::Error::other("Invalid staged version"))?;
        let new = Current {
            slot: slot.into(),
            version: version.to_string(),
        };
        package::verify_runtime(&self.directory.join(slot), version, self.local)?;
        let mut pointer = tempfile::NamedTempFile::new_in(&self.directory)?;
        serde_json::to_writer(&mut pointer, &new)?;
        pointer.flush()?;
        pointer.as_file().sync_all()?;
        pointer
            .persist(self.pointer())
            .map_err(|error| error.error)?;
        Ok(())
    }

    pub fn stage(&self, version: Version, bytes: Vec<u8>) -> io::Result<Staged> {
        fs::create_dir_all(&self.directory)?;
        let directory = tempfile::Builder::new()
            .prefix("version-")
            .tempdir_in(&self.directory)?;
        package::extract(bytes, &version, directory.path(), self.local)?;
        Ok(Staged {
            install: self.clone(),
            version,
            directory,
        })
    }

    fn launch(&self, exe: &Path) -> Command {
        let mut command = Command::new(exe);
        command.arg("--data-dir").arg(&self.config);
        command
    }

    fn reopen(&self) -> io::Result<Child> {
        let mut command = self.launch(&self.running);
        if self.running == self.origin {
            command.arg(DIRECT);
        } else {
            command.arg(ORIGIN).arg(&self.origin);
        }
        command.spawn()
    }
}

impl Staged {
    pub fn restart(self) -> io::Result<()> {
        let Self {
            install,
            version,
            directory,
        } = self;
        let slot = directory
            .path()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let exe = directory.path().join(package::RUNTIME[0]);
        let mut candidate = match install
            .launch(&exe)
            .arg(ORIGIN)
            .arg(&install.origin)
            .arg(ACTIVATE)
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return install.reopen().map(|_| ()),
        };
        let until = Instant::now() + Duration::from_secs(25);
        loop {
            if install.read_current().is_some_and(|current| {
                current.slot == slot && current.version == version.to_string()
            }) {
                let _ = directory.keep();
                return Ok(());
            }
            if let Ok(Some(status)) = candidate.try_wait() {
                let committed = install
                    .read_current()
                    .is_some_and(|current| current.slot == slot);
                if committed {
                    let _ = directory.keep();
                } else if !status.success() {
                    install.reopen()?;
                }
                return Ok(());
            }
            if Instant::now() >= until {
                // A slow candidate may still be starting. Its files must remain available.
                let _ = directory.keep();
                return Ok(());
            }
            thread::sleep(Duration::from_millis(80));
        }
    }
}

fn valid_slot(name: &str) -> bool {
    name.strip_prefix("version-").is_some_and(|suffix| {
        (6..=32).contains(&suffix.len()) && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
    })
}
