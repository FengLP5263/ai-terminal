use ai_terminal::project_files::{self as files, EntryKind, FileClient, FileError};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn wait(client: &mut FileClient, ready: impl Fn(&FileClient) -> bool) {
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        client.poll();
        if ready(client) {
            return;
        }
        assert!(Instant::now() < until, "file worker did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn lazy_directory_listing_orders_folders_and_preserves_unicode_and_hidden_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/hidden.rs"), "nested").unwrap();
    for name in ["z.txt", ".env.example", "你好 ❤️.md", "Alpha.json"] {
        fs::write(root.join(name), "fixture").unwrap();
    }
    let listing = files::read_directory(root, Path::new(""), || false).unwrap();
    assert_eq!(listing.entries.len(), 5);
    assert_eq!(listing.entries[0].name, "src");
    assert_eq!(listing.entries[0].kind, EntryKind::Directory);
    assert!(listing.entries.iter().any(|e| e.name == "你好 ❤️.md"));
    assert!(
        listing
            .entries
            .iter()
            .all(|e| e.path.components().count() == 1)
    );
    assert!(!listing.partial);
    assert_eq!(
        files::read_directory(root, Path::new("src"), || false)
            .unwrap()
            .entries[0]
            .path,
        Path::new("src/hidden.rs")
    );
}
#[test]
fn utf8_bom_and_utf16_content_keep_first_last_lines_emoji_and_syntax() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let text = "# 中文 👩🏽‍💻 ❤️\r\n最后一行\n";
    let mut utf8 = vec![0xef, 0xbb, 0xbf];
    utf8.extend_from_slice(text.as_bytes());
    fs::write(root.join("readme.md"), utf8).unwrap();
    for le in [true, false] {
        let mut bytes = if le {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for c in text.encode_utf16() {
            bytes.extend_from_slice(&if le { c.to_le_bytes() } else { c.to_be_bytes() });
        }
        fs::write(root.join("legacy.ps1"), bytes).unwrap();
        let doc = files::read_file(root, Path::new("legacy.ps1"), || false).unwrap();
        assert_eq!(doc.content.lines[0].text, "# 中文 👩🏽‍💻 ❤️");
        assert_eq!(doc.content.lines[1].text, "最后一行");
        assert_eq!(doc.content.lines[2].text, "");
        assert_eq!(doc.content.lines[2].new, Some(3));
    }
    let doc = files::read_file(root, Path::new("readme.md"), || false).unwrap();
    assert_eq!(doc.encoding, "UTF-8");
    assert_eq!(doc.content.lines.len(), 3);
    fs::write(
        root.join("package.json"),
        "{\n  \"name\": \"demo\",\n  \"count\": 42\n}",
    )
    .unwrap();
    let doc = files::read_file(root, Path::new("package.json"), || false).unwrap();
    assert!(
        doc.content
            .presentation
            .rows
            .iter()
            .flat_map(|r| &r.tokens)
            .any(|t| t.color == ai_terminal::git::syntax::KEY)
    );
    fs::write(root.join("empty"), "").unwrap();
    assert!(
        files::read_file(root, Path::new("empty"), || false)
            .unwrap()
            .content
            .lines
            .is_empty()
    );
}
#[test]
fn limits_errors_and_cancellation_are_explicit_and_never_rewrite_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for (name, bytes, error) in [
        ("binary", vec![0, 1, 2], FileError::Binary),
        ("invalid", vec![0xff], FileError::Encoding),
        ("odd-utf16", vec![0xff, 0xfe, 0x20], FileError::Encoding),
    ] {
        fs::write(root.join(name), &bytes).unwrap();
        assert_eq!(
            files::read_file(root, Path::new(name), || false).unwrap_err(),
            error
        );
        assert_eq!(fs::read(root.join(name)).unwrap(), bytes);
    }
    let big = fs::File::create(root.join("large")).unwrap();
    big.set_len(files::MAX_BYTES + 1).unwrap();
    assert_eq!(
        files::read_file(root, Path::new("large"), || false).unwrap_err(),
        FileError::TooLarge
    );
    fs::write(
        root.join("lines.txt"),
        "line\n".repeat(files::MAX_LINES + 1),
    )
    .unwrap();
    let doc = files::read_file(root, Path::new("lines.txt"), || false).unwrap();
    assert!(doc.content.partial);
    assert_eq!(doc.content.lines.len(), files::MAX_LINES);
    fs::write(root.join("wide.txt"), "中".repeat(files::MAX_CHARS + 1)).unwrap();
    let doc = files::read_file(root, Path::new("wide.txt"), || false).unwrap();
    assert!(doc.content.partial);
    assert_eq!(doc.content.lines[0].text.chars().count(), files::MAX_CHARS);
    assert_eq!(
        files::read_file(root, Path::new("missing"), || true).unwrap_err(),
        FileError::Cancelled
    );
    assert!(matches!(
        files::read_directory(root, Path::new(""), || true),
        Err(FileError::Cancelled)
    ));
    assert_eq!(
        files::read_file(root, Path::new("../outside"), || false).unwrap_err(),
        FileError::OutsideProject
    );
    assert_eq!(
        files::read_file(root, &root.join("wide.txt"), || false).unwrap_err(),
        FileError::OutsideProject
    );
}
#[test]
fn cached_directories_refresh_on_demand_and_old_project_or_file_results_are_discarded() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    fs::write(a.path().join("old.txt"), "old").unwrap();
    fs::write(b.path().join("one.txt"), "one").unwrap();
    fs::write(b.path().join("two.txt"), "two").unwrap();
    let mut client = FileClient::new(egui::Context::default());
    client.set_root(Some(a.path()));
    client.ensure_directory(Path::new(""));
    client.open_file(Path::new("old.txt"));
    client.set_root(Some(b.path()));
    client.ensure_directory(Path::new(""));
    client.open_file(Path::new("one.txt"));
    client.open_file(Path::new("two.txt"));
    wait(&mut client, |c| {
        c.directories
            .get(Path::new(""))
            .is_some_and(|s| s.data.is_some())
            && c.file.as_ref().is_some_and(|s| s.data.is_some())
    });
    assert_eq!(client.file.as_ref().unwrap().path, Path::new("two.txt"));
    assert_eq!(
        client
            .file
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .content
            .lines[0]
            .text,
        "two"
    );
    fs::write(b.path().join("new.txt"), "new").unwrap();
    client.ensure_directory(Path::new(""));
    assert_eq!(
        client.directories[Path::new("")]
            .data
            .as_ref()
            .unwrap()
            .entries
            .len(),
        2
    );
    client.refresh();
    assert!(
        client.directories[Path::new("")].data.is_some(),
        "refresh must retain the tree while reading"
    );
    wait(&mut client, |c| !c.directories[Path::new("")].loading);
    assert_eq!(
        client.directories[Path::new("")]
            .data
            .as_ref()
            .unwrap()
            .entries
            .len(),
        3
    );
    client.close_file();
    client.poll();
    assert!(client.file.is_none());
}
#[test]
fn directory_cache_is_bounded_and_collapsing_releases_capacity() {
    let root = tempfile::tempdir().unwrap();
    let mut client = FileClient::new(egui::Context::default());
    client.set_root(Some(root.path()));
    for i in 0..files::MAX_DIRECTORIES + 1 {
        let name = format!("dir{i}");
        fs::create_dir(root.path().join(&name)).unwrap();
        client.ensure_directory(Path::new(&name));
    }
    assert_eq!(client.directories.len(), files::MAX_DIRECTORIES);
    assert!(client.limit_reached);
    client.collapse(Path::new("dir0"));
    client.ensure_directory(Path::new("dir64"));
    assert_eq!(client.directories.len(), files::MAX_DIRECTORIES);
    assert!(!client.limit_reached);
}
#[test]
fn links_outside_the_project_are_visible_but_never_traversed() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    let target = temp.path().join("outside");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(target.join("private.txt"), "outside fixture").unwrap();
    let link = project.join("link");
    #[cfg(windows)]
    {
        for path in [&link, &target] {
            assert!(!path.to_string_lossy().contains(['"', '&', '|', '^', '%']));
        }
        let command = format!("mklink /J \"{}\" \"{}\"", link.display(), target.display());
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new("cmd.exe")
            .args(["/D", "/C"])
            .raw_arg(command)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "junction fixture: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let entries = files::read_directory(&project, Path::new(""), || false).unwrap();
    assert_eq!(entries.entries[0].kind, EntryKind::Link);
    assert_eq!(
        files::read_file(&project, Path::new("link/private.txt"), || false).unwrap_err(),
        FileError::Link
    );
    assert!(matches!(
        files::read_directory(&project, Path::new("link"), || false),
        Err(FileError::Link)
    ));
}
