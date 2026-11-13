use ai_terminal::model::{self, Agent, SessionGroup, SessionSpec, Workspace};

fn session(id: &str) -> SessionSpec {
    SessionSpec {
        id: id.into(),
        name: format!("会话 {id} ❤️"),
        directory: ".".into(),
        agent: Agent::Shell,
        agent_session_id: Some(format!("history-{id}")),
        color: None,
    }
}
fn group(id: &str, members: &[&str]) -> SessionGroup {
    SessionGroup {
        id: id.into(),
        name: format!("队伍 {id}"),
        members: members.iter().map(|id| (*id).into()).collect(),
    }
}
#[test]
fn grouping_survives_atomic_reload_without_changing_session_identity_or_order() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("sessions.json");
    let mut workspace = Workspace {
        sessions: vec![session("a"), session("b"), session("c")],
        groups: vec![group("team", &["b", "a"])],
        active: Some("b".into()),
        ..Default::default()
    };
    model::save(&file, &workspace).unwrap();
    let loaded = model::load(&file).unwrap();
    assert_eq!(loaded.groups, workspace.groups);
    assert_eq!(
        loaded
            .sessions
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(
        loaded.sessions[1].agent_session_id.as_deref(),
        Some("history-b")
    );
    workspace.groups.clear();
    model::save(&file, &workspace).unwrap();
    let loaded = model::load(&file).unwrap();
    assert!(loaded.groups.is_empty());
    assert_eq!(loaded.sessions.len(), 3);
    assert_eq!(loaded.active.as_deref(), Some("b"));
}
#[test]
fn old_workspace_and_stale_group_references_are_safe() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("sessions.json");
    std::fs::write(&file, br#"{"sessions":[],"active":null}"#).unwrap();
    assert!(model::load(&file).unwrap().groups.is_empty());
    let mut workspace = Workspace {
        sessions: vec![session("a"), session("b")],
        groups: vec![
            group("one", &["gone", "a", "a"]),
            group("two", &["a", "b"]),
            group("empty", &["gone"]),
            group("one", &["b"]),
        ],
        ..Default::default()
    };
    workspace.groups.push(SessionGroup {
        id: "blank".into(),
        name: "  ".into(),
        members: vec!["b".into()],
    });
    model::save(&file, &workspace).unwrap();
    let loaded = model::load(&file).unwrap();
    assert_eq!(loaded.groups, [group("one", &["a"]), group("two", &["b"])]);
}
#[test]
fn moving_members_enforces_one_group_and_prunes_only_empty_groups() {
    let mut groups = vec![group("one", &["a", "b"]), group("two", &["c"])];
    assert!(!model::assign_group(&mut groups, "a", Some("missing")));
    assert!(!model::assign_group(&mut groups, "a", Some("one")));
    assert!(model::assign_group(&mut groups, "a", Some("two")));
    assert_eq!(groups[0].members, ["b"]);
    assert_eq!(groups[1].members, ["c", "a"]);
    assert!(model::assign_group(&mut groups, "b", None));
    assert_eq!(groups.len(), 1);
    assert_eq!(model::session_group(&groups, "a").unwrap().id, "two");
    assert!(model::assign_group(&mut groups, "a", None));
    assert!(model::assign_group(&mut groups, "c", None));
    assert!(groups.is_empty());
    assert!(!model::assign_group(&mut groups, "c", None));
}
