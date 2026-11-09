use ai_terminal::{
    appearance,
    git::{Commit, FileChange, GitError, Snapshot},
    git_ui::{Actions, GitView},
    i18n::{self, Language},
    model::Workspace,
};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};

fn fixture() -> Snapshot {
    Snapshot {
        root: "C:/Projects/demo".into(),
        git_dir: "C:/Projects/demo/.git".into(),
        common_dir: "C:/Projects/demo/.git".into(),
        branch: "feature/native-git".into(),
        detached: false,
        unborn: false,
        added: 3,
        deleted: 1,
        total_files: 1,
        partial: false,
        more: true,
        files: vec![FileChange {
            path: "src/终端.rs".into(),
            index: ' ',
            worktree: 'M',
            added: 3,
            deleted: 1,
            binary: false,
            omitted: false,
        }],
        commits: vec![Commit {
            id: "1234567890abcdef1234567890abcdef12345678".into(),
            parents: vec![],
            author: "Fixture".into(),
            timestamp: 0,
            subject: "feat: local Git panel".into(),
        }],
    }
}
fn text_rect(output: &egui::FullOutput, value: &str) -> Option<Rect> {
    fn find(shape: &egui::Shape, value: &str) -> Option<Rect> {
        match shape {
            egui::Shape::Text(t) if t.galley.job.text == value => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, value)),
            _ => None,
        }
    }
    output.shapes.iter().find_map(|s| find(&s.shape, value))
}
fn pointer(pos: Pos2, pressed: bool) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        },
    ]
}
#[test]
fn real_controls_switch_history_expand_commit_request_more_and_refresh() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    i18n::apply(&ctx, Language::En);
    let snapshot = fixture();
    let mut view = GitView::default();
    let mut tick = 0;
    let mut frame = |events| {
        tick += 1;
        let mut action = Actions::default();
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(340.0, 700.0))),
                time: Some(tick as f64 * 0.1),
                events,
                ..Default::default()
            },
            |ui| {
                action = view.contents(ui, Some(&snapshot), None, false, false, 50);
            },
        );
        (output, action)
    };
    frame(vec![]);
    let (out, _) = frame(vec![]);
    assert!(text_rect(&out, "src/终端.rs").is_some());
    let pos = text_rect(&out, "History").unwrap().center();
    frame(pointer(pos, true));
    let (out, _) = frame(pointer(pos, false));
    let (out, _) = if text_rect(&out, "feat: local Git panel").is_none() {
        frame(vec![])
    } else {
        (out, Actions::default())
    };
    assert!(text_rect(&out, "1970-01-01").is_some());
    let pos = text_rect(&out, "feat: local Git panel").unwrap().center();
    frame(pointer(pos, true));
    frame(pointer(pos, false));
    let (out, _) = frame(vec![]);
    assert!(text_rect(&out, &snapshot.commits[0].id).is_some());
    let pos = text_rect(&out, "Load more commits").unwrap().center();
    frame(pointer(pos, true));
    let (_, actions) = frame(pointer(pos, false));
    assert!(actions.more);
    let (out, _) = frame(vec![]);
    let pos = text_rect(&out, "Refresh").unwrap().center();
    frame(pointer(pos, true));
    let (_, actions) = frame(pointer(pos, false));
    assert!(actions.refresh);
    let pos = ctx
        .read_response(egui::Id::new("git-close"))
        .unwrap()
        .rect
        .center();
    frame(pointer(pos, true));
    let (_, actions) = frame(pointer(pos, false));
    assert!(actions.close);
}
#[test]
fn missing_git_non_repository_and_unborn_states_are_explicit_and_settings_are_compatible() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    i18n::apply(&ctx, Language::En);
    let mut view = GitView::default();
    let out = ctx.run_ui(Default::default(), |ui| {
        view.contents(ui, None, None, false, true, 50);
    });
    assert!(text_rect(&out, "The current directory is not in a Git repository.").is_some());
    let out = ctx.run_ui(Default::default(), |ui| {
        view.contents(ui, None, Some(&GitError::Missing), false, false, 50);
    });
    assert!(
        text_rect(
            &out,
            "Git was not found. Install Git for Windows, then refresh."
        )
        .is_some()
    );
    let mut snapshot = fixture();
    snapshot.unborn = true;
    snapshot.commits.clear();
    let out = ctx.run_ui(Default::default(), |ui| {
        view.contents(ui, Some(&snapshot), None, false, false, 50);
    });
    assert!(text_rect(&out, "No commits yet").is_some());
    let old: Workspace = serde_json::from_str(r#"{"sessions":[],"active":null,"ui":{}}"#).unwrap();
    assert!(!old.ui.git_panel_open);
}

