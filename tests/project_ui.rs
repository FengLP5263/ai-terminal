use ai_terminal::{
    appearance,
    git_ui::GitView,
    i18n::{self, Language},
    project_files::FileClient,
    project_ui::{self, FileView},
};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn text_rect(output: &egui::FullOutput, value: &str) -> Option<Rect> {
    fn find(shape: &egui::Shape, value: &str) -> Option<Rect> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == value => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, value)),
            _ => None,
        }
    }
    output.shapes.iter().find_map(|s| find(&s.shape, value))
}
fn wait(client: &mut FileClient, ready: impl Fn(&FileClient) -> bool) {
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        client.poll();
        if ready(client) {
            return;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
}
struct Fixture {
    root: tempfile::TempDir,
    ctx: egui::Context,
    client: FileClient,
    view: FileView,
    git: GitView,
    files_open: bool,
    git_open: bool,
    document_right: f32,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/app.rs"), "// 示例 ❤️\nfn main() {}\n").unwrap();
        std::fs::write(root.path().join("package.json"), "{\"name\":\"demo\"}").unwrap();
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        i18n::apply(&ctx, Language::En);
        let mut client = FileClient::new(ctx.clone());
        client.set_root(Some(root.path()));
        client.ensure_directory(Path::new(""));
        wait(&mut client, |c| c.directories[Path::new("")].data.is_some());
        Self {
            root,
            ctx,
            client,
            view: Default::default(),
            git: Default::default(),
            files_open: true,
            git_open: false,
            document_right: 0.0,
        }
    }
    fn frame(&mut self, events: Vec<Event>, width: f32) -> egui::FullOutput {
        let mut open = None;
        let mut close = false;
        let output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 720.0))),
                events,
                ..Default::default()
            },
            |ui| {
                open = project_ui::panel(
                    ui,
                    &mut self.client,
                    &mut self.view,
                    None,
                    &mut self.git,
                    &mut self.git_open,
                    &mut self.files_open,
                )
                .file;
                if self.client.file.is_some() {
                    self.document_right = ui.available_rect_before_wrap().right();
                    close = self.view.document(ui, &mut self.client);
                } else {
                    ui.label("TERMINAL");
                }
            },
        );
        if let Some(path) = open {
            self.client.open_file(&path);
        }
        if close {
            self.client.close_file();
        }
        output
    }
    fn click(&mut self, pos: Pos2, width: f32) {
        for pressed in [true, false] {
            self.frame(
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
                width,
            );
        }
    }
}
#[test]
fn folder_click_loads_children_and_file_click_opens_a_read_only_document() {
    let mut f = Fixture::new();
    f.frame(vec![], 1200.0);
    let output = f.frame(vec![], 1200.0);
    let src = text_rect(&output, "src").unwrap();
    f.click(src.center(), 1200.0);
    wait(&mut f.client, |c| {
        c.directories
            .get(Path::new("src"))
            .is_some_and(|s| s.data.is_some())
    });
    let output = f.frame(vec![], 1200.0);
    let file = text_rect(&output, "app.rs").unwrap();
    f.click(file.center(), 1200.0);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| s.data.is_some())
    });
    let output = f.frame(vec![], 1200.0);
    assert!(text_rect(&output, "src/app.rs").is_some());
    assert!(text_rect(&output, "Read only").is_some());
    assert_eq!(
        std::fs::read_to_string(f.root.path().join("src/app.rs")).unwrap(),
        "// 示例 ❤️\nfn main() {}\n"
    );
    let close = f
        .ctx
        .read_response(egui::Id::new("project-document-close"))
        .unwrap()
        .rect
        .center();
    f.click(close, 1200.0);
    assert!(f.client.file.is_none());
}

