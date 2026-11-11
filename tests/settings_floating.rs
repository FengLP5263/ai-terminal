use ai_terminal::{
    appearance,
    floating::{self, Action, Item, Snapshot},
    i18n::{self, Language, Locale},
    model::{self, Agent, UiSettings, Workspace},
    settings,
    sidebar::Icons,
    status::Activity,
};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};

#[test]
fn saved_position_observations_do_not_reposition_an_existing_floating_window() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    ctx.set_embed_viewports(false);
    let bar = floating::FloatingBar::new(Icons::new(&ctx));
    for position in [[100.0, 80.0], [2700.0, 110.0], [2500.0, 90.0]] {
        let mut input = egui::RawInput::default();
        input.viewports.insert(
            floating::viewport_id(),
            egui::ViewportInfo {
                native_pixels_per_point: Some(1.5),
                outer_rect: Some(Rect::from_min_size(
                    Pos2::new(position[0] / 1.5, position[1] / 1.5),
                    egui::vec2(488.0, 48.0),
                )),
                ..Default::default()
            },
        );
        let output = ctx.run_ui(input, |_| bar.show(&ctx, Some(position)));
        let child = &output.viewport_output[&floating::viewport_id()];
        assert!(
            child.builder.position.is_none(),
            "observed position is not a placement command"
        );
        assert!(
            !child
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::OuterPosition(_)))
        );
    }
}

