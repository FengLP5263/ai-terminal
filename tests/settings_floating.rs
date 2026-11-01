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
    let pos = text_rect(&out, "Show floating status bar")
        .unwrap()
        .center();
    for pressed in [true, false] {
        frame(pointer(pos, pressed));
    }
    let (_, prefs) = frame(vec![]);
    assert!(prefs.floating_enabled);
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
    let mut tooltip_seen = false;
    for _ in 0..20 {
        let (out, _) = frame(vec![]);
        if let Some(rect) = text_rect(&out, "Session 2 👩🏽‍💻 · Codex · Needs attention")
        {
            assert!(
                rect.top() >= 0.0 && rect.bottom() <= 48.0,
                "tooltip must fit in its native window: {rect:?}"
            );
            tooltip_seen = true;
        }
    }
    assert!(tooltip_seen, "hover shows the session name and status");
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
