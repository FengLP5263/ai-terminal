#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod platform;

fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--hook") {
        ai_terminal::bridge::report_hook();
        return Ok(());
    }
    let config = args
        .windows(2)
        .find(|a| a[0] == "--data-dir")
        .map(|a| std::path::PathBuf::from(&a[1]))
        .unwrap_or_else(ai_terminal::model::data_directory);
    let smoke = args.iter().any(|a| a == "--smoke-test");
    let idle_benchmark = args.iter().any(|a| a == "--idle-benchmark");
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1160.0, 760.0])
        .with_min_inner_size([720.0, 460.0])
        .with_app_id("ai-terminal");
    #[cfg(feature = "renderer-glow")]
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    #[cfg(all(not(feature = "renderer-glow"), feature = "renderer-wgpu"))]
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(
                eframe::egui_wgpu::WgpuSetupCreateNew {
                    power_preference: wgpu::PowerPreference::LowPower,
                    ..eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle()
                },
            ),
            ..Default::default()
        },
        ..Default::default()
    };
    #[cfg(not(any(feature = "renderer-glow", feature = "renderer-wgpu")))]
    compile_error!("Select renderer-glow or renderer-wgpu");
    let title = format!("ai-terminal {}", env!("CARGO_PKG_VERSION"));
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, config, smoke, idle_benchmark)))),
    )
}
