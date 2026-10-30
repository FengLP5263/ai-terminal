use ai_terminal::{
    appearance,
    model::Agent,
    sidebar::{self, Actions, Entry, Icons},
    status::Activity,
};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};

struct Harness {
    ctx: egui::Context,
    icons: Icons,
    tick: usize,
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
            activity: Activity::Idle,
            unread: false,
            running: true,
            resumable: false,
            hint: String::new(),
        })
        .collect();
        let mut actions = Actions::default();
        let _ = self.ctx.run_ui(input, |ui| {
            actions = sidebar::show(ui, &entries, Some("a"), collapsed, 238.0, &self.icons);
        });
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