#[test]
fn old_shared_scale_migrates_once_and_independent_scales_survive_restart() {
    let old: UiSettings =
        serde_json::from_str(r#"{"ui_scale":1.5,"floating_enabled":true}"#).unwrap();
    let mut migrated = old.normalize();
    assert_eq!(migrated.floating_scale, 1.5);
    migrated.ui_scale = 0.75;
    assert_eq!(migrated.normalize().floating_scale, 1.5);
    migrated.floating_scale = 2.0;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sessions.json");
    let workspace = Workspace {
        ui: migrated,
        ..Default::default()
    };
    model::save(&file, &workspace).unwrap();
    let restored = model::load(&file).unwrap().ui.normalize();
    assert_eq!((restored.ui_scale, restored.floating_scale), (0.75, 2.0));
    for (value, expected) in [(f32::NAN, 1.0), (0.1, 0.5), (5.0, 2.5)] {
        assert_eq!(
            UiSettings {
                floating_scale: value,
                ..Default::default()
            }
            .normalize()
            .floating_scale,
            expected
        );
    }
}

#[test]
fn floating_geometry_and_scaled_clicks_are_independent_of_main_zoom() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    ctx.set_embed_viewports(false);
    let icons = Icons::new(&ctx);
    let bar = floating::FloatingBar::new(icons.clone());
    let snapshot = Snapshot {
        items: vec![Item {
            id: "scaled".into(),
            name: "Scaled".into(),
            agent: Agent::Codex,
            color: None,
            activity: Activity::Idle,
            unread: false,
        }],
        active: Some("scaled".into()),
        locale: Locale::En,
    };
    for main_scale in [0.71, 1.0, 1.5, 2.0] {
        ai_terminal::zoom::initialize(&ctx, main_scale);
        for bar_scale in [0.75, 1.0, 1.42, 2.0] {
            bar.set_scale(&ctx, bar_scale);
            let mut input = egui::RawInput::default();
            input
                .viewports
                .insert(floating::viewport_id(), Default::default());
            let output = ctx.run_ui(input, |_| bar.show(&ctx, Some([100.0, 100.0])));
            let size = output.viewport_output[&floating::viewport_id()]
                .builder
                .inner_size
                .unwrap();
            assert!((size.x * main_scale - 96.0 * bar_scale).abs() < 0.01);
            assert!((size.y * main_scale - 48.0 * bar_scale).abs() < 0.01);
            let mut action = None;
            let mut frame = |events| {
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        events,
                        focused: true,
                        ..Default::default()
                    },
                    |ui| {
                        action = floating::body_at_scale(ui, &snapshot, &icons, bar_scale);
                    },
                )
            };
            frame(vec![]);
            frame(vec![]);
            let row = ctx
                .read_response(egui::Id::new(("floating-session", "scaled")))
                .unwrap();
            let pos = row.rect.center();
            frame(pointer(pos, true));
            frame(pointer(pos, false));
            assert_eq!(
                action,
                Some(Action::Activate("scaled".into())),
                "main={main_scale} bar={bar_scale}"
            );
            assert_eq!(ctx.zoom_factor(), main_scale);
        }
    }
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
fn floating_icons_are_rasterized_at_their_final_physical_size() {
    for main in [0.71, 1.5] {
        for own in [1.0, 1.33] {
            let ctx = egui::Context::default();
            appearance::setup(&ctx);
            ai_terminal::zoom::initialize(&ctx, main);
            let icons = Icons::new(&ctx);
            let snapshot = Snapshot {
                items: [Agent::Shell, Agent::Codex]
                    .into_iter()
                    .enumerate()
                    .map(|(i, agent)| Item {
                        id: i.to_string(),
                        name: "Crisp".into(),
                        agent,
                        color: None,
                        activity: Activity::Idle,
                        unread: false,
                    })
                    .collect(),
                active: None,
                locale: Locale::En,
            };
            let ratio = own / main;
            let mut input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(104.0, 48.0) * ratio,
                )),
                ..Default::default()
            };
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(1.5);
            let out = ctx.run_ui(input, |ui| {
                floating::body_at_scale(ui, &snapshot, &icons, own);
            });
            let pixels = (ai_terminal::chrome::AVATAR_ICON * own * 1.5).round() as usize;
            assert!(
                out.textures_delta
                    .set
                    .iter()
                    .any(|(_, d)| d.image.size() == [pixels, pixels]),
                "Avatar texture must match final pixels, main={main} floating={own}"
            );
            let font = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == "\u{ebc7}" => {
                        Some(t.galley.job.sections[0].format.font_id.size)
                    }
                    _ => None,
                })
                .expect("PowerShell glyph");
            assert!(
                (font * ctx.pixels_per_point() - pixels as f32).abs() < 0.01,
                "Glyphs must be shaped for the final size without a scaled text layer"
            );
        }
    }
}
fn text_rect(output: &egui::FullOutput, text: &str) -> Option<Rect> {
    fn find(shape: &egui::Shape, text: &str) -> Option<Rect> {
        match shape {
            egui::Shape::Text(t) if t.galley.job.text == text => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, text)),
            _ => None,
        }
    }
    output.shapes.iter().find_map(|s| find(&s.shape, text))
}
#[test]
fn old_workspaces_follow_system_and_new_language_and_island_options_survive_restart() {
    let old: Workspace =
        serde_json::from_str(r#"{"sessions":[],"active":null,"ui":{"ui_scale":1.5}}"#).unwrap();
    assert_eq!(old.ui.language, Language::System);
    assert!(!old.ui.floating_enabled);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let mut workspace = old;
    workspace.ui.language = Language::En;
    workspace.ui.floating_enabled = true;
    workspace.ui.floating_position = Some([-520.0, 16.0]);
    workspace.ui.floating_position_pixels = Some([-780.0, 24.0]);
    model::save(&path, &workspace).unwrap();
    let restored = model::load(&path).unwrap();
    assert_eq!(restored.ui, workspace.ui);
    assert!(
        UiSettings {
            floating_position: Some([f32::NAN, 0.0]),
            ..Default::default()
        }
        .normalize()
        .floating_position
        .is_none()
    );
    for tag in ["zh-CN", "zh_TW.UTF-8", "ZH-hans"] {
        assert_eq!(Locale::from_tag(tag), Locale::ZhCn);
    }
    assert_eq!(Locale::from_tag("en-US"), Locale::En);
    assert_eq!(Locale::from_tag("de-DE"), Locale::En);
}
#[test]
fn settings_switches_language_through_the_dropdown_and_toggles_the_island() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    i18n::apply(&ctx, Language::ZhCn);
    let mut preferences = UiSettings {
        language: Language::ZhCn,
        ..Default::default()
    };
    let mut open = true;
    let mut tick = 0;
    let mut frame = |events: Vec<Event>| {
        tick += 1;
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 650.0))),
                time: Some(tick as f64 * 0.1),
                events,
                ..Default::default()
            },
            |_| {
                if settings::show(&ctx, &mut open, &mut preferences) {
                    i18n::apply(&ctx, preferences.language);
                }
            },
        );
        (out, preferences)
    };
    frame(vec![]);
    let (out, _) = frame(vec![]);
    let pos = text_rect(&out, "简体中文").unwrap().center();
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (out, _) = frame(vec![]);
    let pos = text_rect(&out, "English").unwrap().center();
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (out, prefs) = frame(vec![]);
    assert_eq!(prefs.language, Language::En);
    assert!(text_rect(&out, "Interface language").is_some());
    assert!(text_rect(&out, "Done").is_none());
    let pos = text_rect(&out, "Enabled").unwrap().center();
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (_, prefs) = frame(vec![]);
    assert!(prefs.floating_enabled);
    let (out, _) = frame(vec![]);
    let pos = text_rect(&out, "JetBrains Mono").unwrap().center();
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (out, _) = frame(vec![]);
    let pos = text_rect(&out, "Hack").unwrap().center();
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (out, prefs) = frame(vec![]);
    assert_eq!(
        prefs.terminal_font,
        ai_terminal::font_choice::FontChoice::Hack
    );
    assert!(text_rect(&out, "Settings").is_some());
    assert!(text_rect(&out, "Done").is_none());
    let value = text_rect(&out, "100%").unwrap();
    let pos = Pos2::new(value.left() - 64.0, value.center().y);
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (out, prefs) = frame(vec![]);
    assert_ne!(
        prefs.floating_scale, 1.0,
        "Slider changes apply while the form remains open"
    );
    assert!(text_rect(&out, "Settings").is_some());
    let value = text_rect(&out, "0%").expect("Transparency percentage");
    let pos = Pos2::new(value.left() - 64.0, value.center().y);
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (out, prefs) = frame(vec![]);
    assert!(
        prefs.floating_opacity < 1.0,
        "Transparency applies while Settings is open"
    );
    assert!(text_rect(&out, "Settings").is_some());
}