#[test]
fn typing_a_file_search_opens_an_unexpanded_match_and_clear_restores_the_tree() {
    let mut f = Fixture::new();
    f.frame(vec![], 1200.0);
    let search_id = egui::Id::new("project-file-search");
    let search = f.ctx.read_response(search_id).unwrap().rect;
    f.click(search.center(), 1200.0);
    assert!(project_ui::search_has_focus(&f.ctx));
    f.frame(vec![Event::Text("APP.RS".into())], 1200.0);
    wait(&mut f.client, |c| {
        c.search_state().is_some_and(|s| s.result.is_some())
    });
    let output = f.frame(vec![], 1200.0);
    assert!(!f.client.directories.contains_key(Path::new("src")));
    f.click(text_rect(&output, "app.rs").unwrap().center(), 1200.0);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| s.data.is_some())
    });
    assert_eq!(
        f.client.file.as_ref().unwrap().path,
        Path::new("src/app.rs")
    );

    let search = f.ctx.read_response(search_id).unwrap().rect;
    f.click(search.center(), 1200.0);
    let output = f.frame(
        vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        1200.0,
    );
    assert!(f.client.search_state().is_none());
    assert!(text_rect(&output, "package.json").is_some());
}
#[test]
fn panel_tabs_are_exclusive_and_the_files_entry_exists_outside_git() {
    let mut f = Fixture::new();
    f.frame(vec![], 1000.0);
    let output = f.frame(vec![], 1000.0);
    let git = text_rect(&output, "\u{ec6f} Git").unwrap();
    f.click(git.center(), 1000.0);
    assert!(f.git_open && !f.files_open);
    let output = f.frame(vec![], 1000.0);
    let files = text_rect(&output, "\u{ea83} Files").unwrap();
    f.click(files.center(), 1000.0);
    assert!(f.files_open && !f.git_open);
    let mut git_open = false;
    let mut files_open = false;
    let mut frame = |events| {
        f.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 500.0))),
                events,
                ..Default::default()
            },
            |ui| {
                ai_terminal::git_ui::header_with_files(
                    ui,
                    ai_terminal::git_ui::Header {
                        name: "Shell",
                        directory: f.root.path(),
                        scale: 1.0,
                        connection: None,
                    },
                    None,
                    &mut git_open,
                    &mut files_open,
                    None,
                );
            },
        )
    };
    frame(vec![]);
    let output = frame(vec![]);
    let icon = text_rect(&output, "\u{ea83}").unwrap();
    for pressed in [true, false] {
        frame(vec![
            Event::PointerMoved(icon.center()),
            Event::PointerButton {
                pos: icon.center(),
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    assert!(files_open && !git_open);
}

#[test]
fn markdown_renders_code_switches_to_source_and_routes_only_safe_local_links() {
    let mut f = Fixture::new();
    let source = "# Preview title\n\n**Strong text**\n\n```json\n{\n  \"name\": \"demo\"\n}\n```\n\n[Local file](src/app.rs) [Blocked](javascript:alert)\n";
    std::fs::write(f.root.path().join("README.md"), source).unwrap();
    f.client.open_file(Path::new("README.md"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| !s.loading)
    });
    f.frame(vec![], 1200.0);
    let output = f.frame(vec![], 1200.0);
    assert!(text_rect(&output, "Preview title").is_some());
    assert!(
        text_rect(&output, "  \"name\": \"demo\"").is_some_and(|row| text_rect(&output, "{").is_some_and(|first| row.top()>first.top())),
        "fenced code must be rendered"
    );
    let source_button = text_rect(&output, "Source").unwrap().center();
    f.click(source_button, 1200.0);
    let output = f.frame(vec![], 1200.0);
    assert!(text_rect(&output, "# Preview title").is_some());
    f.click(text_rect(&output, "Preview").unwrap().center(), 1200.0);
    let output = f.frame(vec![], 1200.0);
    let blocked = text_rect(&output, "Blocked").unwrap();
    // Inline galleys include preceding horizontal layout space in their bounding box.
    f.click(
        egui::pos2(blocked.right() - 5.0, blocked.center().y),
        1200.0,
    );
    let output = f.frame(vec![], 1200.0);
    assert!(text_rect(&output, "This link is not supported.").is_some());
    assert!(
        !output
            .platform_output
            .commands
            .iter()
            .any(|c| matches!(c, egui::OutputCommand::OpenUrl(_)))
    );
    f.click(text_rect(&output, "Local file").unwrap().center(), 1200.0);
    assert_eq!(
        f.client.file.as_ref().unwrap().path,
        Path::new("src/app.rs")
    );
    assert_eq!(
        std::fs::read_to_string(f.root.path().join("README.md")).unwrap(),
        source
    );
}

#[test]
fn markdown_html_header_is_centered_and_tables_wrap_without_raw_tags_or_page_overflow() {
    let mut f = Fixture::new();
    f.files_open = false;
    image::RgbaImage::from_pixel(160, 80, image::Rgba([30, 120, 210, 255]))
        .save(f.root.path().join("logo.png"))
        .unwrap();
    let description = "This table description must wrap within its own column instead of widening the entire document. ".repeat(3);
    let source = format!(
        "<div align='center'>\n\n<img src='logo.png' width='96'>\n\n# Doc title\n\n[Header file](src/app.rs)\n\n</div>\n\n| Icon | Description |\n|---|---|\n| <img src='logo.png' width='32' height='32'> | {description} |\n\n[After table](src/app.rs)\n"
    );
    std::fs::write(f.root.path().join("README.md"), &source).unwrap();
    f.client.open_file(Path::new("README.md"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|file| !file.loading)
    });
    for width in [700.0, 1100.0] {
        f.frame(vec![], width);
        let output = f.frame(vec![], width);
        let heading = text_rect(&output, "Doc title").unwrap();
        assert!(
            (heading.center().x - width / 2.0).abs() < 4.0,
            "Centered heading at width {width}: {heading:?}"
        );
        let cell = text_rect(&output, description.trim()).unwrap();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(row) if [egui::Color32::from_rgb(19,22,27), egui::Color32::from_rgb(13,16,20)].contains(&row.fill) && row.rect.shrink(7.0).contains_rect(cell))), "Row borders must remain outside wrapped text: {cell:?}");
        assert!(
            cell.height() > 40.0 && cell.right() < width - 15.0,
            "Table cell must wrap: {cell:?}"
        );
        assert!(text_rect(&output, "<img src='logo.png' width='32' height='32'>").is_none());
        let manager = f.ctx.tex_manager();
        let manager = manager.read();
        let image_bounds: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Rect(rect) = &shape.shape
                    && let Some(brush) = &rect.brush
                    && manager
                        .meta(brush.fill_texture_id)
                        .is_some_and(|meta| meta.name.contains("-image-"))
                {
                    return Some(rect.rect);
                }
                if let egui::Shape::Mesh(mesh) = &shape.shape
                    && manager
                        .meta(mesh.texture_id)
                        .is_some_and(|meta| meta.name.contains("-image-"))
                {
                    Some(mesh.calc_bounds())
                } else {
                    None
                }
            })
            .collect();
        assert!(image_bounds.iter().any(|rect| (rect.width() - 96.0).abs() < 1.0 && (rect.height() - 48.0).abs() < 1.0), "Header image dimensions: {image_bounds:?}");
        assert!(image_bounds.iter().any(|rect| (rect.width() - 32.0).abs() < 1.0 && (rect.height() - 32.0).abs() < 1.0), "Table icon dimensions: {image_bounds:?}");
    }
    let output = f.frame(vec![], 1100.0);
    f.click(text_rect(&output, "Header file").unwrap().center(), 1100.0);
    assert_eq!(
        f.client.file.as_ref().unwrap().path,
        Path::new("src/app.rs")
    );
    assert_eq!(
        std::fs::read_to_string(f.root.path().join("README.md")).unwrap(),
        source
    );
}

