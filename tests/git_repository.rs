#![cfg(windows)]
use ai_terminal::{
    discovery::CliSearch,
    git::{self, GitClient, GitError, Snapshot},
};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

struct Repo {
    root: tempfile::TempDir,
    git: PathBuf,
}
impl Repo {
    fn new() -> Self {
        let repo = Self {
            root: tempfile::tempdir().unwrap(),
            git: CliSearch::current()
                .git()
                .expect("Git for Windows is required for repository regressions"),
        };
        repo.run(&["init", "-b", "main"]);
        repo
    }
    fn dir(&self) -> &Path {
        self.root.path()
    }
    fn run(&self, args: &[&str]) -> String {
        use std::os::windows::process::CommandExt;
        let result = Command::new(&self.git)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
                "-c",
                "core.hooksPath=",
            ])
            .args(args)
            .current_dir(self.dir())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "NUL")
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8_lossy(&result.stdout).trim().into()
    }
    fn write(&self, path: &str, text: impl AsRef<[u8]>) {
        let p = self.dir().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    fn commit(&self, message: &str) {
        self.run(&["add", "--all"]);
        self.run(&["commit", "-m", message]);
    }
    fn snapshot(&self) -> Snapshot {
        git::read(self.dir(), 50).unwrap().unwrap()
    }
}

#[test]
fn net_totals_include_untracked_text_without_double_counting_staged_edits() {
    let repo = Repo::new();
    repo.write("文件 space &.txt", "one\ntwo\nthree\n");
    repo.write(".gitignore", "ignored/\n");
    repo.commit("first");
    repo.write("文件 space &.txt", "one\nreplaced\nthree\nadded\n");
    repo.run(&["add", "--all"]);
    repo.write("文件 space &.txt", "one\nfinal\n");
    repo.write("new.txt", "new\nline");
    repo.write("image.bin", [0, 255, 1]);
    repo.write("ignored/cache.txt", "ignore me\n");
    let sub = repo.dir().join("nested");
    std::fs::create_dir(&sub).unwrap();
    let snapshot = git::read(&sub, 50).unwrap().unwrap();
    assert_eq!(snapshot.branch, "main");
    assert_eq!((snapshot.added, snapshot.deleted), (3, 2));
    assert_eq!(snapshot.total_files, 3);
    let tracked = snapshot
        .files
        .iter()
        .find(|f| f.path == "文件 space &.txt")
        .unwrap();
    assert_eq!((tracked.index, tracked.worktree), ('M', 'M'));
    assert_eq!((tracked.added, tracked.deleted), (1, 2));
    assert!(
        snapshot
            .files
            .iter()
            .find(|f| f.path == "image.bin")
            .unwrap()
            .binary
    );
    assert_eq!(snapshot.commits.len(), 1);
}

#[test]
fn unborn_detached_and_branch_history_are_distinct_and_paged() {
    let repo = Repo::new();
    repo.write("new.txt", "initial\n");
    repo.run(&["add", "new.txt"]);
    repo.write("new.txt", "final\nextra\n");
    let unborn = repo.snapshot();
    assert!(unborn.unborn);
    assert_eq!(unborn.branch, "main");
    assert_eq!(unborn.added, 2);
    assert!(unborn.commits.is_empty());
    repo.commit("base");
    let base = repo.run(&["rev-parse", "HEAD"]);
    repo.run(&["switch", "-c", "topic"]);
    repo.write("topic.txt", "topic\n");
    repo.commit("only topic");
    repo.run(&["switch", "main"]);
    repo.write("main.txt", "main\n");
    repo.commit("only main");
    let main = repo.snapshot();
    assert!(!main.commits.iter().any(|c| c.subject == "only topic"));
    let one = git::read(repo.dir(), 1).unwrap().unwrap();
    assert!(one.more);
    assert_eq!(one.commits.len(), 1);
    repo.run(&["merge", "--no-ff", "topic", "-m", "merge topic"]);
    let merged = repo.snapshot();
    assert_eq!(merged.commits[0].parents.len(), 2);
    assert!(merged.commits.iter().any(|c| c.subject == "only topic"));
    repo.run(&["switch", "--detach", &base]);
    let detached = repo.snapshot();
    assert!(detached.detached);
    assert!(!detached.unborn);
    assert!(base.starts_with(&detached.branch));
}

#[test]
fn git_file_metadata_and_non_repository_have_explicit_results() {
    let repo = Repo::new();
    repo.write("file.txt", "line\n");
    repo.commit("initial");
    let external = tempfile::tempdir().unwrap();
    let metadata = external.path().join("metadata");
    repo.run(&["init", "--separate-git-dir", metadata.to_str().unwrap()]);
    assert!(repo.dir().join(".git").is_file());
    assert_eq!(
        repo.snapshot().git_dir.canonicalize().unwrap(),
        metadata.canonicalize().unwrap()
    );
    let not_repo = tempfile::tempdir().unwrap();
    assert!(git::read(not_repo.path(), 50).unwrap().is_none());
}