#[test]
fn settings_rows_align_and_fit_wide_and_narrow_views_without_changing_global_style() {
    for locale in [Language::ZhCn, Language::En] {
        for width in [360.0, 1000.0] {
            let ctx = egui::Context::default();
            appearance::setup(&ctx);
            i18n::apply(&ctx, locale);
            let original = ctx.style_of(egui::Theme::Dark).text_styles.clone();
            let mut open = true;
            let mut prefs = UiSettings {
                floating_enabled: true,
                language: locale,
                ..Default::default()
            };
            let mut output = Default::default();
            for _ in 0..4 {
                output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(width, 900.0),
                        )),
                        ..Default::default()
                    },
                    |_| {
                        settings::show(&ctx, &mut open, &mut prefs);
                    },
                );
            }
            let tr = locale.resolve();
            let labels = [
                tr.text("界面语言", "Interface language"),
                tr.text("终端字体", "Terminal font"),
                tr.text("置顶状态浮窗", "Always-on-top status bar"),
                tr.text("浮窗缩放", "Floating bar scale"),
                tr.text("背景透明度", "Background transparency"),
            ];
            let rects: Vec<_> = labels
                .into_iter()
                .map(|s| text_rect(&output, s).unwrap())
                .collect();
            for rect in &rects {
                assert!(
                    rect.left() >= 0.0 && rect.right() <= width + 1.0 && rect.bottom() <= 900.0,
                    "Clipped form label: {rect:?}"
                );
                assert!(
                    (rect.left() - rects[0].left()).abs() < 1.0,
                    "Labels share a left edge"
                );
            }
            let selected = text_rect(&output, prefs.language.label(tr)).unwrap();
            let font = text_rect(&output, "JetBrains Mono").unwrap();
            assert!((selected.left() - font.left()).abs() < 1.0);
            assert!(font.right() <= width);
            if width > 800.0 {
                assert!(font.left() > rects[1].right() + 12.0);
            } else {
                assert!(font.top() > rects[1].bottom());
            }
            assert!(text_rect(&output, "Done").is_none() && text_rect(&output, "完成").is_none());
            assert_eq!(
                ctx.style_of(egui::Theme::Dark).text_styles,
                original,
                "Dialog style must not alter terminal typography"
            );
        }
    }
}
#[test]
fn island_clicks_identify_sessions_and_keep_failure_question_and_completion_distinct() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let icons = Icons::new(&ctx);
    let snapshot = Snapshot {
        items: [
            Activity::Working,
            Activity::Completed,
            Activity::Waiting,
            Activity::Failed,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, activity)| Item {
            id: i.to_string(),
            name: format!("Session {i} 👩🏽‍💻"),
            agent: Agent::Codex,
            color: None,
            activity,
            unread: true,
        })
        .collect(),
        active: Some("0".into()),
        locale: Locale::En,
    };
    let mut tick = 0;
    let mut frame = |events| {
        tick += 1;
        let mut action = None;
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(780.0, 48.0))),
                time: Some(tick as f64 * 0.1),
                events,
                ..Default::default()
            },
            |ui| {
                action = floating::body(ui, &snapshot, &icons);
            },
        );
        (out, action)
    };
    frame(vec![]);
    let (out, _) = frame(vec![]);
    for status in ["Working", "Completed", "Needs attention", "Failed"] {
        assert!(
            text_rect(&out, status).is_none(),
            "status is shown only on hover: {status}"
        );
    }
    assert_ne!(
        floating::status_color(Activity::Waiting),
        floating::status_color(Activity::Failed)
    );
    assert_ne!(
        floating::status_color(Activity::Completed),
        floating::status_color(Activity::Waiting)
    );
    let pos = ctx
        .read_response(egui::Id::new(("floating-session", &"2".to_string())))
        .unwrap()
        .rect
        .center();
    frame(vec![Event::PointerMoved(pos)]);
    for _ in 0..20 {
        let (out, _) = frame(vec![]);
        assert!(
            text_rect(&out, "Session 2 👩🏽‍💻 · Codex · Needs attention").is_none(),
            "the native hint must not be painted/clipped inside the capsule"
        );
    }
    frame(pointer(pos, true));
    let (_, action) = frame(pointer(pos, false));
    assert_eq!(action, Some(Action::Activate("2".into())));
    assert!(ctx.read_response(egui::Id::new("floating-close")).is_none());
}

