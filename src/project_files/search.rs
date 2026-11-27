use super::{FileError, linked, resolve};
use std::{
    collections::{BinaryHeap, VecDeque},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MAX_RESULTS: usize = 200;
const MAX_PENDING: usize = 16_384;
const MAX_ENTRIES: usize = 200_000;

#[derive(Debug)]
pub struct SearchResults {
    pub paths: Vec<PathBuf>,
    pub partial: bool,
}

pub(super) fn find(
    root: &Path,
    query: &str,
    cancelled: impl Fn() -> bool,
) -> Result<SearchResults, FileError> {
    let root = resolve(root, Path::new(""))?;
    let query = query.replace('\\', "/");
    let query = query.strip_prefix("./").unwrap_or(&query);
    // An existing path prefix lets a targeted search skip unrelated directories.
    let scope = query
        .rsplit_once('/')
        .map(|(prefix, _)| Path::new(prefix))
        .filter(|path| !path.iter().any(|part| part.eq_ignore_ascii_case(".git")))
        .filter(|path| resolve(&root, path).is_ok_and(|full| full.is_dir()))
        .unwrap_or_else(|| Path::new(""));
    let needle = query.to_lowercase();
    let began = Instant::now();
    let mut directories = VecDeque::from([scope.to_owned()]);
    let mut ranked = BinaryHeap::new();
    let mut visited = 0;
    let mut partial = false;

    'scan: while let Some(directory) = directories.pop_front() {
        if cancelled() {
            return Err(FileError::Cancelled);
        }
        if began.elapsed() >= Duration::from_secs(3) {
            partial = true;
            break;
        }
        let entries = match fs::read_dir(root.join(&directory)) {
            Ok(entries) => entries,
            Err(error) if directory == scope => return Err(error.into()),
            Err(_) => {
                partial = true;
                continue;
            }
        };
        let parent = directory
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        for entry in entries {
            if cancelled() {
                return Err(FileError::Cancelled);
            }
            if visited == MAX_ENTRIES || began.elapsed() >= Duration::from_secs(3) {
                partial = true;
                break 'scan;
            }
            visited += 1;
            let Ok(entry) = entry else {
                partial = true;
                continue;
            };
            let name = entry.file_name();
            if name.eq_ignore_ascii_case(".git") {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                partial = true;
                continue;
            };
            if linked(&metadata) {
                continue;
            }
            if metadata.is_dir() {
                if directories.len() < MAX_PENDING {
                    directories.push_back(directory.join(name));
                } else {
                    partial = true;
                }
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let file = name.to_string_lossy().to_lowercase();
            let relative = if parent.is_empty() {
                file.clone()
            } else {
                format!("{parent}/{file}")
            };
            let rank = if file == needle || relative == needle {
                0
            } else if file.starts_with(&needle) {
                1
            } else if file.contains(&needle) {
                2
            } else if relative.contains(&needle) {
                3
            } else {
                continue;
            };
            ranked.push((rank, relative, directory.join(name)));
            if ranked.len() > MAX_RESULTS {
                ranked.pop();
                partial = true;
            }
        }
    }

    Ok(SearchResults {
        paths: ranked
            .into_sorted_vec()
            .into_iter()
            .map(|(_, _, path)| path)
            .collect(),
        partial,
    })
}
