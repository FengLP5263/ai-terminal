use ai_terminal::{
    appearance,
    model::{Agent, SessionColor},
    sidebar::{self, Actions, Entry, Icons},
    status::Activity,
};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};

struct Harness {
    ctx: egui::Context,
    icons: Icons,
    tick: usize,
    output: Option<egui::FullOutput>,
    color: Option<SessionColor>,
    activity: Activity,
    unread: bool,
    extra: Vec<String>,
    width: f32,
    height: f32,
    active: &'static str,
    groups: Vec<ai_terminal::model::SessionGroup>,
}
impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let icons = Icons::new(&ctx);
        Self {
            ctx,
            icons,
            tick: 0,
            output: None,
            color: None,
            activity: Activity::Idle,
            unread: false,
            extra: Vec::new(),
            width: 238.0,
            height: 600.0,
            active: "a",
            groups: Vec::new(),
        }
    }
    fn frame(&mut self, events: Vec<Event>, collapsed: bool) -> Actions {
        self.tick += 1;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(900.0, self.height),
            )),
            events,
            time: Some(self.tick as f64 * 0.1),
            ..Default::default()
        };
        let mut entries: Vec<_> = [
            ("a", "一号", Agent::Codex),
            ("b", "二号", Agent::Claude),
            ("c", "三号", Agent::Shell),
        ]
        .into_iter()
        .map(|(id, name, agent)| Entry {
            id,
            name,
            directory: std::path::Path::new("C:/项目"),
            agent,
            color: if id == "b" { self.color } else { None },
            activity: if id == "a" {
                self.activity
            } else {
                Activity::Idle
            },
            unread: id == "a" && self.unread,
            hint: String::new(),
        })
        .collect();
        entries.extend(self.extra.iter().map(|id| Entry {
            id,
            name: id,
            directory: std::path::Path::new("C:/Projects"),
            agent: Agent::Shell,
            color: None,
            activity: Activity::Idle,
            unread: false,
            hint: String::new(),
        }));
        let mut actions = Actions::default();
        let output = self.ctx.run_ui(input, |ui| {
            actions = sidebar::show_with_groups(
                ui,
                &entries,
                Some(self.active),
                collapsed,
                self.width,
                &self.icons,
                &self.groups,
            );
        });
        self.output = Some(output);
        if !collapsed {
            self.width = actions.width;
        }
        actions
    }
    fn row(&self, id: &str) -> Rect {
        self.ctx
            .read_response(egui::Id::new(("session-row", id)))
            .expect("row drawn")
            .rect
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
    fn click(&mut self, pos: Pos2, collapsed: bool) -> Actions {
        self.frame(Self::pointer(pos, true), collapsed);
        self.frame(Self::pointer(pos, false), collapsed)
    }
}

#[test]
fn collapsed_sidebar_geometry_settles_near_the_scrollbar_threshold() {
    for (count, height, zoom) in [
        (7, 550.0, 1.0),
        (8, 600.0, 0.71),
        (10, 720.0, 1.21),
        (12, 840.0, 1.42),
    ] {
        let mut h = Harness::new();
        h.ctx.set_zoom_factor(zoom);
        h.height = height * zoom;
        h.extra = (3..count).map(|i| format!("session-{i}")).collect();
        for _ in 0..8 {
            h.frame(vec![], true);
        }
        let expected = h.row("a");
        for frame in 0..24 {
            h.frame(vec![], true);
            assert_eq!(
                h.row("a"),
                expected,
                "idle layout oscillation: sessions={count}, height={height}, frame={frame}"
            );
            assert!(
                h.output
                    .as_ref()
                    .unwrap()
                    .viewport_output
                    .values()
                    .all(|v| !v.repaint_delay.is_zero()),
                "idle scrollbar must not create a continuous repaint loop"
            );
        }
        for pointer in [egui::pos2(58.0, 200.0), egui::pos2(400.0, 200.0)] {
            for _ in 0..12 {
                h.frame(vec![Event::PointerMoved(pointer)], true);
                assert_eq!(h.row("a"), expected, "hover must not change rail geometry");
            }
        }
    }
}