#[test]
fn markdown_centered_navigation_scrolls_to_a_generated_section_anchor() {
    let mut f = Fixture::new();
    f.files_open = false;
    let source = format!(
        "<div align='center'>\n\n[Jump to end](#end)\n\n</div>\n\n# Start\n\n{}\n\n## End\n\nLast paragraph",
        "A paragraph with enough room to test scrolling.\n\n".repeat(50)
    );
    std::fs::write(f.root.path().join("README.md"), source).unwrap();
    f.client.open_file(Path::new("README.md"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|file| !file.loading)
    });
    f.frame(vec![], 900.0);
    let before = f.frame(vec![], 900.0);
    assert!(text_rect(&before, "End").is_none_or(|rect| rect.top() > 720.0));
    f.click(text_rect(&before, "Jump to end").unwrap().center(), 900.0);
    for _ in 0..8 {
        f.frame(vec![], 900.0);
    }
    let after = f.frame(vec![], 900.0);
    assert!(
        text_rect(&after, "End").unwrap().top() < 650.0,
        "A local section link must scroll the document"
    );
    assert!(
        !after
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::OpenUrl(_)))
    );
}

#[test]
#[cfg(windows)]
fn markdown_prose_headings_and_tables_use_native_color_emoji() {
    let mut f = Fixture::new();
    f.files_open = false;
    let _renderer = egui_term::TextRendererSession::new(&f.ctx);
    let source =
        "# Heading 🧠\n\nParagraph 📋\n\n| Title |\n| --- |\n| Cell 👩🏽‍💻 |\n\nMath $E=mc^2$.\n";
    let path = Path::new("README.md");
    std::fs::write(f.root.path().join(path), source).unwrap();
    f.client.open_file(path);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|file| !file.loading)
    });
    f.frame(vec![], 900.0);
    let output = f.frame(vec![], 900.0);
    let color_glyphs = output
        .shapes
        .iter()
        .filter_map(|entry| match &entry.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                Some(mesh.calc_bounds())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for label in ["Heading 🧠", "Paragraph 📋", "Cell 👩🏽‍💻"] {
        let rect = text_rect(&output, label).expect(label);
        assert_eq!(
            color_glyphs
                .iter()
                .filter(|glyph| glyph.intersects(rect))
                .count(),
            1,
            "Expected one native glyph for the full emoji in {label}"
        );
    }
    assert!(text_rect(&output, "Math $E=mc^2$.").is_some());
    f.view.preview_mut().set_source(true);
    let source_view = f.frame(vec![], 900.0);
    assert!(text_rect(&source_view, "# Heading 🧠").is_some());
    assert_eq!(
        std::fs::read_to_string(f.root.path().join(path)).unwrap(),
        source
    );
}

