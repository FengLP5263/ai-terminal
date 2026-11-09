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
    let pos = text_rect(&out, "feat: local Git panel").unwrap().center();
    frame(pointer(pos, true));
    let (_, action) = frame(pointer(pos, false));
    assert_eq!(
        action.inspect.as_deref(),
        Some(snapshot.commits[0].id.as_str())
    );
    let (out, _) = frame(vec![]);
    assert!(text_rect(&out, "Reading commit files…").is_some());
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
fn long_commit_subjects_keep_a_compact_row_inside_narrow_panels() {
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
        rect.height() <= 22.0,
        "The history must keep one row per commit"
    );
}

#[test]
fn history_hover_copy_and_expansion_use_real_widgets_without_changing_selection() {
    use ai_terminal::git::{CommitDetails, CommitFile, GitClient, Inspection};
    use std::sync::Arc;
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    i18n::apply(&ctx, Language::En);
    let snapshot = fixture();
    let commit = snapshot.commits[0].clone();
    let mut client = GitClient::new(ctx.clone());
    client.snapshot = Some(Arc::new(snapshot));
    client.watching = true;
    client.inspection = Some(Inspection {
        id: commit.id.clone(),
        data: Some(Arc::new(CommitDetails {
            id: commit.id.clone(),
            message: "feat: local Git panel\n\nComplete message body".into(),
            parents: vec![],
            files: vec![CommitFile {
                path: "past/changed.txt".into(),
                status: 'M',
                added: 13,
                deleted: 3,
                binary: false,
            }],
            total_files: 1,
            added: 13,
            deleted: 3,
            partial: false,
        })),
        error: None,
        loading: false,
    });
    let mut view = GitView::default();
    view.set_history(true);
    let mut tick = 0;
    let mut frame = |events| {
        tick += 1;
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0))),
                time: Some(tick as f64 * 0.2),
                events,
                ..Default::default()
            },
            |ui| {
                view.panel(ui, Some(&client), &mut true);
            },
        )
    };
    frame(vec![]);
    frame(vec![]);
    let row = ctx
        .read_response(egui::Id::new(("git-commit-row", &commit.id)))
        .unwrap()
        .rect;
    let mut out = frame(vec![Event::PointerMoved(row.center())]);
    for _ in 0..8 {
        out = frame(vec![]);
    }
    assert!(text_rect(&out, "1970-01-01 00:00:00 UTC").is_some());
    assert!(text_rect(&out, "feat: local Git panel\n\nComplete message body").is_some());
    assert!(text_rect(&out, "past/changed.txt").is_none());
    let short = text_rect(&out, "1234567").expect("Seven-character commit ID");
    let symbol = text_rect(&out, "\u{eafc}").expect("Official Codicons commit symbol");
    // Wrapped horizontal labels carry first-row indentation inside the galley.
    let short_glyph_left = out
        .shapes
        .iter()
        .find_map(|s| {
            let egui::Shape::Text(t) = &s.shape else {
                return None;
            };
            if t.galley.job.text != "1234567" {
                return None;
            }
            let row = t.galley.rows.first()?;
            Some(t.pos.x + row.pos.x + row.row.glyphs.first()?.pos.x)
        })
        .unwrap();
    assert!(
        symbol.right() <= short_glyph_left && (symbol.center().y - short.center().y).abs() < 1.0,
        "Commit icon must precede the abbreviated ID: icon={symbol:?}, id={short:?}"
    );
    assert!(
        text_rect(&out, &commit.id).is_none(),
        "Full hash should not occupy the hover card"
    );
    // The last copy glyph belongs to the hover card, not the list row.
    let copy = out
        .shapes
        .iter()
        .rev()
        .find_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                (t.galley.job.text == "\u{ebcc}")
                    .then(|| Rect::from_min_size(t.pos, t.galley.size()).center())
            } else {
                None
            }
        })
        .expect("Hover card copy control");
    frame(vec![Event::PointerMoved(copy)]);
    frame(pointer(copy, true));
    let out = frame(pointer(copy, false));
    assert!(
        out.platform_output.commands.iter().any(
            |command| matches!(command,egui::OutputCommand::CopyText(text) if text==&commit.id)
        )
    );
    assert!(
        text_rect(&out, "past/changed.txt").is_none(),
        "Copy must not expand the commit"
    );
    frame(vec![Event::PointerGone]);
    frame(pointer(row.center(), true));
    frame(pointer(row.center(), false));
    let out = frame(vec![Event::PointerGone]);
    assert!(text_rect(&out, "past/changed.txt").is_some());
    assert!(text_rect(&out, "+13").is_some());
    assert!(text_rect(&out, "−3").is_some());
    frame(pointer(row.center(), true));
    frame(pointer(row.center(), false));
    let out = frame(vec![Event::PointerGone]);
    assert!(text_rect(&out, "past/changed.txt").is_none());
}