#[test]
fn sidebar_scrollbar_stays_thin_and_clear_of_rows_during_hover_wheel_and_drag() {
    fn handle(shape: &egui::Shape, row_right: f32, panel_right: f32) -> Option<Rect> {
        match shape {
            egui::Shape::Rect(shape)
                if shape.rect.left() >= row_right - 0.1
                    && shape.rect.right() <= panel_right
                    && shape.rect.width() < 18.0
                    && shape.rect.height() > 24.0
                    && shape.fill != egui::Color32::TRANSPARENT =>
            {
                Some(shape.rect)
            }
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .find_map(|shape| handle(shape, row_right, panel_right)),
            _ => None,
        }
    }
    fn bar(h: &Harness, width: f32) -> Rect {
        h.output
            .as_ref()
            .unwrap()
            .shapes
            .iter()
            .find_map(|shape| handle(&shape.shape, h.row("a").right(), width))
            .expect("visible scrollbar")
    }
    for collapsed in [false, true] {
        for zoom in [1.0, 1.29] {
            let mut h = Harness::new();
            h.ctx.set_zoom_factor(zoom);
            h.extra = (0..30).map(|i| format!("session-{i}")).collect();
            let mut width = 0.0;
            for _ in 0..4 {
                width = h.frame(vec![], collapsed).width;
            }
            let first = h.row("a");
            if collapsed {
                assert!((first.width() - first.height()).abs() < 0.01, "{first:?}");
            }
            assert!(
                !h.output.as_ref().unwrap().shapes.iter().any(|shape| handle(
                    &shape.shape,
                    first.right(),
                    width
                )
                .is_some()),
                "idle scrollbar must be hidden"
            );
            let pointer = egui::pos2(width - 2.5, first.top() + 20.0);
            for _ in 0..6 {
                h.frame(vec![Event::PointerMoved(pointer)], collapsed);
            }
            let idle = bar(&h, width);
            assert!(
                idle.left() > first.right(),
                "scrollbar must have its own gutter"
            );
            assert!(
                (width - idle.right()).abs() <= 1.6,
                "bar must sit beside the divider: {idle:?}, divider={width}"
            );
            let pointer = idle.center();
            for _ in 0..6 {
                h.frame(vec![Event::PointerMoved(pointer)], collapsed);
            }
            assert!(
                bar(&h, width).width() <= 3.01,
                "hover must not expand the scrollbar"
            );
            h.frame(
                vec![
                    Event::PointerMoved(first.center()),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, -3.0),
                        modifiers: Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ],
                collapsed,
            );
            for _ in 0..4 {
                h.frame(vec![], collapsed);
            }
            assert!(
                h.row("a").top() < first.top(),
                "wheel must scroll: collapsed={collapsed} zoom={zoom} first={first:?} now={:?} pointer={:?}",
                h.row("a"),
                h.ctx.input(|i| i.pointer.hover_pos())
            );
            let scrolled = h.row("a").top();
            let thumb = bar(&h, width);
            assert!(thumb.width() <= 3.01);
            h.frame(Harness::pointer(thumb.center(), true), collapsed);
            h.frame(
                vec![Event::PointerMoved(thumb.center() + egui::vec2(0.0, 70.0))],
                collapsed,
            );
            let action = h.frame(
                Harness::pointer(thumb.center() + egui::vec2(0.0, 70.0), false),
                collapsed,
            );
            h.frame(vec![], collapsed);
            assert!(
                h.row("a").top() < scrolled,
                "thumb drag: collapsed={collapsed} zoom={zoom} thumb={thumb:?} before={scrolled} now={:?} width={} action={:?}",
                h.row("a"),
                action.width,
                action.select
            );
            assert!(action.select.is_none() && action.reorder.is_none());
            assert!(bar(&h, width).width() <= 3.01);
            for _ in 0..6 {
                h.frame(
                    vec![Event::PointerMoved(egui::pos2(width + 80.0, 300.0))],
                    collapsed,
                );
            }
            assert!(
                !h.output.as_ref().unwrap().shapes.iter().any(|shape| handle(
                    &shape.shape,
                    h.row("a").right(),
                    width
                )
                .is_some()),
                "scrollbar must disappear after pointer leaves"
            );
            assert_ne!(
                h.ctx.style_of(egui::Theme::Dark).spacing.scroll.bar_width,
                3.0,
                "sidebar style must remain local"
            );
        }
    }
}

