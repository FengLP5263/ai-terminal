use ai_terminal::{
    project_files::{self as files, Cursor, FileClient, FileError},
    project_media as media,
};
use std::{
    fs,
    io::Cursor as Buffer,
    path::Path,
    time::{Duration, Instant},
};

#[test]
fn text_larger_than_two_mib_is_paged_without_losing_unicode_or_long_line_content() {
    let root = tempfile::tempdir().unwrap();
    for utf16 in [false, true] {
        let text = "中😀abc".repeat(300_001);
        let bytes = if utf16 {
            let mut bytes = vec![0xff, 0xfe];
            bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
            bytes
        } else {
            text.as_bytes().to_vec()
        };
        assert!(bytes.len() > files::MAX_BYTES as usize);
        fs::write(root.path().join("large.txt"), &bytes).unwrap();
        let mut cursor = Cursor::default();
        let mut actual = String::new();
        let mut pages = 0;
        loop {
            let doc =
                files::read_page(root.path(), Path::new("large.txt"), cursor, || false).unwrap();
            assert!(doc.content.lines.len() <= files::MAX_LINES);
            assert_eq!(doc.page.total_bytes, bytes.len() as u64);
            for line in &doc.content.lines {
                assert!(line.text.chars().count() <= files::MAX_CHARS);
                actual.push_str(&line.text);
            }
            pages += 1;
            if let Some(next) = doc.page.next {
                assert!(next.offset > cursor.offset);
                cursor = next;
            } else {
                break;
            }
            assert!(pages < 8);
        }
        assert!(pages > 1);
        assert_eq!(actual, text);
        assert_eq!(fs::read(root.path().join("large.txt")).unwrap(), bytes);
    }
}

