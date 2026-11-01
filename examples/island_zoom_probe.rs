//! Native viewport zoom regression probe. Synthetic sessions only; no model requests or user input.
use ai_terminal::{
    appearance,
    floating::{self, Action, FloatingBar, Item, Snapshot},
    model::Agent,
    sidebar::Icons,
    status::Activity,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Probe {
    bar: FloatingBar,
    snapshot: Snapshot,
    output: PathBuf,
    began: Instant,
    stage: usize,
    samples: Vec<(f32, i32, i32, i32, i32, i32, i32)>,
    target_physical_x: Option<i32>,
    dpi_debug: Vec<serde_json::Value>,
    position: Option<[f32; 2]>,
    previous_zoom: Option<f32>,
    shots: usize,
    shot_requested: [bool; 2],
}
impl eframe::App for Probe {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn raw_input_hook(&mut self, _input_ctx: &egui::Context, input: &mut egui::RawInput) {
        if input.viewport_id != floating::viewport_id() {
            return;
        }
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let file =
                    std::fs::File::create(self.output.join(format!("zoom-{}.png", self.shots)))
                        .unwrap();
                let mut encoder = png::Encoder::new(
                    std::io::BufWriter::new(file),
                    image.width() as u32,
                    image.height() as u32,
                );
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(
                        &image
                            .pixels
                            .iter()
                            .flat_map(|c| c.to_array())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();
                self.shots += 1;
            }
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let elapsed = self.began.elapsed().as_secs_f32();
        const ZOOMS: [f32; 5] = [0.71, 1.0, 1.5, 0.5, 1.0];
        let deadline = 1.8 + self.stage as f32 * 1.5;
        let shot_index = match self.stage {
            0 => Some(0),
            2 => Some(1),
            _ => None,
        };
        if let Some(index) = shot_index
            && !self.shot_requested[index]
            && elapsed > deadline - 0.6
        {
            ctx.send_viewport_cmd_to(
                floating::viewport_id(),
                egui::ViewportCommand::Screenshot(egui::UserData::default()),
            );
            self.shot_requested[index] = true;
        }
        let screenshot_done = shot_index.is_none_or(|index| self.shots > index);
        if self.stage < ZOOMS.len()
            && elapsed > deadline
            && screenshot_done
            && let Some((width, height, center_error, top, work_left, work_right)) = owned_metrics()
        {
            let child =
                ctx.input(|input| input.raw.viewports.get(&floating::viewport_id()).cloned());
            self.dpi_debug.push(serde_json::json!({
                "root_ppp": ctx.pixels_per_point(),
                "child_native_ppp": child.as_ref().and_then(|v| v.native_pixels_per_point),
                "child_outer": child.as_ref().and_then(|v| v.outer_rect).map(|r| [r.left(), r.top(), r.right(), r.bottom()]),
                "saved": self.position,
            }));
            self.samples.push((
                ctx.zoom_factor(),
                width,
                height,
                center_error,
                top,
                work_left,
                work_right,
            ));
            self.stage += 1;
            if self.stage < ZOOMS.len() {
                ctx.set_zoom_factor(ZOOMS[self.stage]);
            }
        }
        let zoom = ctx.zoom_factor();
        if self.previous_zoom.is_none()
            && let Some(x) = self.target_physical_x
        {
            self.position = Some([x as f32, 80.0]);
        }
        let changed = self.previous_zoom.is_some_and(|old| old != zoom);
        for action in self.bar.actions() {
            if let Action::Position(pos) = action
                && !changed
            {
                self.position = Some(pos);
            }
        }
        if self.previous_zoom.is_none() || changed {
            let saved = self.position;
            self.position =
                Some(floating::centered_geometry(ctx, self.snapshot.items.len(), saved).0);
        }
        self.bar.update(self.snapshot.clone(), ctx);
        self.bar.show(ctx, self.position);
        self.previous_zoom = Some(zoom);
        if self.stage == ZOOMS.len() && self.shots >= 2 {
            let base = self.samples[1];
            let valid = self.samples.iter().zip(ZOOMS).all(|(sample, expected)| {
                (sample.0 - expected).abs() < 0.001
                    && (sample.1 as f32 - base.1 as f32 * expected).abs() <= 3.0
                    && (sample.2 as f32 - base.2 as f32 * expected).abs() <= 3.0
                    && sample.3.abs() <= 6
                    && (sample.4 - self.samples[0].4).abs() <= 3
                    && self
                        .target_physical_x
                        .is_none_or(|x| x >= sample.5 && x < sample.6)
            });
            let result = serde_json::json!({"ok":valid,"samples":self.samples,"screenshots":self.shots,"dpi_debug":self.dpi_debug,"real_model_requests":false});
            std::fs::write(
                self.output.join("result.json"),
                serde_json::to_vec_pretty(&result).unwrap(),
            )
            .unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.stage += 1;
        }
        if elapsed > 25.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Floating viewport zoom verification (synthetic sessions)");
        });
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let verify = output.clone();
    let target_physical_x = std::env::args()
        .nth(2)
        .map(|s| s.parse::<i32>().expect("physical monitor x"));
    eframe::run_native(
        "ai-terminal zoom probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([900.0, 600.0])
                .with_transparent(true)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            cc.egui_ctx.set_zoom_factor(0.71);
            let icons = Icons::new(&cc.egui_ctx);
            let snapshot = Snapshot {
                items: (0..12)
                    .map(|i| Item {
                        id: format!("s{i}"),
                        name: format!("Session {i}"),
                        agent: Agent::Shell,
                        color: None,
                        activity: Activity::Idle,
                        unread: false,
                    })
                    .collect(),
                ..Default::default()
            };
            Ok(Box::new(Probe {
                bar: FloatingBar::new(icons),
                snapshot,
                output,
                began: Instant::now(),
                stage: 0,
                samples: Vec::new(),
                target_physical_x,
                dpi_debug: Vec::new(),
                position: Some([80.0, 80.0]),
                previous_zoom: None,
                shots: 0,
                shot_requested: [false; 2],
            }))
        }),
    )?;
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(verify.join("result.json")).expect("probe result"))
            .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    Ok(())
}
#[cfg(windows)]
fn owned_metrics() -> Option<(i32, i32, i32, i32, i32, i32)> {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
        },
        UI::WindowsAndMessaging::*,
    };
    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> i32 {
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            if pid == std::process::id() {
                let mut title = [0u16; 80];
                let n = GetWindowTextW(window, title.as_mut_ptr(), 80);
                if String::from_utf16_lossy(&title[..n as usize]) == "ai-terminal status" {
                    let mut client = RECT::default();
                    let mut outer = RECT::default();
                    let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
                    let mut info = MONITORINFO {
                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if IsWindowVisible(window) != 0
                        && GetClientRect(window, &mut client) != 0
                        && GetWindowRect(window, &mut outer) != 0
                        && GetMonitorInfoW(monitor, &mut info) != 0
                    {
                        let center_error =
                            outer.left + outer.right - info.rcWork.left - info.rcWork.right;
                        *(data as *mut Option<(i32, i32, i32, i32, i32, i32)>) = Some((
                            client.right - client.left,
                            client.bottom - client.top,
                            center_error,
                            outer.top,
                            info.rcWork.left,
                            info.rcWork.right,
                        ));
                    }
                }
            }
            1
        }
    }
    let mut result = None;
    unsafe {
        EnumWindows(Some(visit), &mut result as *mut _ as LPARAM);
    }
    result
}
#[cfg(not(windows))]
fn owned_metrics() -> Option<(i32, i32, i32, i32, i32, i32)> {
    None
}