#[test]
fn sidebar_resize_uses_the_divider_outside_the_scrollbar_and_preserves_limits() {
    let mut h = Harness::new();
    h.extra = (0..30).map(|i| format!("session-{i}")).collect();
    h.frame(vec![], false);
    let before = h.row("a").width();
    let grab = egui::pos2(h.width + 2.0, 350.0);
    h.frame(Harness::pointer(grab, true), false);
    h.frame(
        vec![Event::PointerMoved(grab + egui::vec2(60.0, 0.0))],
        false,
    );
    h.frame(Harness::pointer(grab + egui::vec2(60.0, 0.0), false), false);
    h.frame(vec![], false);
    assert!(h.width > 290.0 && h.row("a").width() > before + 50.0);
    let grab = egui::pos2(h.width + 2.0, 350.0);
    h.frame(Harness::pointer(grab, true), false);
    h.frame(
        vec![Event::PointerMoved(grab + egui::vec2(700.0, 0.0))],
        false,
    );
    h.frame(
        Harness::pointer(grab + egui::vec2(700.0, 0.0), false),
        false,
    );
    assert_eq!(h.width, ai_terminal::model::UiSettings::MAX_SIDEBAR_WIDTH);
}

#[test]
fn card_click_edit_and_drag_produce_distinct_actions() {
    let mut h = Harness::new();
    h.frame(vec![], false);
    let b = h.row("b");
    assert_eq!(h.click(b.center(), false).select.as_deref(), Some("b"));
    let a = h.row("a");
    let edit = h.click(egui::pos2(a.right() - 15.0, a.center().y), false);
    assert_eq!(edit.rename.as_deref(), Some("a"));
    assert!(edit.select.is_none(), "edit must not also activate the row");
    let c = h.row("c");
    h.frame(Harness::pointer(a.center(), true), false);
    h.frame(
        vec![Event::PointerMoved(a.center() + egui::vec2(0.0, 12.0))],
        false,
    );
    let destination = egui::pos2(c.center().x, c.bottom() - 8.0);
    h.frame(vec![Event::PointerMoved(destination)], false);
    let dropped = h.frame(Harness::pointer(destination, false), false);
    assert_eq!(dropped.reorder, Some(("a".into(), "c".into(), true)));
}

#[test]
fn collapsed_rail_keeps_clickable_agent_rows_and_expands_again() {
    let mut h = Harness::new();
    h.frame(vec![], false);
    h.frame(vec![], false);
    assert!(h.row("a").width() > 180.0);
    h.frame(vec![], true);
    let collapsed = h.frame(vec![], true);
    assert!(
        h.row("a").width() < 55.0,
        "row {:?}, panel {}",
        h.row("a"),
        collapsed.width
    );
    let b = h.row("b");
    assert_eq!(h.click(b.center(), true).select.as_deref(), Some("b"));
    assert!(
        (b.width() - b.height()).abs() < 0.01,
        "collapsed session must be square: {b:?}"
    );
    h.frame(vec![], false);
    h.frame(vec![], false);
    assert!(h.row("a").width() > 180.0);
}

#[test]
fn expanded_cards_keep_equal_side_insets_with_and_without_overflow() {
    for zoom in [0.71, 1.0, 1.42, 2.0] {
        for count in [0, 30] {
            let mut h = Harness::new();
            h.ctx.set_zoom_factor(zoom);
            h.extra = (0..count).map(|i| format!("extra-{i}")).collect();
            for _ in 0..6 {
                h.frame(vec![], false);
            }
            let row = h.row("a");
            let tolerance = 1.0 / h.ctx.pixels_per_point() + 0.01;
            assert!((row.left() - 8.0).abs() <= tolerance);
            assert!(
                (h.width - row.right() - 8.0).abs() <= tolerance,
                "zoom {zoom}, extra {count}: row {row:?}, panel {}",
                h.width
            );
        }
    }
}

