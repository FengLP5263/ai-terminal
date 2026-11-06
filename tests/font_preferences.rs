use ai_terminal::{
    appearance,
    font_choice::FontChoice,
    model::{self, Workspace},
};

#[test]
fn old_and_future_preferences_keep_sessions_and_default_to_the_accepted_font() {
    for ui in [r#"{"ui_scale":1.1}"#, r#"{"terminal_font":"future_font"}"#] {
        let text = format!(
            r#"{{"sessions":[{{"id":"kept","name":"Example ❤️","directory":".","agent":"Shell","agent_session_id":"resume-kept"}}],"active":"kept","ui":{ui}}}"#
        );
        let workspace: Workspace = serde_json::from_str(&text).unwrap();
        assert_eq!(workspace.ui.terminal_font, FontChoice::JetBrainsMono);
        assert_eq!(workspace.sessions[0].id, "kept");
        assert_eq!(
            workspace.sessions[0].agent_session_id.as_deref(),
            Some("resume-kept")
        );
    }
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("sessions.json");
    for font in FontChoice::ALL {
        let mut workspace = Workspace::default();
        workspace.ui.terminal_font = font;
        model::save(&path, &workspace).unwrap();
        assert_eq!(model::load(&path).unwrap().ui.terminal_font, font);
    }
}

#[test]
fn every_available_family_uses_real_metrics_and_keeps_cjk_and_emoji() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let mut renderer = egui_term::TextRendererSession::new(&ctx);
    for choice in FontChoice::ALL {
        let actual = appearance::set_terminal_font(&ctx, choice);
        assert_eq!(
            actual,
            if appearance::font_available(choice) {
                choice
            } else {
                FontChoice::default()
            }
        );
        let font = appearance::current_terminal_font(&ctx, 14.0);
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let (metrics, baseline) = font.metrics(ui.ctx());
            assert!(metrics.width > 0.0 && metrics.height > 0.0 && baseline > 0.0);
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(30.0, 30.0));
            #[cfg(windows)]
            assert!(font.emoji_shape(ui.ctx(), 14.0, "❤️", rect).is_some());
            ui.label(egui::RichText::new("Aa 0123 中文").font(font.font_type()));
        });
        #[cfg(windows)]
        assert!(
            output.textures_delta.set.iter().any(|(_, delta)| {
                let egui::ImageData::Color(image) = &delta.image;
                image
                    .pixels
                    .iter()
                    .any(|pixel| pixel.a() > 0 && pixel.r() > pixel.g().saturating_add(30))
            }),
            "{} must rasterize a visible color emoji, not an empty tile",
            choice.family()
        );
    }
    renderer.release();
}

#[test]
fn installer_marker_does_not_treat_other_files_as_an_installation() {
    let root = tempfile::tempdir().unwrap();
    assert!(!ai_terminal::shortcut::managed_installation(root.path()));
    let marker = root.path().join(".ai-terminal-installed");
    std::fs::write(&marker, "unrelated data").unwrap();
    assert!(!ai_terminal::shortcut::managed_installation(root.path()));
    std::fs::write(
        &marker,
        "ai-terminal-install-v1\r\nAI-Terminal\r\nfixture\r\n",
    )
    .unwrap();
    assert!(ai_terminal::shortcut::managed_installation(root.path()));
}