#[test]
fn long_commit_subjects_wrap_inside_narrow_panels() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let mut snapshot = fixture();
    snapshot.commits[0].subject =
        "fix: preserve active terminal during all repository layout changes and session switches"
            .into();
    let mut view = GitView::default();
    view.set_history(true);
    let mut last = None;
    for _ in 0..3 {
        last = Some(ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(240.0, 700.0))),
                ..Default::default()
            },
            |ui| {
                view.contents(ui, Some(&snapshot), None, false, false, 50);
            },
        ));
    }
    let rect = text_rect(last.as_ref().unwrap(), &snapshot.commits[0].subject).unwrap();
    assert!(
        rect.width() <= 240.0,
        "Commit text escaped its panel: {rect:?}"
    );
    assert!(
        rect.height() > 20.0,
        "The long subject must wrap instead of clipping"
    );
}

#[test]
fn header_shares_one_row_and_inherits_the_selected_font_and_ui_size() {
    use ai_terminal::{font_choice::FontChoice, git::GitClient, git_ui};
    for choice in [FontChoice::JetBrainsMono, FontChoice::Hack] {
        for scale in [1.0, 1.14, 2.0] {
            let ctx = egui::Context::default();
            appearance::setup(&ctx);
            appearance::set_terminal_font(&ctx, choice);
            ctx.style_mut_of(egui::Theme::Dark, |style| {
                style
                    .text_styles
                    .insert(egui::TextStyle::Body, egui::FontId::proportional(17.0));
            });
            ctx.set_zoom_factor(scale);
            let mut client = GitClient::new(ctx.clone());
            client.snapshot = Some(std::sync::Arc::new(fixture()));
            let mut open = false;
            let mut last = None;
            for _ in 0..3 {
                last = Some(ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(1100.0, 180.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        git_ui::header(
                            ui,
                            git_ui::Header {
                                name: "Session",
                                directory: std::path::Path::new("C:/Projects/demo"),
                                scale,
                                connection: None,
                            },
                            Some(&client),
                            &mut open,
                        );
                        ui.label("terminal-content");
                    },
                ));
            }
            let output = last.unwrap();
            let mut centers = vec![];
            for text in [
                "Session",
                "C:/Projects/demo",
                "feature/native-git *",
                "+3",
                "−1",
            ] {
                let rect =
                    text_rect(&output, text).unwrap_or_else(|| panic!("Header omitted {text}"));
                centers.push(rect.center().y);
                let shape = output
                    .shapes
                    .iter()
                    .find_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape {
                            (t.galley.job.text == text).then_some(t)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                assert!(
                    shape
                        .galley
                        .job
                        .sections
                        .iter()
                        .all(|s| s.format.font_id.family == egui::FontFamily::Monospace
                            && s.format.font_id.size == 17.0),
                    "{text} did not inherit the app font/style"
                );
            }
            assert!(
                centers.iter().all(|y| (*y - centers[0]).abs() < 4.0),
                "Header split across rows: {centers:?}"
            );
            let icon = text_rect(&output, "\u{ec6f}").expect("Official Codicons branch glyph");
            assert!(
                icon.height() <= 24.0,
                "Icon grew beyond inherited text size"
            );
            assert!(
                text_rect(&output, "terminal-content").unwrap().top() < 55.0,
                "Header consumed an extra row"
            );
        }
    }
}

#[test]
fn narrow_headers_shorten_metadata_instead_of_overlapping_or_wrapping() {
    use ai_terminal::{git::GitClient, git_ui};
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let mut client = GitClient::new(ctx.clone());
    client.snapshot = Some(std::sync::Arc::new(fixture()));
    for width in [240.0, 360.0, 540.0] {
        let mut open = false;
        let mut last = None;
        for _ in 0..3 {
            last = Some(ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 180.0))),
                    ..Default::default()
                },
                |ui| {
                    git_ui::header(
                        ui,
                        git_ui::Header {
                            name: "Long session title",
                            directory: std::path::Path::new(
                                "C:/Projects/long-directory-name/workspace",
                            ),
                            scale: 1.14,
                            connection: None,
                        },
                        Some(&client),
                        &mut open,
                    );
                    ui.label("terminal-content");
                },
            ));
        }
        let output = last.unwrap();
        let name = text_rect(&output, "Long session title").unwrap();
        let branch = text_rect(&output, "feature/native-git *").unwrap();
        let scale = text_rect(&output, "114%").unwrap();
        assert!(name.right() <= branch.left());
        assert!(branch.right() <= scale.left());
        assert!(text_rect(&output, "terminal-content").unwrap().top() < 50.0);
    }
}