#[test]
fn collapsed_toolbar_matches_session_tiles_and_keeps_a_compact_pitch() {
    for zoom in [0.71, 1.0, 1.42, 2.0] {
        let mut h = Harness::new();
        h.ctx.set_zoom_factor(zoom);
        h.frame(vec![], true);
        h.frame(vec![], true);
        let session = h.row("a");
        let tolerance = 1.0 / h.ctx.pixels_per_point() + 0.01;
        let controls = [
            "settings-button",
            ai_terminal::chrome::NEW_TERMINAL_ID,
            "sidebar-toggle",
        ]
        .map(|id| h.ctx.read_response(egui::Id::new(id)).unwrap().rect);
        for rect in controls {
            assert_eq!(rect.size(), session.size());
            assert!(
                (rect.center().x - session.center().x).abs() <= tolerance,
                "zoom {zoom}: control {rect:?}, session {session:?}"
            );
        }
        assert!((controls[1].top() - controls[0].bottom() - 4.0).abs() <= tolerance);
        assert!((controls[2].top() - controls[1].bottom() - 4.0).abs() <= tolerance);
        assert!((h.row("b").top() - session.bottom() - 4.0).abs() <= tolerance);
    }
}

fn text_rect(output: &egui::FullOutput, wanted: &str) -> Option<Rect> {
    fn find(shape: &egui::Shape, wanted: &str) -> Option<Rect> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == wanted => {
                Some(Rect::from_min_size(text.pos, text.galley.size()))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, wanted)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .rev()
        .find_map(|s| find(&s.shape, wanted))
}
#[test]
fn new_session_is_a_shell_menu_without_directory_or_agent_selection() {
    for collapsed in [false, true] {
        let mut h = Harness::new();
        h.frame(vec![], collapsed);
        let button = h
            .ctx
            .read_response(egui::Id::new(ai_terminal::chrome::NEW_TERMINAL_ID))
            .expect("new terminal control")
            .rect;
        h.click(button.center(), collapsed);
        h.frame(vec![], collapsed);
        let output = h.output.as_ref().unwrap();
        for label in ["PowerShell", "CMD", "Git Bash"] {
            assert!(text_rect(output, label).is_some(), "missing shell {label}");
        }
        assert!(text_rect(output, "工作目录").is_none());
        assert!(text_rect(output, "Agent").is_none());
        let cmd = text_rect(output, "CMD").unwrap();
        assert_eq!(h.click(cmd.center(), collapsed).create, Some(Agent::Cmd));
    }
}

#[test]
fn settings_replaces_title_and_aligns_with_header_controls_at_all_zoom_levels() {
    for zoom in [1.0, 1.5, 2.0] {
        let mut h = Harness::new();
        h.ctx.set_zoom_factor(zoom);
        h.frame(vec![], false);
        h.frame(vec![], false);
        let rects = [
            "settings-button",
            ai_terminal::chrome::NEW_TERMINAL_ID,
            "sidebar-toggle",
        ]
        .map(|id| h.ctx.read_response(egui::Id::new(id)).unwrap().rect);
        assert!(text_rect(h.output.as_ref().unwrap(), "AI终端").is_none());
        assert!(h.click(rects[0].center(), false).settings);
        for rect in rects {
            assert!((rect.center().y - rects[0].center().y).abs() < 0.1);
            assert!((rect.height() - rects[0].height()).abs() < 0.1);
        }
    }
}

