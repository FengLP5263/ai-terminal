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
        }
    }
    fn frame(&mut self, events: Vec<Event>, collapsed: bool) -> Actions {
        self.tick += 1;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 600.0))),
            events,
            time: Some(self.tick as f64 * 0.1),
            ..Default::default()
        };
        let entries: Vec<_> = [
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
            activity: Activity::Idle,
            unread: false,
            hint: String::new(),
        })
        .collect();
        let mut actions = Actions::default();
        let output = self.ctx.run_ui(input, |ui| {
            actions = sidebar::show(ui, &entries, Some("a"), collapsed, 238.0, &self.icons);
        });
        self.output = Some(output);
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
    h.frame(vec![], false);
    h.frame(vec![], false);
    assert!(h.row("a").width() > 180.0);
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
    output.shapes.iter().find_map(|s| find(&s.shape, wanted))
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
        for label in ["PowerShell", "CMD", "Bash"] {
            assert!(text_rect(output, label).is_some(), "missing shell {label}");
        }
        assert!(text_rect(output, "工作目录").is_none());
        assert!(text_rect(output, "Agent").is_none());
        let cmd = text_rect(output, "CMD").unwrap();
        assert_eq!(h.click(cmd.center(), collapsed).create, Some(Agent::Cmd));
    }
}

#[test]
fn sidebar_title_shares_header_center_and_height_at_common_zoom_levels() {
    for zoom in [1.0, 1.5, 2.0] {
        let mut h = Harness::new();
        h.ctx.set_zoom_factor(zoom);
        h.frame(vec![], false);
        h.frame(vec![], false);
        let rects = [
            "sidebar-title",
            ai_terminal::chrome::NEW_TERMINAL_ID,
            "sidebar-toggle",
        ]
        .map(|id| h.ctx.read_response(egui::Id::new(id)).unwrap().rect);
        assert!(text_rect(h.output.as_ref().unwrap(), "AI终端").is_some());
        for rect in rects {
            assert!((rect.center().y - rects[0].center().y).abs() < 0.1);
            assert!((rect.height() - rects[0].height()).abs() < 0.1);
        }
    }
}

#[test]
fn right_click_palette_sets_and_resets_color_without_selecting_another_tab() {
    for collapsed in [false, true] {
        let mut h = Harness::new();
        for wanted in [Some(SessionColor::Purple), None] {
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
