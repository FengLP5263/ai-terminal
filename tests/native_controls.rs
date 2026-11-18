use ai_terminal::{
    control::{self, actions::*},
    model::UiSettings,
};
fn parse(args: &[&str]) -> control::ApiResult<Action> {
    match control::cli::parse(args.iter().map(|s| (*s).into()).collect())?.command {
        control::Command::Native { action } => Ok(action),
        _ => panic!("native command expected"),
    }
}
#[test]
fn settings_ranges_are_strict_and_transparency_is_not_opacity() {
    let current = UiSettings::default();
    let patch = SettingsPatch {
        floating_transparency_percent: Some(50.),
        floating_scale_percent: Some(142.),
        ..Default::default()
    };
    let next = patch.apply(current).unwrap();
    assert_eq!(next.floating_opacity, 0.5);
    assert_eq!(next.floating_scale, 1.42);
    assert_eq!(next.ui_scale, current.ui_scale);
    for v in [-1., 100.1, f32::NAN, f32::INFINITY] {
        assert!(
            SettingsPatch {
                floating_transparency_percent: Some(v),
                ..Default::default()
            }
            .apply(current)
            .is_err()
        );
    }
    for v in [0., 100.] {
        assert!(
            SettingsPatch {
                floating_transparency_percent: Some(v),
                ..Default::default()
            }
            .apply(current)
            .is_ok()
        );
    }
    for bad in [
        r#"{"terminal_font":"typo"}"#,
        r#"{"language":"typo"}"#,
        r#"{"sidebar_width":999}"#,
        r#"{"floating_center":true,"floating_position_pixels":[0,0]}"#,
    ] {
        let p: SettingsPatch = serde_json::from_str(bad).unwrap();
        assert!(p.apply(current).is_err(), "{bad}");
    }
    assert!(serde_json::from_str::<SettingsPatch>(r#"{"floating_opactiy":0.5}"#).is_err());
}
#[test]
fn native_parser_covers_ui_domains_and_rejects_ambiguous_options() {
    for args in [
        vec![
            "floating",
            "set",
            "--enabled",
            "true",
            "--transparency",
            "50%",
            "--border",
            "false",
        ],
        vec![
            "settings",
            "set",
            "--font",
            "hack",
            "--language",
            "en",
            "--scale",
            "114",
        ],
        vec!["sidebar", "set", "--width", "210"],
        vec!["panel", "open", "history"],
        vec!["window", "set", "--width", "1000", "--height", "700"],
        vec![
            "file", "list", "--path", "src", "--offset", "10", "--limit", "20",
        ],
        vec!["file", "preview"],
        vec!["image", "zoom", "--percent", "200"],
        vec!["media", "seek", "--seconds", "1.25"],
        vec!["git", "open-diff", "--path", "中文 #1.rs"],
        vec!["terminal", "key", "current", "--key", "ctrl+c"],
        vec![
            "terminal",
            "write",
            "current",
            "--text",
            "你好 😀",
            "--submit",
        ],
        vec!["session", "restart", "current", "--force"],
        vec!["terminal", "scroll", "current", "--lines", "-20"],
        vec!["window", "capture-status"],
    ] {
        let action = parse(&args).unwrap();
        let req = control::Request {
            schema_version: 1,
            request_id: uuid::Uuid::new_v4().to_string(),
            caller_session: None,
            action: control::Command::Native { action },
        };
        let data = serde_json::to_vec(&req).unwrap();
        assert!(
            serde_json::from_slice::<control::Request>(&data).is_ok(),
            "{args:?}"
        );
    }
    for args in [
        vec!["floating", "set", "--enabled", "maybe"],
        vec!["floating", "set", "--x", "1"],
        vec!["settings", "set", "--scale", "NaN"],
        vec!["media", "volume"],
        vec!["terminal", "write", "current", "--text", "a", "--file", "b"],
        vec!["file", "show", "unexpected"],
        vec!["settings", "set", "--floating-opactiy", "50"],
        vec!["terminal", "key", "current", "--key", "magic"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
}
#[test]
fn query_contents_are_never_receipts_and_key_modes_are_preserved() {
    assert!(
        !Action::TerminalRead {
            id: "current".into(),
            offset: 0,
            limit: 1
        }
        .cache_receipt()
    );
    assert!(
        !Action::FileShow {
            content: true,
            offset: 0,
            limit: 1
        }
        .cache_receipt()
    );
    assert!(
        !Action::GitShow {
            section: GitSection::Diff,
            offset: 0,
            limit: 1
        }
        .cache_receipt()
    );
    assert!(
        Action::TerminalWrite {
            id: "current".into(),
            text: "private".into(),
            submit: false
        }
        .cache_receipt()
    );
    assert_eq!(TerminalKey::Up.bytes(true), b"\x1bOA");
    assert_eq!(TerminalKey::Up.bytes(false), b"\x1b[A");
    assert_eq!(TerminalKey::CtrlC.bytes(false), b"\x03");
}
#[cfg(windows)]
#[test]
fn request_fingerprints_use_sha256_without_retaining_input() {
    let digest = fingerprint(b"abc").unwrap();
    assert_eq!(
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_ne!(digest, fingerprint(b"abd").unwrap());
}

#[cfg(windows)]
#[test]
fn newer_cli_preflights_old_manager_without_sending_a_native_mutation() {
    use std::{sync::Arc, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let server = control::ControlServer::start(dir.path(), Arc::new(|| {})).unwrap();
    let path = dir.path().to_owned();
    let client = std::thread::spawn(move || {
        std::process::Command::new(env!("CARGO_BIN_EXE_ai-terminal-cli"))
            .args(["--data-dir", path.to_str().unwrap(), "floating", "show"])
            .output()
            .unwrap()
    });
    let pending = server
        .receiver
        .recv_timeout(Duration::from_secs(4))
        .unwrap();
    assert!(matches!(pending.request.action, control::Command::Info));
    pending
        .reply
        .send(control::Response::new(
            &pending.request.request_id,
            &server.descriptor.instance_id,
            Ok(serde_json::json!({"version":"0.14.0-preview.1"})),
        ))
        .unwrap();
    let result = client.join().unwrap();
    assert_eq!(result.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["error"]["code"], "unsupported_command");
    assert!(server.receiver.try_recv().is_err());
}