#[test]
fn a_single_session_stays_clickable_after_a_long_hover() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let icons = Icons::new(&ctx);
    let snapshot = Snapshot {
        items: vec![Item {
            id: "one".into(),
            name: "A long session name for tooltip".into(),
            agent: Agent::Codex,
            color: None,
            activity: Activity::Waiting,
            unread: true,
        }],
        ..Default::default()
    };
    let mut tick = 0;
    let mut frame = |events| {
        tick += 1;
        let mut action = None;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(96.0, 48.0))),
                events,
                time: Some(tick as f64 * 0.1),
                ..Default::default()
            },
            |ui| {
                action = floating::body(ui, &snapshot, &icons);
            },
        );
        action
    };
    frame(vec![]);
    frame(vec![Event::PointerMoved(egui::pos2(28.0, 24.0))]);
    for _ in 0..20 {
        frame(vec![]);
    }
    frame(pointer(egui::pos2(28.0, 24.0), true));
    assert_eq!(
        frame(pointer(egui::pos2(28.0, 24.0), false)),
        Some(Action::Activate("one".into()))
    );
}

#[test]
fn floating_window_moves_from_its_unmarked_edge() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let icons = Icons::new(&ctx);
    let snapshot = Snapshot::default();
    let mut tick = 0;
    let mut frame = |events| {
        tick += 1;
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(250.0, 48.0))),
                events,
                time: Some(tick as f64 * 0.1),
                ..Default::default()
            },
            |ui| {
                floating::body(ui, &snapshot, &icons);
            },
        )
    };
    frame(vec![]);
    let pressed = frame(pointer(egui::pos2(100.0, 2.0), true));
    frame(vec![Event::PointerMoved(egui::pos2(120.0, 2.0))]);
    assert!(
        pressed
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .any(|command| matches!(command, egui::ViewportCommand::StartDrag)),
        "dragging blank edge starts the native window drag; response={:?}",
        ctx.read_response(egui::Id::new(("floating-window-edge", 0usize)))
    );
}