#[test]
fn utf16_surrogate_pairs_split_at_the_page_boundary_are_preserved_in_both_byte_orders() {
    let root = tempfile::tempdir().unwrap();
    let text = format!("{}😀末尾", "a".repeat(files::MAX_BYTES as usize / 2 - 2));
    for le in [true, false] {
        let mut bytes = if le {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for unit in text.encode_utf16() {
            bytes.extend(if le {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        fs::write(root.path().join("boundary.txt"), &bytes).unwrap();
        let first = files::read_file(root.path(), Path::new("boundary.txt"), || false).unwrap();
        let next = first.page.next.unwrap();
        let second =
            files::read_page(root.path(), Path::new("boundary.txt"), next, || false).unwrap();
        assert_eq!(second.content.lines[0].text, "😀末尾");
        let actual = first
            .content
            .lines
            .iter()
            .chain(&second.content.lines)
            .map(|l| l.text.as_str())
            .collect::<String>();
        assert_eq!(actual, text);
        assert!(second.page.next.is_none());
    }
}

#[test]
fn markdown_complexity_and_inline_image_counts_are_bounded() {
    let root = tempfile::tempdir().unwrap();
    let nested = format!("{}deep", "> ".repeat(40));
    assert!(
        media::prepare_markdown(root.path(), Path::new("README.md"), &nested, &|| false)
            .unwrap()
            .is_none()
    );
    let many = (0..20)
        .map(|i| format!("![{i}](missing-{i}.png)\n\n"))
        .collect::<String>();
    let md = media::prepare_markdown(root.path(), Path::new("README.md"), &many, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(md.images.len(), 20);
    assert_eq!(
        md.images
            .values()
            .filter(|v| matches!(v, Err(FileError::ImageLimit)))
            .count(),
        4
    );
    assert_eq!(
        media::prepare_markdown(root.path(), Path::new("README.md"), "text", &|| true).unwrap_err(),
        FileError::Cancelled
    );
}
fn wait(client: &mut FileClient) {
    let until = Instant::now() + Duration::from_secs(8);
    while client.file.as_ref().is_none_or(|f| f.loading) {
        client.poll();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn page_navigation_preserves_line_numbers_and_detects_changed_files() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("lines.txt"),
        "row\r\n".repeat(files::MAX_LINES + 2),
    )
    .unwrap();
    let mut client = FileClient::new(egui::Context::default());
    client.set_root(Some(root.path()));
    client.ensure_directory(Path::new(""));
    client.open_file(Path::new("lines.txt"));
    wait(&mut client);
    let first = client.file.as_ref().unwrap().data.clone().unwrap();
    assert_eq!(
        first.content.lines.last().unwrap().new,
        Some(files::MAX_LINES as u64)
    );
    client.next_page();
    client.refresh_on_focus(true);
    wait(&mut client);
    let second = client.file.as_ref().unwrap().data.clone().unwrap();
    assert_eq!(
        second.content.lines[0].new,
        Some(files::MAX_LINES as u64 + 1)
    );
    assert_eq!(
        second.content.lines.last().unwrap().new,
        Some(files::MAX_LINES as u64 + 3)
    );
    assert!(client.can_previous_page());
    client.refresh_on_focus(true);
    wait(&mut client);
    let refreshed = client.file.as_ref().unwrap().data.as_ref().unwrap();
    assert_eq!(refreshed.page.cursor, second.page.cursor);
    assert!(client.can_previous_page());
    client.previous_page();
    wait(&mut client);
    assert_eq!(
        client
            .file
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .page
            .cursor
            .offset,
        0
    );
    fs::write(root.path().join("lines.txt"), "changed").unwrap();
    client.next_page();
    wait(&mut client);
    assert_eq!(
        client.file.as_ref().unwrap().error,
        Some(FileError::Changed)
    );
    client.reload_file();
    wait(&mut client);
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
        "changed"
    );
    assert!(!client.can_previous_page());
}
#[test]
fn raster_formats_and_svg_preview_with_alpha_and_no_external_svg_reads() {
    let root = tempfile::tempdir().unwrap();
    for (ext, format) in [
        ("png", image::ImageFormat::Png),
        ("jpg", image::ImageFormat::Jpeg),
        ("gif", image::ImageFormat::Gif),
        ("webp", image::ImageFormat::WebP),
        ("bmp", image::ImageFormat::Bmp),
        ("ico", image::ImageFormat::Ico),
        ("tiff", image::ImageFormat::Tiff),
    ] {
        let image = if format == image::ImageFormat::Jpeg {
            image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                16,
                8,
                image::Rgb([40, 130, 200]),
            ))
        } else {
            image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                16,
                8,
                image::Rgba([40, 130, 200, 128]),
            ))
        };
        let mut encoded = Buffer::new(vec![]);
        image.write_to(&mut encoded, format).unwrap();
        let name = format!("test.{ext}");
        fs::write(root.path().join(&name), encoded.into_inner()).unwrap();
        let document = files::read_file(root.path(), Path::new(&name), || false).unwrap();
        let preview = document.image.unwrap();
        assert_eq!(preview.original, [16, 8]);
        assert_eq!(preview.pixels.size, [16, 8]);
        assert!(document.page.next.is_none());
    }
    fs::copy(
        root.path().join("test.png"),
        root.path().join("signature.bin"),
    )
    .unwrap();
    assert!(
        files::read_file(root.path(), Path::new("signature.bin"), || false)
            .unwrap()
            .image
            .is_some()
    );
    fs::write(root.path().join("shape.svg"), r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32"><rect width="32" height="32" fill="#4477ee"/><text x="1" y="20" font-size="12">Hi</text></svg>"##).unwrap();
    let svg = media::read_image(root.path(), Path::new("shape.svg"), false, &|| false).unwrap();
    assert_eq!(svg.original, [64, 32]);
    assert_eq!(svg.pixels[(63, 31)].a(), 0);
    assert_ne!(svg.pixels[(0, 0)].a(), 0);
    fs::write(root.path().join("external.svg"), r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><image href="test.png" width="16" height="16"/></svg>"#).unwrap();
    let svg = media::read_image(root.path(), Path::new("external.svg"), false, &|| false).unwrap();
    assert!(svg.pixels.pixels.iter().all(|p| p.a() == 0));
}
#[test]
fn large_image_is_supported_but_corrupt_oversized_and_office_files_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    let bitmap = image::RgbImage::from_pixel(1024, 1024, image::Rgb([1, 2, 3]));
    bitmap.save(root.path().join("large.bmp")).unwrap();
    assert!(fs::metadata(root.path().join("large.bmp")).unwrap().len() > files::MAX_BYTES);
    assert!(
        files::read_file(root.path(), Path::new("large.bmp"), || false)
            .unwrap()
            .image
            .is_some()
    );
    fs::write(root.path().join("bad.png"), "invalid").unwrap();
    assert!(matches!(
        media::read_image(root.path(), Path::new("bad.png"), false, &|| false),
        Err(FileError::Image(_))
    ));
    fs::File::create(root.path().join("too-large.jpg"))
        .unwrap()
        .set_len(media::MAX_IMAGE_BYTES + 1)
        .unwrap();
    assert_eq!(
        media::read_image(root.path(), Path::new("too-large.jpg"), false, &|| false).unwrap_err(),
        FileError::TooLarge
    );
    image::RgbImage::new(16385, 1)
        .save(root.path().join("dimensions.png"))
        .unwrap();
    assert_eq!(
        media::read_image(root.path(), Path::new("dimensions.png"), false, &|| false).unwrap_err(),
        FileError::ImageLimit
    );
    fs::write(root.path().join("document.docx"), "not text preview").unwrap();
    assert_eq!(
        files::read_file(root.path(), Path::new("document.docx"), || false).unwrap_err(),
        FileError::Unsupported
    );
    assert_eq!(
        media::read_image(root.path(), Path::new("missing.png"), false, &|| true).unwrap_err(),
        FileError::Cancelled
    );
}
#[test]
fn markdown_prepares_highlighted_code_and_only_project_local_images() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("docs")).unwrap();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([30, 100, 180, 128]))
        .save(root.path().join("图片.png"))
        .unwrap();
    let source = "# 标题\n\n| 项目 | 状态 |\n|---|---|\n| 项目 | 好 |\n\n```json\n{\"name\": \"项目\"}\n```\n\n```rust\nlet x = 1;\n```\n\n![图](../%E5%9B%BE%E7%89%87.png)\n\n![remote](https://example.invalid/a.png)\n\n[文件](../test.rs) [bad](javascript:alert)";
    let md = media::prepare_markdown(root.path(), Path::new("docs/README.md"), source, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(md.code.len(), 2);
    assert_eq!(md.code[0].lines[0].text, "{\"name\": \"项目\"}");
    assert!(!md.text.contains("```"));
    assert!(md.text.contains(&format!("<{}-0 />", md.code_marker)));
    assert_eq!(md.tables[0].rows[0], ["项目", "状态"]);
    assert!(!md.code[0].presentation.rows[0].tokens.is_empty());
    assert!(md.images["../%E5%9B%BE%E7%89%87.png"].is_ok());
    assert_eq!(
        md.images["https://example.invalid/a.png"]
            .as_ref()
            .unwrap_err(),
        &FileError::OutsideProject
    );
    assert!(md.links.iter().any(|s| s == "javascript:alert"));
    for uri in [
        "../../../file",
        "file:///c:/a",
        "https://host/a",
        "%2Foutside",
        "..%2f..%2fsecret",
        "a%3Ab",
        "//host/a",
        "%00",
    ] {
        assert!(
            media::local_target(Path::new("docs/a.md"), uri).is_err(),
            "{uri}"
        );
    }
    assert_eq!(
        media::local_target(Path::new("docs/a.md"), "../src/main.rs#line-2").unwrap(),
        Path::new("src/main.rs")
    );
    assert!(!media::external_link("javascript:alert(1)"));
    assert!(!media::external_link("https://host\n"));
    assert!(media::external_link("https://example.invalid"));
}

