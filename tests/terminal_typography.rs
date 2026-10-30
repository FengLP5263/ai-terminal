#![cfg(windows)]
use ai_terminal::appearance;
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn terminal_metrics_are_integral_physical_pixels_at_all_common_dpi() {
    for dpi in [1.0_f32, 1.25, 1.5, 2.0] {
        let ctx = egui::Context::default();
        appearance::setup(&ctx);
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(dpi);
        let font = appearance::terminal_font(15.0);
        let _ = ctx.run_ui(input, |ui| {
            let (size, baseline) = font.metrics(ui.ctx());
            for value in [size.width, size.height, baseline] {
                assert!(
                    (value * dpi - (value * dpi).round()).abs() < 0.001,
                    "fractional physical pixel at DPI {dpi}: {value}"
                );
            }
            assert!(size.width > 0.0 && baseline > 0.0 && baseline < size.height);
        });
    }
}

#[test]
fn mixed_terminal_glyphs_share_a_baseline_and_use_real_weight_faces() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let (tx, _) = mpsc::channel();
    let mut backend=TerminalBackend::new(1293,ctx.clone(),tx,BackendSettings {
        shell:"powershell.exe".into(),
        args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write(([char]27)+'[2J'+([char]27)+'[H'+'ABCxyz 0123 你好 '+([char]27)+'[1m'+'BOLD '+([char]27)+'[3m'+'SLANT'+([char]27)+'[0m'+' DONE'); Start-Sleep -Seconds 20".into()],
        ..Default::default()
    }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while !backend.plain_text().contains("DONE") {
        assert!(Instant::now() < deadline, "fixture did not render");
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 700.0),
        )),
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(1.5);
    let output = ctx.run_ui(input, |ui| {
        let view = TerminalView::new(ui, &mut backend)
            .set_focus(true)
            .set_font(appearance::terminal_font(15.0));
        ui.add(view);
    });
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text),
            _ => None,
        })
        .collect();
    let reference = texts.iter().find(|t| t.galley.job.text == "A").unwrap();
    let baseline = |t: &egui::epaint::TextShape| {
        t.pos.y + t.galley.rows[0].pos.y + t.galley.rows[0].glyphs[0].pos.y
    };
    for t in &texts {
        assert!(
            (baseline(t) - baseline(reference)).abs() * 1.5 <= 1.01,
            "glyph {:?} jumps off the baseline",
            t.galley.job.text
        );
        assert!(
            (t.pos.x * 1.5 - (t.pos.x * 1.5).round()).abs() < 0.001,
            "glyph origin between physical pixels"
        );
    }
    assert!(
        texts
            .iter()
            .any(|t| t.galley.job.sections[0].format.font_id.family
                == egui::FontFamily::Name("terminal-bold".into()))
    );
    assert!(
        texts
            .iter()
            .any(|t| t.galley.job.sections[0].format.font_id.family
                == egui::FontFamily::Name("terminal-bold-italic".into()))
    );
}