#[test]
fn refreshing_and_reopening_a_markdown_page_keep_its_reading_position() {
    let mut f = Fixture::new();
    f.files_open = false;
    let path = Path::new("README.md");
    let source = (0..80)
        .map(|i| format!("Reading paragraph {i}\n\n"))
        .collect::<String>();
    std::fs::write(f.root.path().join(path), &source).unwrap();
    f.client.open_file(path);
    let loaded = |c: &FileClient| c.file.as_ref().is_some_and(|file| !file.loading);
    wait(&mut f.client, loaded);
    let output = f.frame(vec![], 900.0);
    let top = text_rect(&output, "Reading paragraph 0").unwrap();
    f.frame(
        vec![
            Event::PointerMoved(egui::pos2(300.0, 350.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(0.0, -650.0),
                modifiers: Modifiers::NONE,
            },
        ],
        900.0,
    );
    let mut scrolled = f.frame(vec![], 900.0);
    for _ in 0..10 {
        scrolled = f.frame(vec![], 900.0);
    }
    let (marker, before) = (1..80)
        .find_map(|i| {
            let text = format!("Reading paragraph {i}");
            let rect = text_rect(&scrolled, &text)?;
            (rect.center().y > 180.0 && rect.center().y < 490.0).then_some((text, rect))
        })
        .expect("scrolled paragraph must be visible");
    assert!(
        text_rect(&scrolled, "Reading paragraph 0")
            .is_none_or(|rect| rect.top() < top.top() - 50.0)
    );
    std::fs::write(f.root.path().join(path), source + "\nFresh content\n").unwrap();
    f.client.refresh_on_focus(false);
    wait(&mut f.client, loaded);
    let refreshed = f.frame(vec![], 900.0);
    let after = text_rect(&refreshed, &marker).expect("reading position survived refresh");
    assert!(
        (before.top() - after.top()).abs() < 8.0,
        "{before:?} -> {after:?}"
    );

    let file = f.client.take_bookmark().unwrap();
    let view = f.view.preview_mut().take_bookmark(&f.ctx, path).unwrap();
    f.client.restore_bookmark(file);
    f.view.preview_mut().restore_bookmark(view);
    f.frame(vec![], 900.0);
    wait(&mut f.client, loaded);
    let reopened = f.frame(vec![], 900.0);
    let restored = text_rect(&reopened, &marker).expect("reading position survived reopening");
    assert!((after.top() - restored.top()).abs() < 8.0);
}

#[test]
fn markdown_without_a_rendered_preview_shows_source_without_a_preview_button() {
    let mut f = Fixture::new();
    f.files_open = false;
    let path = Path::new("nested.md");
    let source = format!("# Actual source\n\n{}deep\n", "> ".repeat(40));
    std::fs::write(f.root.path().join(path), source).unwrap();
    f.client.open_file(path);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|file| !file.loading)
    });
    let document = f.client.file.as_ref().unwrap().data.as_ref().unwrap();
    assert!(document.markdown.is_none());
    let output = f.frame(vec![], 900.0);
    assert!(text_rect(&output, "Source").is_some());
    assert!(text_rect(&output, "Preview").is_none());
    assert!(text_rect(&output, "# Actual source").is_some());
    assert!(text_rect(&output, "This content exceeds the Markdown preview limits.").is_some());
}