#[test]
fn markdown_html_images_keep_dimensions_centered_headers_and_table_line_breaks() {
    let root = tempfile::tempdir().unwrap();
    image::RgbaImage::from_pixel(160, 80, image::Rgba([40, 110, 200, 255]))
        .save(root.path().join("a&b.png"))
        .unwrap();
    let source = "<div align='center'>\n\n<IMG SRC=\"a&amp;b.png\" WIDTH=96 alt='图标'>\n\n# Team **work**\n\n[File](source.rs)\n\n</div>\n\n| Icon | Description |\n|:---:|---|\n| <img src='a&#38;b.png' width=32 height=32> | first<br>second |\n";
    let md = media::prepare_markdown(root.path(), Path::new("README.md"), source, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(md.centered.len(), 1);
    assert_eq!(md.headings[0].anchor, "team-work");
    assert_eq!(md.tables[0].rows[0], ["Icon", "Description"]);
    assert_eq!(md.tables[0].rows[1][1], "first  \nsecond");
    let sizes: Vec<_> = md
        .images
        .values()
        .map(|i| i.as_ref().unwrap().pixels.source_size)
        .collect();
    assert!(sizes.contains(&egui::vec2(96.0, 48.0)) && sizes.contains(&egui::vec2(32.0, 32.0)));
    assert!(md.links.contains(&"source.rs".into()));
    assert!(!md.text.contains("<IMG") && !md.tables[0].rows[1][0].contains("<img"));
    assert!(
        md.images
            .values()
            .all(|i| i.as_ref().unwrap().original == [160, 80])
    );
}

#[test]
fn markdown_html_does_not_execute_or_fetch_and_preserves_code_literals() {
    let root = tempfile::tempdir().unwrap();
    let source = "<!-- private decoration -->\n<script>discarded-script</script>\n\n`<img src='literal.png'>`\n\n```html\n<img src='literal.png'>\n```\n\n<img src='https://example.invalid/a.png' onerror='run()'>\n\n<a href='javascript:alert(1)'>blocked</a>\n\n## Same\n\n## Same\n";
    let md = media::prepare_markdown(root.path(), Path::new("README.md"), source, &|| false)
        .unwrap()
        .unwrap();
    assert!(
        !md.text.contains("discarded-script")
            && !md.text.contains("private decoration")
            && !md.text.contains("onerror")
    );
    assert!(md.text.contains("`<img src='literal.png'>`"));
    assert_eq!(md.code[0].lines[0].text, "<img src='literal.png'>");
    assert_eq!(md.images.len(), 1);
    assert_eq!(
        md.images.values().next().unwrap().as_ref().unwrap_err(),
        &FileError::OutsideProject
    );
    assert!(md.links.contains(&"javascript:alert(1)".into()));
    assert_eq!(
        md.headings
            .iter()
            .map(|h| h.anchor.as_str())
            .collect::<Vec<_>>(),
        ["same", "same-1"]
    );
}

#[test]
fn repository_readmes_prepare_native_logos_tables_and_section_anchors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["README.md", "README.en.md"] {
        let source = fs::read_to_string(root.join(name)).unwrap();
        let md = media::prepare_markdown(root, Path::new(name), &source, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(md.centered.len(), 1, "{name}");
        assert!(md.tables.len() >= 3, "{name}");
        assert!(
            md.images[if name == "README.en.md" {
                "assets/readme/workspace.en.png"
            } else {
                "assets/readme/workspace.png"
            }]
            .is_ok(),
            "The first content image must not lose its budget to later images: {name}"
        );
        let settings = if name == "README.en.md" {
            "assets/readme/settings.en.png"
        } else {
            "assets/readme/settings.png"
        };
        assert!(
            md.images[settings].is_ok(),
            "The later settings screenshot is a valid local image: {name}: {:?}",
            md.images[settings]
        );
        assert!(md.headings.len() > 10, "{name}");
        assert_eq!(md.headings[0].text, "AI Terminal");
        let icon_references: Vec<_> = md
            .tables
            .iter()
            .flat_map(|table| &table.rows)
            .flatten()
            .flat_map(|cell| {
                pulldown_cmark::Parser::new(cell).filter_map(|event| match event {
                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image {
                        dest_url, ..
                    }) => Some(dest_url.into_string()),
                    _ => None,
                })
            })
            .collect();
        assert!(
            icon_references
                .iter()
                .filter(|uri| md.images.get(*uri).is_some_and(|image| image
                    .as_ref()
                    .is_ok_and(|image| image.pixels.source_size == egui::vec2(32.0, 32.0))))
                .count()
                >= 7,
            "All seven Agent image references must render: {name}"
        );
        assert!(
            md.images
                .values()
                .filter_map(|i| i.as_ref().ok())
                .any(|i| i.pixels.source_size.x == 96.0)
        );
        assert!(
            md.tables
                .iter()
                .flat_map(|t| &t.rows)
                .flatten()
                .all(|cell| !cell.contains("<img"))
        );
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), source);
    }
}

