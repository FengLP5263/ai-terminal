#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod platform;
mod shutdown;
mod window_geometry;

use ai_terminal::updater;

fn app_error(error: std::io::Error) -> eframe::Error {
    eframe::Error::AppCreation(Box::new(error))
}

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
    let (restart_tx, restart_rx) = std::sync::mpsc::sync_channel(1);
    let updater = if smoke || idle_benchmark {
        None
    } else {
        std::fs::create_dir_all(&config).map_err(app_error)?;
        let local = cfg!(feature = "update-preview")
            && args.windows(2).any(|pair| pair[0] == "--data-dir")
            && config.canonicalize().ok()
                != ai_terminal::model::data_directory().canonicalize().ok();
        let source = updater::Source::from_args(&args, local).map_err(app_error)?;
        let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("Cargo version");
        let activation = updater::Install::is_activation(&args);
        match updater::Install::resolve(&config, &args, local) {
            Ok(install) => {
                if install.forward(&args, &version) {
                    return Ok(());
                }
                Some(updater::Updater::new(
                    install, source, version, activation, restart_tx,
                ))
            }
            Err(error) if activation => return Err(app_error(error)),
            Err(_) => None,
        }
    };
    let manager_lease = ai_terminal::control::ManagerLease::acquire(&config).map_err(app_error)?;
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1160.0, 760.0])
        .with_min_inner_size([720.0, 460.0])
        .with_app_id("ai-terminal")
        .with_transparent(true)
        .with_icon(ai_terminal::appearance::window_icon());
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
    let result = eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc, config, smoke, idle_benchmark);
            app.updater = updater;
            Ok(Box::new(app))
        }),
    );
    drop(manager_lease);
    result?;
    if let Ok(staged) = restart_rx.try_recv() {
        staged.restart().map_err(app_error)?;
    }
    Ok(())
}