#[test]
fn markdown_code_blocks_share_reading_width_keep_gaps_and_copy_raw_emoji_and_tabs() {
    let mut f = Fixture::new();
    f.files_open = false;
    let _renderer = egui_term::TextRendererSession::new(&f.ctx);
    let short = "ai-terminal-cli instance info --json\nai-terminal-cli session list --json\nai-terminal-cli session rename current --name '主协调' --json";
    let long = format!(
        "$r = ai-terminal-cli session create --name '前端 ✨ 👩🏽‍💻' --directory $PWD.Path --json {}\n\tif (-not $r.ok) {{ throw $r.error.message }}\nai-terminal-cli session run $r.result.results[0].id --agent codex",
        " --argument=value".repeat(25)
    );
    let source = format!(
        "Before code\n\n```powershell\n{short}\n```\n\nBetween blocks\n\n```powershell\n{long}\n```\n\nAfter code\n"
    );
    std::fs::write(f.root.path().join("README.md"), &source).unwrap();
    f.client.open_file(Path::new("README.md"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|file| !file.loading)
    });
    for (width, zoom) in [(800.0, 1.0), (1400.0, 1.0), (1200.0, 1.33)] {
        f.ctx.set_zoom_factor(zoom);
        f.frame(vec![], width);
        let output = f.frame(vec![], width);
        let blocks: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == egui::Color32::from_gray(30) && rect.rect.width() > 250.0 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(blocks.len(), 2);
        assert!(
            (blocks[0].width() - blocks[1].width()).abs() < 1.0
                && (blocks[0].left() - blocks[1].left()).abs() < 1.0,
            "Code blocks must use the same reading width: {blocks:?}"
        );
        let before = text_rect(&output, "Before code").unwrap();
        let between = text_rect(&output, "Between blocks").unwrap();
        let after = text_rect(&output, "After code").unwrap();
        assert!(blocks[0].top() - before.bottom() >= 10.0);
        assert!(between.top() - blocks[0].bottom() >= 10.0);
        assert!(blocks[1].top() - between.bottom() >= 10.0);
        assert!(after.top() - blocks[1].bottom() >= 10.0);
        let copies: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Copy" => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(copies.len(), 2);
        for (block, button) in blocks.iter().zip(&copies) {
            assert!(
                block.right() - button.right() < 30.0 && block.contains_rect(*button),
                "Copy stays in the right header: {block:?} {button:?}"
            );
        }
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default())), "Code emoji must use the native bitmap path");
    }
    f.ctx.set_zoom_factor(1.0);
    f.frame(vec![], 1000.0);
    let output = f.frame(vec![], 1000.0);
    let copy = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Copy" => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            }
            _ => None,
        })
        .nth(1)
        .unwrap();
    let pointer = |pressed| Event::PointerButton {
        pos: copy,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    f.frame(vec![Event::PointerMoved(copy), pointer(true)], 1000.0);
    let copied = f.frame(vec![pointer(false)], 1000.0);
    assert!(
        copied.platform_output.commands.iter().any(
            |command| matches!(command, egui::OutputCommand::CopyText(value) if value == &long)
        ),
        "Copy must preserve tabs and the original Unicode text"
    );
    let first_line = long.lines().next().unwrap();
    let initial = text_rect(&copied, first_line).unwrap();
    let pointer = egui::pos2(initial.left() + 30.0, initial.center().y);
    f.frame(
        vec![
            Event::PointerMoved(pointer),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(-400.0, 0.0),
                modifiers: Modifiers::NONE,
            },
        ],
        1000.0,
    );
    for _ in 0..8 {
        f.frame(vec![], 1000.0);
    }
    let scrolled = f.frame(vec![], 1000.0);
    assert!(
        text_rect(&scrolled, first_line).unwrap().left() < initial.left() - 30.0,
        "Long code lines must remain reachable by horizontal scrolling"
    );
    assert_eq!(
        std::fs::read_to_string(f.root.path().join("README.md")).unwrap(),
        source
    );
}