fn session_rects(output: &egui::FullOutput) -> Vec<Rect> {
    fn visit(shape: &egui::Shape, found: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Circle(circle) if (circle.radius - 16.0).abs() < 0.1 => {
                found.push(Rect::from_center_size(
                    circle.center,
                    egui::Vec2::splat(32.0),
                ));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| visit(shape, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for shape in &output.shapes {
        visit(&shape.shape, &mut found);
    }
    found
}

#[test]
fn floating_width_stops_growing_after_ten_sessions() {
    let ctx = egui::Context::default();
    let (_, nine) = floating::geometry(&ctx, 9, None);
    let (_, ten) = floating::geometry(&ctx, 10, None);
    let (_, many) = floating::geometry(&ctx, 100, None);
    assert!(
        ten > nine,
        "the first ten sessions fit when the monitor allows it"
    );
    assert_eq!(many, ten, "more than ten sessions cannot widen the bar");
}

#[test]
fn many_sessions_stay_still_and_only_drag_scrolls_without_opening_a_session() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let icons = Icons::new(&ctx);
    let snapshot = Snapshot {
        items: (0..40)
            .map(|i| Item {
                id: i.to_string(),
                name: format!("Session {i}"),
                agent: Agent::Codex,
                color: None,
                activity: Activity::Idle,
                unread: false,
            })
            .collect(),
        ..Default::default()
    };
    let mut tick = 0;
    let mut frame = |events| {
        tick += 1;
        let mut action = None;
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(488.0, 48.0))),
                events,
                time: Some(tick as f64 * 0.1),
                ..Default::default()
            },
            |ui| {
                action = floating::body(ui, &snapshot, &icons);
            },
        );
        (out, action)
    };
    frame(vec![]);
    let initial = session_rects(&frame(vec![]).0);
    assert_eq!(
        initial
            .iter()
            .filter(|r| r.left() >= 6.0 && r.right() <= 482.0)
            .count(),
        10
    );
    let before = initial[2];
    for _ in 0..30 {
        frame(vec![]);
    }
    let still = session_rects(&frame(vec![]).0)[2];
    assert_eq!(still, before, "overflow must not move on its own");
    frame(vec![
        Event::PointerMoved(egui::pos2(250.0, 24.0)),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            phase: egui::TouchPhase::Move,
            delta: egui::vec2(0.0, -3.0),
            modifiers: Modifiers::NONE,
        },
    ]);
    assert_eq!(
        session_rects(&frame(vec![]).0)[2],
        before,
        "wheel must not move the bar"
    );
    let start = egui::pos2(250.0, 24.0);
    frame(pointer(start, true));
    for x in [230.0, 200.0, 170.0, 140.0] {
        let (_, action) = frame(vec![Event::PointerMoved(egui::pos2(x, 24.0))]);
        assert!(action.is_none(), "dragging must not activate a session");
    }
    let (_, action) = frame(pointer(egui::pos2(140.0, 24.0), false));
    assert!(
        action.is_none(),
        "releasing a drag must not activate a session"
    );
    let moved = session_rects(&frame(vec![]).0)[2];
    assert!(
        moved.left() < before.left() - 30.0,
        "drag moves the content: {before:?} -> {moved:?}"
    );
    for _ in 0..15 {
        frame(vec![]);
    }
    assert_eq!(
        session_rects(&frame(vec![]).0)[2],
        moved,
        "release must stop scrolling immediately"
    );
    let rects = session_rects(&frame(vec![]).0);
    let (index, rect) = rects
        .iter()
        .enumerate()
        .find(|(_, r)| r.left() > 45.0 && r.right() < 480.0)
        .unwrap();
    frame(pointer(rect.center(), true));
    let (_, action) = frame(pointer(rect.center(), false));
    assert_eq!(action, Some(Action::Activate(index.to_string())));
    for _ in 0..6 {
        frame(pointer(egui::pos2(420.0, 24.0), true));
        frame(vec![Event::PointerMoved(egui::pos2(100.0, 24.0))]);
        frame(pointer(egui::pos2(100.0, 24.0), false));
        frame(vec![]);
    }
    let last = session_rects(&frame(vec![]).0)[39];
    assert!(
        last.left() >= 6.0 && last.right() <= 482.0,
        "last session must be reachable: {last:?}"
    );
    frame(pointer(last.center(), true));
    let (_, action) = frame(pointer(last.center(), false));
    assert_eq!(action, Some(Action::Activate("39".into())));
}

