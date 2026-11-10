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
                Some(Rect::from_min_size(text.pos, text.galley.size()))
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
    let close = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "\u{ea76}" && t.pos.x < 850.0 => {
                Some(t.pos + egui::vec2(4.0, 4.0))
            }
            _ => None,
        })
        .next()
        .unwrap();
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
fn files_use_shared_codicons_and_inherited_font_without_scaling_a_painted_layer() {
    let mut f = Fixture::new();
    for zoom in [0.71, 1.0, 1.5, 2.0] {
        f.ctx.set_zoom_factor(zoom);
        f.frame(vec![], 520.0);
        let output = f.frame(vec![], 520.0);
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
        let icon = texts
            .iter()
            .find(|t| t.galley.job.text == "\u{eb0f}")
            .unwrap();
        assert_eq!(
            file.galley.job.sections[0].format.font_id.size,
            icon.galley.job.sections[0].format.font_id.size
        );
        assert_eq!(
            icon.galley.job.sections[0].format.font_id.family,
            egui::FontFamily::Name("codicons".into())
        );
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