#[test]
fn image_controls_and_text_page_buttons_remain_inside_the_read_only_view() {
    let mut f = Fixture::new();
    image::RgbaImage::from_pixel(16, 8, image::Rgba([10, 100, 210, 128]))
        .save(f.root.path().join("image.png"))
        .unwrap();
    f.client.open_file(Path::new("image.png"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| !s.loading)
    });
    f.frame(vec![], 1200.0);
    let output = f.frame(vec![], 1200.0);
    assert!(text_rect(&output, "Fit").is_some());
    assert!(text_rect(&output, "100%").is_some());
    assert!(text_rect(&output, "Read only").is_some());
    for (width, zoom) in [(800.0, 1.0), (1200.0, 1.33), (1600.0, 2.0)] {
        f.ctx.set_zoom_factor(zoom);
        f.frame(vec![], width);
        let output = f.frame(vec![], width);
        let manager = f.ctx.tex_manager();
        let manager = manager.read();
        let image = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Mesh(mesh) = &shape.shape
                    && manager
                        .meta(mesh.texture_id)
                        .is_some_and(|meta| meta.name == "project-image")
                {
                    Some((mesh.calc_bounds(), shape.clip_rect))
                } else {
                    None
                }
            })
            .expect("The loaded image must be drawn");
        assert!(
            (image.0.center().x - image.1.center().x).abs() < 1.0,
            "Image must be centered horizontally"
        );
        assert!(
            (image.0.center().y - image.1.center().y).abs() < 1.0,
            "Image must be centered vertically"
        );
    }
    f.ctx.set_zoom_factor(1.0);
    std::fs::write(f.root.path().join("large.txt"), "row\n".repeat(20_004)).unwrap();
    f.client.open_file(Path::new("large.txt"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| !s.loading)
    });
    let output = f.frame(vec![], 1200.0);
    f.click(text_rect(&output, "Next").unwrap().center(), 1200.0);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| !s.loading)
    });
    let output = f.frame(vec![], 1200.0);
    assert_eq!(
        f.client
            .file
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .content
            .lines[0]
            .new,
        Some(20_001)
    );
    f.click(text_rect(&output, "Previous").unwrap().center(), 1200.0);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| !s.loading)
    });
    assert_eq!(
        f.client
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
}

