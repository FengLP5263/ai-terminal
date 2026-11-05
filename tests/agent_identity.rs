use ai_terminal::{
    discovery::CliSearch,
    launch,
    model::{Agent, SessionSpec, Workspace},
};

const IDENTITY_ONLY: [(Agent, &str); 3] = [
    (Agent::Kimi, "kimi"),
    (Agent::ZCode, "zcode"),
    (Agent::Trae, "traecli"),
];

#[test]
fn public_launcher_names_are_discovered_in_extension_order() {
    let root = tempfile::tempdir().unwrap();
    let search = CliSearch::from_directories([root.path().into()]);
    for (agent, stem) in IDENTITY_ONLY {
        assert!(search.find(agent).is_none(), "{}", agent.label());
        std::fs::write(root.path().join(format!("{stem}.bat")), "@echo off").unwrap();
        assert_eq!(
            search.find(agent),
            Some(root.path().join(format!("{stem}.bat")))
        );
        std::fs::write(root.path().join(format!("{stem}.exe")), "fixture").unwrap();
        assert_eq!(
            search.find(agent),
            Some(root.path().join(format!("{stem}.exe")))
        );
    }
    assert!(
        !root.path().join("trae.exe").exists(),
        "the Trae IDE entry must not be treated as its CLI"
    );
}

#[test]
fn agent_arguments_are_scoped_and_do_not_write_user_configuration() {
    let root = tempfile::tempdir().unwrap();
    let runtime = root.path().join("runtime");
    let helper = root.path().join("ai-terminal-hook.exe");
    for (agent, _) in IDENTITY_ONLY {
        let spec = SessionSpec {
            id: "one".into(),
            name: agent.label().into(),
            directory: root.path().into(),
            agent,
            agent_session_id: Some("history".into()),
            color: None,
        };
        let args = launch::agent_arguments(&spec, &runtime, &helper).unwrap();
        if agent == Agent::Trae {
            assert!(args.iter().any(|a| a.starts_with("hooks.SessionStart=")));
            assert!(args.iter().any(|a| a.contains("$env:AI_TERMINAL_HELPER")));
            assert_eq!(&args[args.len() - 2..], &["resume", "history"]);
            assert!(
                !args
                    .iter()
                    .any(|a| a.contains("trusted") || a.contains("bypass"))
            );
        } else {
            assert!(args.is_empty());
        }
    }
    assert!(!runtime.exists());
    for directory in [".kimi-code", ".zcode", ".trae", ".trae-cn"] {
        assert!(!root.path().join(directory).exists());
    }
}

#[test]
fn new_agent_identity_roundtrips_without_inventing_startup() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let workspace = Workspace {
        sessions: IDENTITY_ONLY
            .into_iter()
            .map(|(agent, _)| SessionSpec {
                id: agent.label().into(),
                name: agent.label().into(),
                directory: root.path().into(),
                agent,
                agent_session_id: None,
                color: None,
            })
            .collect(),
        ..Default::default()
    };
    ai_terminal::model::save(&path, &workspace).unwrap();
    let restored = ai_terminal::model::load(&path).unwrap();
    for (agent, _) in IDENTITY_ONLY {
        let session = restored
            .sessions
            .iter()
            .find(|session| session.agent == agent)
            .unwrap();
        assert_eq!(session.name, agent.label());
        assert!(!session.agent.is_shell());
    }
    assert_eq!(Agent::Kimi.label(), "Kimi Code");
    assert_eq!(Agent::ZCode.label(), "ZCode");
    assert_eq!(Agent::Trae.label(), "TRAE");
}