#[test]
fn file_icons_are_shared_by_both_views_align_with_commit_text_and_loading_does_not_reflow() {
    use ai_terminal::git::{CommitDetails, CommitFile, GitClient, Inspection};
    use std::sync::Arc;
    for scale in [1.0, 1.14, 2.0] {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        i18n::apply(&ctx, Language::En);
        ctx.set_zoom_factor(scale);
        let mut snapshot = fixture();
        let examples = [
            ("README.md", "\u{eb1d}"),
            ("src/app.rs", "\u{eae9}"),
            ("settings.json", "\u{eb0f}"),
            (".gitignore", "\u{eb52}"),
            ("build.ps1", "\u{ea85}"),
            ("icon.png", "\u{eaea}"),
            ("blob.dat", "\u{eae8}"),
        ];
        snapshot.files = examples
            .iter()
            .map(|(path, _)| FileChange {
                path: (*path).into(),
                index: ' ',
                worktree: 'M',
                added: 1,
                deleted: 0,
                binary: path.ends_with(".dat"),
                omitted: false,
            })
            .collect();
        snapshot.total_files = examples.len();
        let commit = snapshot.commits[0].clone();
        let details = CommitDetails {
            id: commit.id.clone(),
            message: commit.subject.clone(),
            parents: vec![],
            files: snapshot
                .files
                .iter()
                .map(|f| CommitFile {
                    path: f.path.clone(),
                    status: 'M',
                    added: f.added,
                    deleted: f.deleted,
                    binary: f.binary,
                })
                .collect(),
            total_files: examples.len(),
            added: 7,
            deleted: 0,
            partial: false,
        };
        let mut client = GitClient::new(ctx.clone());
        client.snapshot = Some(Arc::new(snapshot));
        client.watching = true;
        client.inspection = Some(Inspection {
            id: commit.id.clone(),
            data: Some(Arc::new(details)),
            error: None,
            loading: false,
        });
        let mut view = GitView::default();
        let mut tick = 0;
        let mut frame = |events, loading, history| {
            tick += 1;
            client.loading = loading;
            view.set_history(history);
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1100.0, 900.0))),
                    time: Some(tick as f64 * 0.2),
                    events,
                    ..Default::default()
                },
                |ui| {
                    view.panel(ui, Some(&client), &mut true);
                },
            )
        };
        frame(vec![], false, false);
        let out = frame(vec![], false, false);
        let normal_y = text_rect(&out, "README.md").unwrap().top();
        for (_, glyph) in examples {
            assert!(
                text_rect(&out, glyph).is_some(),
                "Missing Changes icon {glyph}"
            );
        }
        let out = frame(vec![], true, false);
        assert_eq!(
            text_rect(&out, "README.md").unwrap().top(),
            normal_y,
            "Loading inserted a row"
        );
        assert!(text_rect(&out, "Reading…").is_some());
        assert!(text_rect(&out, "Reading local repository…").is_none());
        frame(vec![], false, true);
        let out = frame(vec![], false, true);
        let pos = text_rect(&out, &commit.subject).unwrap().center();
        frame(pointer(pos, true), false, true);
        frame(pointer(pos, false), false, true);
        let out = frame(vec![Event::PointerGone], false, true);
        let subject = text_rect(&out, &commit.subject).unwrap();
        for (_, glyph) in examples {
            let icon = text_rect(&out, glyph).expect("Shared expanded-file icon");
            assert!(
                (icon.left() - subject.left()).abs() < 0.5,
                "Icon not aligned to commit text at {scale}: {icon:?} vs {subject:?}"
            );
        }
    }
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
