use ai_terminal::{
    media_player::{source_path, time_label},
    project_files::{Cursor, FileError, read_page},
    project_media::{MediaKind, playable_path},
};
use std::path::Path;

#[test]
fn local_media_does_not_read_large_bodies_or_treat_playlists_as_audio() {
    let root = tempfile::tempdir().unwrap();
    for (file, kind) in [
        ("配音 😀.MP3", MediaKind::Audio),
        ("film.mp4", MediaKind::Video),
    ] {
        let file_path = root.path().join(file);
        let f = std::fs::File::create(&file_path).unwrap();
        f.set_len(512 * 1024 * 1024).unwrap();
        let doc = read_page(root.path(), Path::new(file), Cursor::default(), || false).unwrap();
        assert_eq!(doc.bytes, 0);
        assert!(doc.content.lines.is_empty());
        assert!(doc.page.next.is_none());
        let media = doc.media.unwrap();
        assert_eq!(media.kind, kind);
        assert!(media.path.is_absolute());
    }
    assert_eq!(playable_path(Path::new("playlist.m3u8")), None);
    assert_eq!(playable_path(Path::new("remote.url")), None);
    assert_eq!(
        read_page(
            root.path(),
            Path::new("../film.mp4"),
            Cursor::default(),
            || false
        )
        .unwrap_err(),
        FileError::OutsideProject
    );
    assert_eq!(
        read_page(
            root.path(),
            Path::new("film.mp4"),
            Cursor::default(),
            || true
        )
        .unwrap_err(),
        FileError::Cancelled
    );
}

#[test]
fn media_source_paths_preserve_unicode_delimiters_and_unc_paths() {
    assert_eq!(
        source_path(Path::new(r"C:\media\a #100%.mp4")).unwrap(),
        r"C:\media\a #100%.mp4"
    );
    assert_eq!(
        source_path(Path::new(r"\\?\C:\测试\😀.wav")).unwrap(),
        r"C:\测试\😀.wav"
    );
    assert_eq!(
        source_path(Path::new(r"\\?\UNC\server\share\a.mp4")).unwrap(),
        r"\\server\share\a.mp4"
    );
    assert!(source_path(Path::new("https://example.invalid/file.mp4")).is_err());
    assert!(source_path(Path::new("relative.mp4")).is_err());
    assert_eq!(time_label(f64::NAN), "0:00");
    assert_eq!(time_label(3661.0), "1:01:01");
}
