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
                if let Some(file) = &self.client.file {
                    self.document_right = ui.available_rect_before_wrap().right();
                    close = project_ui::document(ui, file, self.client.root.as_deref());
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
        assert!(
            text_rect(&output, "src").unwrap().top() < 80.0,
            "A redundant row separates the toolbar and tree"
        );
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