#[test]
fn hovered_and_selected_cards_share_one_fill_without_a_separate_outline() {
    fn card(h: &Harness) -> egui::epaint::RectShape {
        fn find(shape: &egui::Shape, rect: Rect) -> Option<egui::epaint::RectShape> {
            match shape {
                egui::Shape::Rect(shape) if shape.rect == rect => Some(shape.clone()),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, rect)),
                _ => None,
            }
        }
        h.output
            .as_ref()
            .unwrap()
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, h.row("b")))
            .expect("card background")
    }
    fn outlined(h: &Harness) -> bool {
        fn find(shape: &egui::Shape, rect: Rect) -> bool {
            match shape {
                egui::Shape::Rect(shape) => {
                    shape.rect.center().distance(rect.center()) < 0.1
                        && (shape.rect.width() - rect.width()).abs() < 2.0
                        && shape.stroke != egui::Stroke::NONE
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(|shape| find(shape, rect)),
                _ => false,
            }
        }
        h.output
            .as_ref()
            .unwrap()
            .shapes
            .iter()
            .any(|shape| find(&shape.shape, h.row("b")))
    }
    for collapsed in [false, true] {
        for color in std::iter::once(None).chain(SessionColor::ALL.into_iter().map(Some)) {
            let mut h = Harness::new();
            h.color = color;
            h.frame(
                vec![Event::PointerMoved(egui::pos2(800.0, 500.0))],
                collapsed,
            );
            let normal = card(&h);
            h.active = "b";
            h.frame(vec![], collapsed);
            let selected = card(&h);
            assert!(!outlined(&h), "selected card has no separate outline");
            h.active = "a";
            let pos = h.row("b").center();
            h.frame(vec![Event::PointerMoved(pos)], collapsed);
            let hovered = card(&h);
            assert!(!outlined(&h), "hovered card has no separate outline");
            assert_eq!(hovered.fill, selected.fill, "hover and selected: {color:?}");
            assert_ne!(
                normal.fill, selected.fill,
                "normal remains subdued: {color:?}"
            );
            for state in [normal, selected, hovered] {
                assert_eq!(
                    state.stroke,
                    egui::Stroke::NONE,
                    "no contrasting card outline"
                );
            }
        }
    }
}

#[test]
fn right_click_palette_sets_and_resets_color_without_selecting_another_tab() {
    for collapsed in [false, true] {
        let mut h = Harness::new();
        for wanted in SessionColor::ALL.into_iter().map(Some).chain([None]) {
            h.frame(vec![], collapsed);
            let pos = h.row("b").center();
            for pressed in [true, false] {
                h.frame(
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: PointerButton::Secondary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                    collapsed,
                );
            }
            h.frame(vec![], collapsed);
            let menu = text_rect(h.output.as_ref().unwrap(), "会话颜色").unwrap();
            h.click(menu.center(), collapsed);
            h.frame(vec![], collapsed);
            let swatches: Vec<_> = std::iter::once(None)
                .chain(SessionColor::ALL.into_iter().map(Some))
                .map(|color| {
                    h.ctx
                        .read_response(egui::Id::new(("session-color", "b", color)))
                        .unwrap()
                        .rect
                })
                .collect();
            assert!(
                swatches
                    .iter()
                    .all(|rect| (rect.center().y - swatches[0].center().y).abs() < 0.1),
                "all seven choices must stay on one row"
            );
            let swatch = h
                .ctx
                .read_response(egui::Id::new(("session-color", "b", wanted)))
                .expect("palette swatch available")
                .rect;
            let actions = h.click(swatch.center(), collapsed);
            assert_eq!(actions.color, Some(("b".into(), wanted)));
            assert!(
                actions.select.is_none(),
                "color changes must not activate the tab"
            );
            h.color = wanted;
            h.frame(
                vec![Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                collapsed,
            );
        }
    }
}

#[test]
fn result_indicator_disappears_after_reading_in_both_sidebar_sizes() {
    fn result_dot(shape: &egui::Shape, color: egui::Color32) -> bool {
        match shape {
            egui::Shape::Circle(circle) => circle.fill == color,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| result_dot(shape, color)),
            _ => false,
        }
    }
    for collapsed in [false, true] {
        for (activity, color) in [
            (Activity::Completed, ai_terminal::chrome::COMPLETED),
            (Activity::Failed, ai_terminal::chrome::FAILED),
        ] {
            let mut h = Harness::new();
            h.activity = activity;
            h.unread = true;
            h.frame(vec![], collapsed);
            assert!(
                h.output
                    .as_ref()
                    .unwrap()
                    .shapes
                    .iter()
                    .any(|s| result_dot(&s.shape, color))
            );
            h.unread = false;
            h.frame(vec![], collapsed);
            assert!(
                !h.output
                    .as_ref()
                    .unwrap()
                    .shapes
                    .iter()
                    .any(|s| result_dot(&s.shape, color)),
                "a viewed {activity:?} must not retain an unread dot"
            );
            assert_eq!(h.activity, activity);
        }
    }
}