#[test]
fn large_untracked_files_and_binary_files_do_not_produce_invented_line_counts() {
    let repo = Repo::new();
    repo.write("base.txt", "line\n");
    repo.commit("initial");
    repo.write("large.txt", vec![b'a'; 1024 * 1024 + 1]);
    repo.write("binary.bin", [0, 1, 2]);
    let snapshot = repo.snapshot();
    assert_eq!(snapshot.added, 0);
    assert!(snapshot.partial);
    assert!(snapshot.files.iter().any(|f| f.omitted));
}

fn wait(client: &mut GitClient, condition: impl Fn(&Snapshot) -> bool) {
    let until = Instant::now() + Duration::from_secs(15);
    while Instant::now() < until {
        client.poll();
        if let Some(snapshot) = &client.snapshot
            && condition(snapshot)
        {
            return;
        }
        if let Some(error) = &client.error {
            panic!("Git worker failed: {error:?}");
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    panic!("Git worker did not produce the expected snapshot");
}

#[test]
fn notifications_refresh_active_repo_ignore_build_files_and_reject_old_directory_results() {
    let first = Repo::new();
    first.write("one.txt", "first\n");
    first.write(".gitignore", "ignored/\n");
    first.commit("first repo");
    let second = Repo::new();
    second.write("two.txt", "second\n");
    second.commit("second repo");
    let mut client = GitClient::new(egui::Context::default());
    client.set_directory(Some(second.dir()));
    client.set_directory(Some(first.dir()));
    wait(&mut client, |s| s.commits[0].subject == "first repo");
    assert!(client.watching);
    std::thread::sleep(Duration::from_millis(700));
    client.poll();
    let before = client.snapshot.as_ref().unwrap().clone();
    first.write("ignored/cache.txt", "ignored\n");
    std::thread::sleep(Duration::from_millis(900));
    client.poll();
    assert!(
        Arc::ptr_eq(&before, client.snapshot.as_ref().unwrap()),
        "ignored changes must not rescan the repository snapshot"
    );
    first.write("one.txt", "first\nnew line\n");
    wait(&mut client, |s| s.added == 1);
    first.run(&["switch", "-c", "active-branch"]);
    wait(&mut client, |s| s.branch == "active-branch");
    client.set_directory(Some(second.dir()));
    wait(&mut client, |s| s.commits[0].subject == "second repo");
    assert_eq!(client.snapshot.as_ref().unwrap().added, 0);
}

#[test]
fn oversized_history_fails_boundedly_without_blocking_or_changing_repository() {
    let repo = Repo::new();
    repo.write("one.txt", "one\n");
    repo.run(&["add", "--all"]);
    let message = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(message.path(), vec![b'x'; 2 * 1024 * 1024 + 1000]).unwrap();
    repo.run(&["commit", "-F", message.path().to_str().unwrap()]);
    let until = Instant::now();
    let result = git::read(repo.dir(), 50);
    assert!(until.elapsed() < Duration::from_secs(12));
    assert!(matches!(result,Err(GitError::Command(ref error)) if error.contains("2 MiB")));
    assert!(repo.run(&["status", "--porcelain"]).is_empty());
}

#[test]
fn file_display_cap_is_partial_but_tracked_totals_stay_complete() {
    let repo = Repo::new();
    for i in 0..501 {
        repo.write(&format!("files/{i:03}.txt"), "base\n");
    }
    repo.commit("many files");
    for i in 0..501 {
        repo.write(&format!("files/{i:03}.txt"), "base\nadded\n");
    }
    let snapshot = repo.snapshot();
    assert_eq!(snapshot.files.len(), 500);
    assert_eq!(snapshot.total_files, 501);
    assert_eq!((snapshot.added, snapshot.deleted), (501, 0));
    assert!(snapshot.partial);
}

#[test]
fn reading_status_and_diff_never_executes_configured_content_filters() {
    let repo = Repo::new();
    repo.write("one.txt", "base\n");
    repo.write(".gitattributes", "*.txt filter=fixture\n");
    repo.commit("filter fixture");
    repo.run(&[
        "config",
        "filter.fixture.clean",
        "echo invoked > filter-ran; cat",
    ]);
    repo.run(&[
        "config",
        "filter.fixture.process",
        "echo invoked > filter-ran; exit 1",
    ]);
    repo.run(&["config", "filter.fixture.required", "true"]);
    repo.write("one.txt", "changed\nsecond\n");
    let snapshot = repo.snapshot();
    assert!(!repo.dir().join("filter-ran").exists());
    assert!(snapshot.partial);
    assert!(
        snapshot
            .files
            .iter()
            .find(|f| f.path == "one.txt")
            .unwrap()
            .omitted
    );
    assert_eq!((snapshot.added, snapshot.deleted), (0, 0));
    assert_eq!(repo.run(&["config", "filter.fixture.required"]), "true");
    repo.run(&["config", "--unset", "filter.fixture.process"]);
    assert!(repo.snapshot().partial);
    assert!(
        !repo.dir().join("filter-ran").exists(),
        "A standalone clean filter must also remain disabled"
    );
}