#[test]
fn repeated_html_images_share_a_resource_without_consuming_later_image_slots() {
    let root = tempfile::tempdir().unwrap();
    for name in ["icon.png", "later.png"] {
        image::RgbaImage::from_pixel(24, 24, image::Rgba([10, 90, 180, 255]))
            .save(root.path().join(name))
            .unwrap();
    }
    let source = format!(
        "{}\n\n![later](later.png)",
        "<img src='icon.png' width='32' height='32' alt='badge'>\n\n".repeat(24)
    );
    let md = media::prepare_markdown(root.path(), Path::new("README.md"), &source, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(
        md.images.len(),
        2,
        "Repeated display references must reuse the same prepared URI and texture identity"
    );
    assert!(md.images.values().all(Result::is_ok));
    let uris: Vec<_> = pulldown_cmark::Parser::new(&md.text)
        .filter_map(|event| match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) => {
                Some(dest_url.into_string())
            }
            _ => None,
        })
        .collect();
    assert_eq!(uris.len(), 25);
    assert!(uris[..24].iter().all(|uri| uri == &uris[0]));
    assert_ne!(uris[0], uris[24]);
}

#[test]
fn reused_html_resources_do_not_bypass_the_rendering_complexity_limit() {
    let root = tempfile::tempdir().unwrap();
    let source = "<img src='same.png' width='32'>".repeat(6000);
    assert!(source.len() < 256 * 1024);
    assert!(
        media::prepare_markdown(root.path(), Path::new("README.md"), &source, &|| false)
            .unwrap()
            .is_none()
    );
}