#[test]
fn floating_result_dot_disappears_when_the_session_is_read() {
    fn has_dot(shape: &egui::Shape, color: egui::Color32) -> bool {
        match shape {
            egui::Shape::Circle(circle) => circle.fill == color,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_dot(shape, color)),
            _ => false,
        }
    }
    for (activity, color) in [
        (Activity::Completed, ai_terminal::chrome::COMPLETED),
        (Activity::Failed, ai_terminal::chrome::FAILED),
    ] {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let icons = Icons::new(&ctx);
        let mut snapshot = Snapshot {
            items: vec![Item {
                id: "one".into(),
                name: "Session".into(),
                agent: Agent::Codex,
                color: None,
                activity,
                unread: true,
            }],
            active: None,
            locale: Locale::En,
        };
        let frame = |snapshot: &Snapshot| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 48.0))),
                    ..Default::default()
                },
                |ui| {
                    floating::body(ui, snapshot, &icons);
                },
            )
        };
        let output = frame(&snapshot);
        assert!(
            output
                .shapes
                .iter()
                .any(|shape| has_dot(&shape.shape, color)),
            "unread {activity:?} must be visible"
        );
        snapshot.items[0].unread = false;
        let output = frame(&snapshot);
        assert!(
            !output
                .shapes
                .iter()
                .any(|shape| has_dot(&shape.shape, color)),
            "read {activity:?} must not retain an unread dot"
        );
        assert_eq!(snapshot.items[0].activity, activity);
    }
}

#[test]
fn floating_opacity_defaults_clamps_and_survives_restart_independently() {
    let old: UiSettings =
        serde_json::from_str(r#"{"ui_scale":1.33,"floating_enabled":true}"#).unwrap();
    assert_eq!(old.normalize().floating_opacity, 1.0);
    for (value, expected) in [
        (f32::NAN, 1.0),
        (f32::INFINITY, 1.0),
        (-2.0, 0.2),
        (0.0, 0.2),
        (0.5, 0.5),
        (2.0, 1.0),
    ] {
        assert_eq!(
            UiSettings {
                floating_opacity: value,
                ..old
            }
            .normalize()
            .floating_opacity,
            expected
        );
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let workspace = Workspace {
        ui: UiSettings {
            floating_opacity: 0.5,
            floating_scale: 0.75,
            ..old
        },
        ..Default::default()
    };
    model::save(&path, &workspace).unwrap();
    let restored = model::load(&path).unwrap();
    assert_eq!(restored.ui, workspace.ui);
}

#[test]
fn live_floating_opacity_updates_background_preserves_avatars_and_click_targets() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    ctx.set_embed_viewports(false);
    let bar = floating::FloatingBar::new(Icons::new(&ctx));
    bar.update(
        Snapshot {
            items: vec![Item {
                id: "opacity".into(),
                name: "Status".into(),
                agent: Agent::Codex,
                color: None,
                activity: Activity::Idle,
                unread: false,
            }],
            ..Default::default()
        },
        &ctx,
    );
    let output = ctx.run_ui(egui::RawInput::default(), |_| {
        bar.show(&ctx, Some([100.0, 80.0]))
    });
    // Reuse the same deferred callback, as an already-open native window does.
    let callback = output.viewport_output[&floating::viewport_id()]
        .viewport_ui_cb
        .clone()
        .unwrap();
    let frame = |events| {
        let mut target = None;
        let output = ctx.run_ui(
            egui::RawInput {
                viewport_id: floating::viewport_id(),
                viewports: [
                    (egui::ViewportId::ROOT, Default::default()),
                    (floating::viewport_id(), Default::default()),
                ]
                .into_iter()
                .collect(),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(96.0, 48.0))),
                events,
                ..Default::default()
            },
            |ui| {
                callback(ui);
                target = ui
                    .ctx()
                    .read_response(egui::Id::new(("floating-session", "opacity")))
                    .map(|r| r.rect);
            },
        );
        (output, target.unwrap())
    };
    let mut target = None;
    for opacity in [1.0, 0.5, 0.2, 1.0] {
        bar.set_opacity(&ctx, opacity);
        frame(vec![]);
        let (out, rect) = frame(vec![]);
        assert_eq!(
            *target.get_or_insert(rect),
            rect,
            "Opacity must not move hit targets"
        );
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.rect.width()>90.0 && (i32::from(r.fill.a())-(255.0*opacity).round() as i32).abs()<=1)));
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Mesh(m) if rect.contains(m.calc_bounds().center()) && m.vertices.iter().all(|v|v.color.a()==255))),"Avatars must remain opaque");
        for pressed in [true, false] {
            frame(pointer(rect.center(), pressed));
        }
        assert!(bar.actions().contains(&Action::Activate("opacity".into())));
    }
    assert_eq!(ctx.zoom_factor(), 1.0);
}