#[test]
fn ignored_rows_are_muted_and_remain_expandable_and_readable() {
    let mut f = Fixture::new();
    let git = ai_terminal::discovery::CliSearch::current().git().unwrap();
    assert!(
        std::process::Command::new(git)
            .args(["init", "-q"])
            .current_dir(f.root.path())
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(f.root.path().join(".gitignore"), "src/\n").unwrap();
    f.client.refresh();
    wait(&mut f.client, |c| !c.directories[Path::new("")].loading);
    let output = f.frame(vec![], 1200.0);
    fn color(output: &egui::FullOutput, name: &str) -> egui::Color32 {
        fn find(shape: &egui::Shape, name: &str) -> Option<egui::Color32> {
            match shape {
                egui::Shape::Text(text) if text.galley.job.text == name => {
                    Some(text.galley.job.sections[0].format.color)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, name)),
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|s| find(&s.shape, name))
            .unwrap()
    }
    assert_eq!(color(&output, "src"), ai_terminal::chrome::SECONDARY);
    assert_ne!(color(&output, "src"), color(&output, "package.json"));
    let folder_y = text_rect(&output, "src").unwrap().center().y;
    let folder_icon = output.shapes.iter().find_map(|shape| match &shape.shape {
        egui::Shape::Text(text)
            if text.galley.job.text == "\u{ea83}"
                && (text.galley.rect.translate(text.pos.to_vec2()).center().y - folder_y).abs()
                    < 2.0 =>
        {
            Some(text.galley.job.sections[0].format.color)
        }
        _ => None,
    });
    assert_eq!(folder_icon, Some(ai_terminal::chrome::SECONDARY));
    f.click(text_rect(&output, "src").unwrap().center(), 1200.0);
    wait(&mut f.client, |c| {
        c.directories
            .get(Path::new("src"))
            .is_some_and(|s| s.data.is_some())
    });
    let output = f.frame(vec![], 1200.0);
    assert_eq!(color(&output, "app.rs"), ai_terminal::chrome::SECONDARY);
    f.click(text_rect(&output, "app.rs").unwrap().center(), 1200.0);
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| s.data.is_some())
    });
    let output = f.frame(vec![], 1200.0);
    assert!(text_rect(&output, "src/app.rs").is_some());
    assert!(text_rect(&output, "Read only").is_some());
}
#[test]
fn files_use_shared_type_icons_and_inherited_font_without_scaling_a_painted_layer() {
    let mut f = Fixture::new();
    for zoom in [0.71, 1.0, 1.5, 2.0] {
        f.ctx.set_zoom_factor(zoom);
        f.frame(vec![], 520.0);
        let output = f.frame(vec![], 520.0);
        let root_name = f.root.path().file_name().unwrap().to_string_lossy();
        assert!(
            text_rect(&output, &root_name).is_none(),
            "The panel repeats the project name"
        );
        let tabs = text_rect(&output, "\u{ea83} Files").unwrap();
        let collapse = text_rect(&output, "\u{eac5}").unwrap();
        let refresh = text_rect(&output, "\u{eb37}").unwrap();
        let close = text_rect(&output, "\u{ea76}").unwrap();
        assert!((tabs.center().y - collapse.center().y).abs() < 2.0);
        assert!((collapse.center().y - refresh.center().y).abs() < 2.0);
        assert!((refresh.center().y - close.center().y).abs() < 2.0);
        assert!(
            tabs.right() <= collapse.left()
                && collapse.right() <= refresh.left()
                && refresh.right() <= close.left()
        );
        let search = f
            .ctx
            .read_response(egui::Id::new("project-file-search"))
            .unwrap()
            .rect;
        let tree = text_rect(&output, "src").unwrap();
        assert!(search.top() > tabs.bottom() && search.bottom() <= tree.top());
        assert!(tree.top() - search.bottom() < 20.0);
        let texts = output
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    Some(t)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let file = texts
            .iter()
            .find(|t| t.galley.job.text == "package.json")
            .unwrap();
        let file_rect = text_rect(&output, "package.json").unwrap();
        let icon = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Mesh(mesh)
                    if (mesh.calc_bounds().center().y - file_rect.center().y).abs() < 2.0
                        && mesh.calc_bounds().right() < file_rect.left() =>
                {
                    Some(mesh.calc_bounds())
                }
                _ => None,
            })
            .unwrap();
        assert!((icon.width() - file.galley.job.sections[0].format.font_id.size).abs() < 1.0);
        assert!(file_rect.left() - icon.right() < 12.0);
        assert!(
            text_rect(&output, "\u{ec6f} Git").unwrap().right()
                < text_rect(&output, "\u{ea76}").unwrap().left()
        );
    }
}
#[test]
fn old_settings_keep_git_preference_and_new_file_panel_round_trips() {
    let old: ai_terminal::model::UiSettings =
        serde_json::from_str("{\"git_panel_open\":true}").unwrap();
    assert!(old.git_panel_open && !old.project_files_open);
    let mut new = old;
    new.project_files_open = true;
    let new = new.normalize();
    assert!(!new.git_panel_open && new.project_files_open);
    let saved: ai_terminal::model::UiSettings =
        serde_json::from_str(&serde_json::to_string(&new).unwrap()).unwrap();
    assert_eq!(saved, new);
}