#[test]
fn drag_card_follows_grab_point_and_rows_make_space_without_committing_early() {
    for collapsed in [false, true] {
        for zoom in [0.71, 1.0, 1.42, 2.0] {
            let mut h = Harness::new();
            h.ctx.set_zoom_factor(zoom);
            for _ in 0..3 {
                h.frame(vec![], collapsed);
            }
            let a = h.row("a");
            let b = h.row("b");
            let c = h.row("c");
            let grab = egui::vec2(12.0, 11.0);
            let press = a.min + grab;
            h.frame(Harness::pointer(press, true), collapsed);
            h.frame(
                vec![Event::PointerMoved(press + egui::vec2(0.0, 15.0))],
                collapsed,
            );
            let pointer = c.min + grab;
            let actions = h.frame(vec![Event::PointerMoved(pointer)], collapsed);
            assert!(
                egui::DragAndDrop::has_any_payload(&h.ctx),
                "drag not started"
            );
            h.frame(vec![], collapsed);
            assert!(actions.reorder.is_none());
            assert!(actions.select.is_none());
            assert_eq!(
                h.row("a"),
                c,
                "the source leaves a full-size destination gap"
            );
            assert_eq!(h.row("b"), a);
            assert_eq!(h.row("c"), b);
            let ghost = Rect::from_min_size(pointer - grab, a.size());
            assert!(
                h.output.as_ref().unwrap().shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::Shape::Rect(card) if card.rect == ghost
                    && card.fill == ai_terminal::chrome::SELECTED)
                }),
                "full card must follow the pointer at its original grab point: {ghost:?}"
            );
            // A stationary pointer must not feed the shifted card geometry back into ordering.
            for _ in 0..8 {
                h.frame(vec![], collapsed);
                assert_eq!(h.row("a"), c);
                assert_eq!(h.row("b"), a);
            }
            assert!(!h.output.as_ref().unwrap().shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("C:/项目"))
            }), "a hover tooltip must not cover the card during dragging");
            let actions = h.frame(Harness::pointer(pointer, false), collapsed);
            assert_eq!(actions.reorder, Some(("a".into(), "c".into(), true)));
            assert!(actions.select.is_none() && actions.rename.is_none());
        }
    }
}

