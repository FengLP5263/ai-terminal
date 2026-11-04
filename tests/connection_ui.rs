use ai_terminal::{appearance, connection_ui, model::Agent, status::Status};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
fn text_rect(output: &egui::FullOutput, wanted: &str) -> Option<Rect> {
    fn find(s: &egui::Shape, wanted: &str) -> Option<Rect> {
        match s {
            egui::Shape::Text(t) if t.galley.job.text.contains(wanted) => {
                Some(Rect::from_min_size(t.pos, t.galley.size()))
            }
            egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, wanted)),
            _ => None,
        }
    }
    output.shapes.iter().find_map(|s| find(&s.shape, wanted))
}
#[test]
fn pending_status_explains_the_right_agent_and_copies_only_the_view_command() {
    for agent in [Agent::Codex, Agent::Claude, Agent::QoderCn] {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let status = Status::default();
        let mut tick = 0;
        let mut frame = |events| {
            tick += 1;
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))),
                    time: Some(tick as f64 * 0.1),
                    events,
                    ..Default::default()
                },
                |ui| {
                    connection_ui::show(ui, agent, &status);
                },
            )
        };
        frame(vec![]);
        let output = frame(vec![]);
        let pos = text_rect(&output, "等待任务通知").unwrap().center();
        for pressed in [true, false] {
            frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        let output = frame(vec![]);
        assert!(text_rect(&output, &format!("已识别 {}", agent.label())).is_some());
        let pos = text_rect(&output, "复制 /hooks").unwrap().center();
        let mut copied = false;
        for pressed in [true, false] {
            let output = frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
            copied |= output
                .platform_output
                .commands
                .iter()
                .any(|c| matches!(c,egui::OutputCommand::CopyText(s) if s=="/hooks"));
        }
        assert!(copied);
        assert_eq!(status.activity, ai_terminal::status::Activity::Unknown);
    }
}

#[test]
fn identity_only_agents_explain_the_hook_boundary_without_a_copy_action() {
    for agent in [Agent::Kimi, Agent::ZCode, Agent::Trae] {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let status = Status::default();
        let mut tick = 0;
        let mut frame = |events| {
            tick += 1;
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))),
                    time: Some(tick as f64 * 0.1),
                    events,
                    ..Default::default()
                },
                |ui| {
                    connection_ui::show(ui, agent, &status);
                },
            )
        };
        frame(vec![]);
        let output = frame(vec![]);
        let pos = text_rect(&output, "等待任务通知").unwrap().center();
        for pressed in [true, false] {
            frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        let output = frame(vec![]);
        assert!(text_rect(&output, &format!("已识别 {}", agent.label())).is_some());
        assert!(
            text_rect(&output, "此版本仅识别该 CLI 的头像与启动／退出").is_some(),
            "{} guidance missing",
            agent.label()
        );
        assert!(
            text_rect(&output, "复制 /hooks").is_none(),
            "{} must not offer a hook review command",
            agent.label()
        );
    }
}