#[test]
fn named_project_entry_uses_the_tree_root_and_opens_when_its_text_is_clicked() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    i18n::apply(&ctx, Language::En);
    let project = Path::new("C:/Workspace/项目文件 ❤️ long project name");
    let directory = project.join("src");
    let status = ai_terminal::status::Status::default();
    for (width, zoom) in [(320.0, 1.0), (540.0, 1.33), (1100.0, 2.0)] {
        ctx.set_zoom_factor(zoom);
        let mut git_open = true;
        let mut files_open = false;
        let mut frame = |events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 200.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ai_terminal::git_ui::header_with_files(
                        ui,
                        ai_terminal::git_ui::Header {
                            name: "Shell",
                            directory: &directory,
                            scale: zoom,
                            connection: Some((ai_terminal::model::Agent::Codex, &status)),
                        },
                        None,
                        &mut git_open,
                        &mut files_open,
                        Some(project),
                    );
                    ui.label("TERMINAL");
                },
            )
        };
        frame(vec![]);
        let output = frame(vec![]);
        let label = text_rect(&output, project.file_name().unwrap().to_str().unwrap()).unwrap();
        let scale = text_rect(&output, &format!("{:.0}%", zoom * 100.0)).unwrap();
        assert!(label.right() <= scale.left() && label.top() < 40.0);
        if width > 540.0 {
            let state = text_rect(
                &output,
                ai_terminal::i18n::activity(ai_terminal::i18n::Locale::En, status.activity),
            )
            .unwrap();
            assert!(
                label.right() <= state.left() && state.right() <= scale.left(),
                "Expected folder, status, zoom order"
            );
        }
        assert!(text_rect(&output, "TERMINAL").unwrap().top() < 50.0);
        for pressed in [true, false] {
            frame(vec![
                Event::PointerMoved(label.center()),
                Event::PointerButton {
                    pos: label.center(),
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        assert!(files_open && !git_open);
    }
}

#[test]
fn read_only_title_keeps_the_icon_next_to_the_left_aligned_filename() {
    let mut f = Fixture::new();
    f.client.open_file(Path::new("src/app.rs"));
    wait(&mut f.client, |c| {
        c.file.as_ref().is_some_and(|s| s.data.is_some())
    });
    for (width, zoom) in [(900.0, 1.0), (1200.0, 1.33), (1600.0, 2.0)] {
        f.ctx.set_zoom_factor(zoom);
        // read_response uses the preceding pass; settle the DPI and width transition.
        f.frame(vec![], width);
        f.frame(vec![], width);
        let output = f.frame(vec![], width);
        let title = text_rect(&output, "src/app.rs").unwrap();
        let close = f
            .ctx
            .read_response(egui::Id::new("project-document-close"))
            .unwrap()
            .rect;
        assert!(
            (close.right() - f.document_right).abs() < 1.0,
            "Close must align with the content edge: width={width} zoom={zoom} close={close:?} right={}",
            f.document_right
        );
        let read_only = text_rect(&output, "Read only").unwrap();
        assert!(read_only.right() <= close.left() + 1.0 && close.left() - read_only.right() < 10.0);

        let icon = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Mesh(m)
                    if (m.calc_bounds().center().y - title.center().y).abs() < 3.0
                        && m.calc_bounds().left() < 40.0 =>
                {
                    Some(m.calc_bounds())
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "Missing title icon: width={width} zoom={zoom} title={title:?} meshes={:?}",
                    output
                        .shapes
                        .iter()
                        .filter_map(|s| if let egui::Shape::Mesh(m) = &s.shape {
                            Some(m.calc_bounds())
                        } else {
                            None
                        })
                        .collect::<Vec<_>>()
                )
            });
        assert!(
            title.left() >= icon.right() && title.left() - icon.right() < 14.0,
            "Title separated from its icon"
        );
    }
}