#[test]
fn drag_can_move_up_or_cancel_without_opening_a_session() {
    for collapsed in [false, true] {
        for cancel in ["escape", "outside", "none"] {
            let mut h = Harness::new();
            h.frame(vec![], collapsed);
            let a = h.row("a");
            let c = h.row("c");
            h.frame(Harness::pointer(c.center(), true), collapsed);
            h.frame(
                vec![Event::PointerMoved(c.center() - egui::vec2(0.0, 15.0))],
                collapsed,
            );
            h.frame(vec![Event::PointerMoved(a.center())], collapsed);
            h.frame(vec![], collapsed);
            assert_eq!(h.row("c"), a);
            let pointer = if cancel == "outside" {
                egui::pos2(600.0, 250.0)
            } else {
                a.center()
            };
            if cancel == "escape" {
                h.frame(
                    vec![Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                    collapsed,
                );
            }
            let actions = h.frame(Harness::pointer(pointer, false), collapsed);
            assert_eq!(
                actions.reorder,
                (cancel == "none").then(|| ("c".into(), "a".into(), false))
            );
            assert!(actions.select.is_none() && actions.rename.is_none());
            h.frame(vec![], collapsed);
            h.frame(vec![], collapsed);
            assert_eq!(h.row("a"), a);
            assert_eq!(h.row("c"), c);
            assert!(!egui::DragAndDrop::has_any_payload(&h.ctx));
        }
    }
}

#[test]
fn held_card_scrolls_long_lists_at_the_edge_and_stops_after_cancel() {
    for collapsed in [false, true] {
        let mut h = Harness::new();
        h.extra = (0..30).map(|i| format!("extra-{i}")).collect();
        for _ in 0..4 {
            h.frame(vec![], collapsed);
        }
        let a = h.row("a");
        h.frame(Harness::pointer(a.center(), true), collapsed);
        h.frame(
            vec![Event::PointerMoved(a.center() + egui::vec2(0.0, 15.0))],
            collapsed,
        );
        let edge = egui::pos2(a.center().x, h.height - 15.0);
        h.frame(vec![Event::PointerMoved(edge)], collapsed);
        let before = h.row("extra-0").top();
        for _ in 0..24 {
            h.frame(vec![], collapsed);
        }
        assert!(
            h.row("extra-0").top() < before - 10.0,
            "held card should scroll the overflowing list"
        );
        let outside = egui::pos2(600.0, 250.0);
        assert!(
            h.frame(Harness::pointer(outside, false), collapsed)
                .reorder
                .is_none()
        );
        for _ in 0..8 {
            h.frame(vec![], collapsed);
        }
        let settled = h.row("extra-0");
        for _ in 0..8 {
            h.frame(vec![], collapsed);
            assert_eq!(h.row("extra-0"), settled);
            assert!(
                h.output
                    .as_ref()
                    .unwrap()
                    .viewport_output
                    .values()
                    .all(|v| !v.repaint_delay.is_zero()),
                "a cancelled drag must not leave an idle repaint loop"
            );
        }
    }
}

#[test]
fn session_context_menu_groups_without_selecting_or_closing_a_terminal() {
    use sidebar::GroupAction;
    for collapsed in [false, true] {
        for choice in ["新建分组…", "开发队", "移出分组", "重命名分组…", "解散分组"]
        {
            let mut h = Harness::new();
            h.groups.push(ai_terminal::model::SessionGroup {
                id: "team".into(),
                name: "开发队".into(),
                members: vec!["b".into(), "c".into()],
            });
            h.frame(vec![], collapsed);
            let pos = h.row("b").center();
            for pressed in [true, false] {
                h.frame(
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: PointerButton::Secondary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                    collapsed,
                );
            }
            h.frame(vec![], collapsed);
            let menu = text_rect(h.output.as_ref().unwrap(), "分组").unwrap();
            h.click(menu.center(), collapsed);
            h.frame(vec![], collapsed);
            let item = text_rect(h.output.as_ref().unwrap(), choice).unwrap();
            let actions = h.click(item.center(), collapsed);
            assert!(
                actions.select.is_none() && actions.close.is_none() && actions.rename.is_none()
            );
            match (choice, actions.group.unwrap()) {
                ("新建分组…", GroupAction::Create(id)) => assert_eq!(id, "b"),
                ("开发队", GroupAction::Assign(id, target)) => {
                    assert_eq!(id, "b");
                    assert_eq!(target.as_deref(), Some("team"));
                }
                ("移出分组", GroupAction::Assign(id, target)) => {
                    assert_eq!(id, "b");
                    assert_eq!(target, None);
                }
                ("重命名分组…", GroupAction::Rename(id))
                | ("解散分组", GroupAction::Dissolve(id)) => {
                    assert_eq!(id, "team")
                }
                _ => panic!("unexpected group menu action"),
            }
        }
    }
}

#[test]
fn group_submenus_keep_one_line_at_each_zoom_and_language() {
    use ai_terminal::i18n::{self, Language};
    for language in [Language::ZhCn, Language::En] {
        for zoom in [0.75, 1.0, 1.33, 2.0] {
            let mut h = Harness::new();
            h.ctx.set_zoom_factor(zoom);
            i18n::apply(&h.ctx, language);
            let long_name = "很长的项目队伍名称 / A very long project team name ".repeat(8);
            h.groups = vec![
                ai_terminal::model::SessionGroup {
                    id: "team".into(),
                    name: "聚合团队".into(),
                    members: vec!["b".into()],
                },
                ai_terminal::model::SessionGroup {
                    id: "long".into(),
                    name: long_name.clone(),
                    members: vec!["c".into()],
                },
            ];
            for _ in 0..3 {
                h.frame(vec![], false);
            }
            let pos = h.row("b").center();
            for pressed in [true, false] {
                h.frame(
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: PointerButton::Secondary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                    false,
                );
            }
            h.frame(vec![], false);
            let menu = text_rect(
                h.output.as_ref().unwrap(),
                if language == Language::En {
                    "Group"
                } else {
                    "分组"
                },
            )
            .unwrap();
            h.click(menu.center(), false);
            h.frame(vec![], false);
            let commands = if language == Language::En {
                [
                    "New group…",
                    "Remove from group",
                    "Rename group…",
                    "Ungroup sessions",
                ]
            } else {
                ["新建分组…", "移出分组", "重命名分组…", "解散分组"]
            };
            fn find<'a>(
                shape: &'a egui::Shape,
                wanted: &str,
            ) -> Option<&'a egui::epaint::TextShape> {
                match shape {
                    egui::Shape::Text(t) if t.galley.job.text == wanted => Some(t),
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, wanted)),
                    _ => None,
                }
            }
            let parent_commands = if language == Language::En {
                ["Rename", "Session color", "Close session"]
            } else {
                ["编辑名称", "会话颜色", "关闭会话"]
            };
            for label in commands
                .into_iter()
                .chain(parent_commands)
                .chain(["聚合团队", long_name.as_str()])
            {
                let text = h
                    .output
                    .as_ref()
                    .unwrap()
                    .shapes
                    .iter()
                    .find_map(|shape| find(&shape.shape, label))
                    .unwrap_or_else(|| panic!("missing {label} at {zoom:?}"));
                assert_eq!(text.galley.rows.len(), 1, "wrapped {label} at {zoom:?}");
                assert!(
                    text.galley.size().x <= 200.0,
                    "menu text exceeded fixed width"
                );
            }
        }
    }
}

