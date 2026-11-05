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
        for point_size in [8.0, 14.0, 15.5, 24.0, 32.0] {
            let ctx = egui::Context::default();
            appearance::setup(&ctx);
            let mut input = egui::RawInput::default();
            input
                .viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .native_pixels_per_point = Some(dpi);
            let font = appearance::terminal_font(point_size);
            let _ = ctx.run_ui(input, |ui| {
                let (size, baseline) = font.metrics(ui.ctx());
                let raster = font.styled_font_at_scale(false, false, dpi);
                assert!((raster.size * dpi - (raster.size * dpi).round()).abs() < 0.001);
                assert!(
                    size.height <= raster.size * 1.6,
                    "fallback line height inflated grid: {size:?}"
                );
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
}

#[test]
fn mixed_terminal_glyphs_share_a_baseline_and_use_real_weight_faces() {
    let ctx = egui::Context::default();
    appearance::setup(&ctx);
    let (tx, _) = mpsc::channel();
    let mut backend=TerminalBackend::new(1293,ctx.clone(),tx,BackendSettings {
        shell:"powershell.exe".into(),
        args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write(([char]27)+'[2J'+([char]27)+'[H'+'ABCxyz 0123 你好 '+([char]27)+'[1m'+'BOLD '+([char]27)+'[0;3m'+'ITALIC '+([char]27)+'[1;3m'+'BOTH'+([char]27)+'[0m'+' DONE'); Start-Sleep -Seconds 20".into()],
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
    let quads: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Mesh(mesh) if mesh.vertices.len() == 4 => Some(mesh.calc_bounds()),
            _ => None,
        })
        .collect();
    assert!(quads.len() > 20, "native glyph atlas was not used");
    let row = quads[0].top();
    for rect in quads {
        assert!(
            (rect.top() - row).abs() < 0.01,
            "mixed glyphs jumped off the common row"
        );
        for v in [rect.left(), rect.top(), rect.right(), rect.bottom()] {
            assert!(
                (v * 1.5 - (v * 1.5).round()).abs() < 0.001,
                "native glyph lies between physical pixels"
            );
        }
    }
}