#[test]
fn group_badges_are_monochrome_and_do_not_label_session_roles() {
    let mut h = Harness::new();
    h.groups.push(ai_terminal::model::SessionGroup {
        id: "team".into(),
        name: "聚合团队".into(),
        members: vec!["b".into(), "a".into()],
    });
    h.frame(vec![], false);
    h.frame(vec![], false);
    let output = h.output.as_ref().unwrap();
    let names: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(t) if t.galley.job.text == "聚合团队" => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(names.len(), 2);
    assert_eq!(
        names[0].galley.job.sections[0].format.color,
        names[1].galley.job.sections[0].format.color
    );
    assert!(text_rect(output, "主").is_none());
    assert!(text_rect(output, "L").is_none());
    assert_eq!(
        names[0].galley.job.sections[0].format.color,
        egui::Color32::BLACK
    );
    let white_badges=output.shapes.iter().filter(|shape| matches!(&shape.shape,egui::Shape::Rect(r) if r.fill==egui::Color32::WHITE && r.rect.height()<=20.0)).count();
    assert_eq!(white_badges, 2);
    for text in &names {
        assert_eq!(text.galley.rows.len(), 1);
        assert!(text.galley.size().x < 100.0);
        let badge = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(r)
                    if r.fill == egui::Color32::WHITE && r.rect.contains(text.pos) =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .expect("white name badge");
        assert!((text.pos.x - badge.left() - 6.0).abs() < 0.1);
        assert!((badge.right() - text.pos.x - text.galley.size().x - 6.0).abs() < 0.1);
    }
    let badge_color = names[0].galley.job.sections[0].format.color;
    h.groups[0].name = "重命名队伍".into();
    h.frame(vec![], false);
    let renamed = h
        .output
        .as_ref()
        .unwrap()
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "重命名队伍" => {
                Some(t.galley.job.sections[0].format.color)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(renamed, badge_color);
    h.frame(vec![], true);
    h.frame(vec![], true);
    assert!(text_rect(h.output.as_ref().unwrap(), "主").is_none());
    assert!(text_rect(h.output.as_ref().unwrap(), "重").is_some());
    assert!(text_rect(h.output.as_ref().unwrap(), "重命名队伍").is_none());
}
