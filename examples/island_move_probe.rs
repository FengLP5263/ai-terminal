//! Move only this probe's floating viewport between physical monitors; no Agent/model/input injection.
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
const ZOOMS: [f32; 7] = [1.0, 1.42, 1.42, 0.71, 0.71, 2.0, 1.0];
struct Probe {
    bar: FloatingBar,
    snapshot: Snapshot,
    output: PathBuf,
    monitors: Vec<[i32; 4]>,
    stage: usize,
    since: Instant,
    began: Instant,
    initialized: bool,
    position: Option<[f32; 2]>,
    pending: bool,
    captured: bool,
    root_captured: bool,
    checks: Vec<serde_json::Value>,
    move_step: usize,
    start: [i32; 2],
    held_size: Option<[i32; 2]>,
    stable_samples: usize,
    root_sizes: std::collections::HashMap<usize, [i32; 2]>,
}
impl eframe::App for Probe {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        let child = input.viewport_id == floating::viewport_id();
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let file = std::fs::File::create(self.output.join(format!(
                    "{}-{}.png",
                    if child { "move" } else { "main" },
                    self.stage
                )))
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
                            .flat_map(|p| p.to_array())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();
                if child {
                    self.captured = true;
                } else {
                    self.root_captured = true;
                }
            }
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.began.elapsed() > Duration::from_secs(35) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.stage == ZOOMS.len() {
            return;
        }
        let monitor = self.monitors[(self.stage + 1) % self.monitors.len()];
        if !self.initialized {
            ctx.set_zoom_factor(ZOOMS[self.stage]);
            if self.stage == 0 {
                self.position = Some([
                    self.monitors[0][0] as f32 + 80.0,
                    self.monitors[0][1] as f32 + 80.0,
                ]);
            }
            self.since = Instant::now();
            self.initialized = true;
            self.move_step = 0;
            self.held_size = None;
            self.stable_samples = 0;
        }
        for action in self.bar.actions() {
            if self.since.elapsed() > Duration::from_millis(400)
                && let Action::Position(pos) = action
            {
                self.position = Some(pos);
            }
        }
        self.bar.update(self.snapshot.clone(), ctx);
        self.bar.show(ctx, self.position);
        let elapsed = self.since.elapsed().as_millis();
        let step = ((elapsed.saturating_sub(500)) / 100).min(10) as usize;
        if elapsed >= 500
            && self.move_step == 0
            && let Some(metrics) = owned_metrics()
        {
            self.start = [metrics[0], metrics[1]];
            native_move(None, Some(true), false);
            // Keep native move state active across actual DPI boundaries in
            // both windows. No system mouse input or cursor movement is injected.
            self.move_step = 1;
        }
        if self.move_step > 0 && step + 1 > self.move_step {
            let fraction = step as f32 / 10.0;
            native_move(
                Some([
                    (self.start[0] as f32 * (1.0 - fraction) + (monitor[0] + 100) as f32 * fraction)
                        as i32,
                    (self.start[1] as f32 * (1.0 - fraction) + (monitor[1] + 100) as f32 * fraction)
                        as i32,
                ]),
                None,
                false,
            );
            self.move_step = step + 1;
        }
        if elapsed >= 1600 && self.move_step == 11 {
            // Size repair must wait until release, otherwise it fights the
            // native DPI/mouse-anchor path during movement.
            native_move(None, None, true);
            self.held_size = owned_metrics().map(|m| [m[2], m[3]]);
            self.move_step = 12;
        }
        if elapsed >= 1850 && self.move_step == 12 {
            let held = owned_metrics().map(|m| [m[2], m[3]]);
            self.checks.push(serde_json::json!({"stage":self.stage,"phase":"no_size_repair_while_moving","ok":held.is_some() && held == self.held_size}));
            native_move(None, Some(false), false);
            self.move_step = 13;
        }
        if elapsed >= 2150
            && !self.pending
            && let Some(metrics) = owned_metrics()
        {
            let scale = metrics[4] as f32 / 96.0 * ZOOMS[self.stage];
            let valid = (metrics[0] - (monitor[0] + 100)).abs() <= 3
                && (metrics[1] - (monitor[1] + 100)).abs() <= 3
                && (metrics[2] as f32 - 488.0 * scale).abs() <= 3.0
                && (metrics[3] as f32 - 48.0 * scale).abs() <= 3.0;
            if valid {
                self.stable_samples += 1;
            } else {
                self.stable_samples = 0;
            }
        }
        if (self.stable_samples >= 5 || elapsed >= 4800)
            && !self.pending
            && let Some(metrics) = owned_metrics()
        {
            let scale = metrics[4] as f32 / 96.0 * ZOOMS[self.stage];
            let expected_width =
                488.0_f32.min(((monitor[2] - monitor[0]) as f32 / scale - 16.0).max(1.0));
            let valid = (metrics[2] as f32 - expected_width * scale).abs() <= 3.0
                && (metrics[3] as f32 - 48.0 * scale).abs() <= 3.0
                && metrics[0] >= monitor[0]
                && metrics[0] < monitor[2];
            self.checks.push(serde_json::json!({"stage":self.stage,"zoom":ZOOMS[self.stage],"monitor":monitor,"native":metrics,"expected_logical_width":expected_width,"ok":valid}));
            let root = root_metrics();
            let monitor_index = (self.stage + 1) % self.monitors.len();
            let root_ok = root.is_some_and(|m| {
                let base = self.root_sizes.entry(monitor_index).or_insert([m[2], m[3]]);
                (m[0] - (monitor[0] + 100)).abs() <= 3
                    && (m[1] - (monitor[1] + 300)).abs() <= 3
                    && (m[2] as f32 - 600.0 * m[4] as f32 / 96.0).abs() <= 8.0
                    && (m[3] as f32 - 180.0 * m[4] as f32 / 96.0).abs() <= 8.0
                    && (m[2] - base[0]).abs() <= 2
                    && (m[3] - base[1]).abs() <= 2
            });
            self.checks.push(serde_json::json!({"stage":self.stage,"phase":"main_window","native":root,"ok":root_ok}));
            self.pending = true;
            ctx.send_viewport_cmd_to(
                floating::viewport_id(),
                egui::ViewportCommand::Screenshot(egui::UserData::default()),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if self.captured && self.root_captured {
            self.stage += 1;
            self.initialized = false;
            self.pending = false;
            self.captured = false;
            self.root_captured = false;
            if self.stage == ZOOMS.len() {
                let trace = dpi_trace::report();
                let ok = self.checks.iter().all(|check| check["ok"] == true) && trace["ok"] == true;
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":ok,"monitors":self.monitors,"checks":self.checks,"dpi_trace":trace,"model_requests":false,"native_window_moves":true,"native_move_messages":true,"native_move_state_spans_dpi_changes":true,"physical_mouse_drag":false,"oversized_client_recovery":true})).unwrap()).unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.label("Floating viewport cross-monitor verification (synthetic sessions)");
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal floating move probe",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([600.0, 180.0])
                .with_transparent(true)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            // Winit establishes per-monitor DPI awareness before this callback.
            // Enumerating earlier returns Windows' virtualized coordinates.
            let monitors = monitors();
            assert!(
                monitors.len() >= 2,
                "Two physical monitors required; no simulated pass"
            );
            appearance::setup(&cc.egui_ctx);
            ai_terminal::window_frame::apply_dark(cc);
            let icons = Icons::new(&cc.egui_ctx);
            Ok(Box::new(Probe {
                bar: FloatingBar::new(icons),
                snapshot: Snapshot {
                    items: (0..12)
                        .map(|i| Item {
                            id: format!("s{i}"),
                            name: format!("Session {i}"),
                            agent: if i % 2 == 0 {
                                Agent::Codex
                            } else {
                                Agent::Claude
                            },
                            color: None,
                            activity: Activity::Idle,
                            unread: false,
                        })
                        .collect(),
                    ..Default::default()
                },
                output,
                monitors,
                stage: 0,
                since: Instant::now(),
                began: Instant::now(),
                initialized: false,
                position: None,
                pending: false,
                captured: false,
                root_captured: false,
                checks: vec![],
                move_step: 0,
                start: [0, 0],
                held_size: None,
                stable_samples: 0,
                root_sizes: Default::default(),
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
fn monitors() -> Vec<[i32; 4]> {
    use windows_sys::Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::*,
    };
    unsafe extern "system" fn visit(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
        unsafe {
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info) != 0 {
                (*(data as *mut Vec<[i32; 4]>)).push([
                    info.rcWork.left,
                    info.rcWork.top,
                    info.rcWork.right,
                    info.rcWork.bottom,
                ]);
            }
            1
        }
    }
    let mut result = vec![];
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(visit),
            &mut result as *mut _ as LPARAM,
        );
    }
    result
}

#[cfg(windows)]
fn owned_windows() -> Vec<(windows_sys::Win32::Foundation::HWND, bool)> {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::*,
    };
    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> i32 {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut pid);
        }
        if pid == std::process::id() {
            let mut title = [0u16; 80];
            let len = unsafe { GetWindowTextW(window, title.as_mut_ptr(), 80) };
            let title = String::from_utf16_lossy(&title[..len as usize]);
            if title == "ai-terminal status" || title == "ai-terminal floating move probe" {
                unsafe {
                    (*(data as *mut Vec<(HWND, bool)>))
                        .push((window, title == "ai-terminal status"));
                }
            }
        }
        1
    }
    let mut result = vec![];
    unsafe {
        EnumWindows(Some(visit), &mut result as *mut _ as LPARAM);
    }
    result
}
#[cfg(windows)]
fn window_metrics(child: bool) -> Option<[i32; 5]> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    };
    let (window, _) = owned_windows()
        .into_iter()
        .find(|(_, is_child)| *is_child == child)?;
    let mut outer = RECT::default();
    let mut client = RECT::default();
    if unsafe { GetWindowRect(window, &mut outer) != 0 && GetClientRect(window, &mut client) != 0 }
    {
        Some([
            outer.left,
            outer.top,
            client.right,
            client.bottom,
            unsafe { GetDpiForWindow(window) } as i32,
        ])
    } else {
        None
    }
}
#[cfg(windows)]
fn owned_metrics() -> Option<[i32; 5]> {
    window_metrics(true)
}
#[cfg(windows)]
fn root_metrics() -> Option<[i32; 5]> {
    window_metrics(false)
}
#[cfg(windows)]
fn native_move(position: Option<[i32; 2]>, moving: Option<bool>, distort: bool) {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::*};
    for (window, child) in owned_windows() {
        dpi_trace::install(window, child);
        unsafe {
            if let Some(moving) = moving {
                SendMessageW(
                    window,
                    if moving {
                        WM_ENTERSIZEMOVE
                    } else {
                        WM_EXITSIZEMOVE
                    },
                    0,
                    0,
                );
            }
            if let Some([x, y]) = position {
                SetWindowPos(
                    window,
                    std::ptr::null_mut(),
                    x,
                    y + if child { 0 } else { 200 },
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            if child && distort {
                let mut client = RECT::default();
                if GetClientRect(window, &mut client) != 0 {
                    SetWindowPos(
                        window,
                        std::ptr::null_mut(),
                        0,
                        0,
                        client.right + 32,
                        client.bottom + 8,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }
}
#[cfg(windows)]
mod dpi_trace {
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
    };
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        UI::{Shell::*, WindowsAndMessaging::*},
    };
    #[derive(Default)]
    struct Trace {
        windows: HashMap<usize, bool>,
        moving: HashSet<usize>,
        depth: usize,
        max_depth: usize,
        events: Vec<serde_json::Value>,
        apply_events: Vec<serde_json::Value>,
    }
    thread_local! { static TRACE: RefCell<Trace> = RefCell::new(Trace::default()); }
    const ID: usize = 0xD914;
    pub fn install(window: HWND, child: bool) {
        let fresh = TRACE.with(|s| {
            s.borrow_mut()
                .windows
                .insert(window as usize, child)
                .is_none()
        });
        if fresh {
            assert_ne!(unsafe { SetWindowSubclass(window, Some(event), ID, 0) }, 0);
        }
    }
    unsafe extern "system" fn event(
        window: HWND,
        msg: u32,
        w: usize,
        l: isize,
        id: usize,
        _: usize,
    ) -> isize {
        if msg == WM_NCDESTROY {
            TRACE.with(|s| {
                let mut s = s.borrow_mut();
                s.windows.remove(&(window as usize));
                s.moving.remove(&(window as usize));
            });
            unsafe {
                RemoveWindowSubclass(window, Some(event), id);
            }
        }
        if msg == WM_ENTERSIZEMOVE {
            TRACE.with(|s| {
                s.borrow_mut().moving.insert(window as usize);
            });
        }
        let inspect = msg == WM_DPICHANGED
            && l != 0
            && TRACE.with(|s| s.borrow().moving.contains(&(window as usize)));
        let suggested = inspect.then(|| unsafe { *(l as *const RECT) });
        if inspect {
            TRACE.with(|s| {
                let mut s = s.borrow_mut();
                s.depth += 1;
                s.max_depth = s.max_depth.max(s.depth);
            });
        }
        let mut before = RECT::default();
        if msg == 0x8174 {
            unsafe {
                GetClientRect(window, &mut before);
            }
        }
        let result = unsafe { DefSubclassProc(window, msg, w, l) };
        if msg == 0x8174 {
            let mut after = RECT::default();
            unsafe {
                GetClientRect(window, &mut after);
            }
            TRACE.with(|s| {
                let mut s=s.borrow_mut();
                let moving=s.moving.contains(&(window as usize));
                if s.apply_events.len()<128 {s.apply_events.push(serde_json::json!({"moving":moving,"before":[before.right,before.bottom],"after":[after.right,after.bottom]}));}
            });
        }
        if let Some(suggested) = suggested {
            let mut actual = RECT::default();
            assert_ne!(unsafe { GetWindowRect(window, &mut actual) }, 0);
            TRACE.with(|s| {
                let mut s=s.borrow_mut(); s.depth-=1;
                let child=s.windows[&(window as usize)];
                let error=[actual.left-suggested.left,actual.top-suggested.top,actual.right-suggested.right,actual.bottom-suggested.bottom];
                if s.events.len()<128 {s.events.push(serde_json::json!({"floating":child,"dpi":w&0xffff,"suggested":[suggested.left,suggested.top,suggested.right,suggested.bottom],"actual":[actual.left,actual.top,actual.right,actual.bottom],"edge_error":error,"ok":error.iter().all(|v|v.abs()<=2)}));}
            });
        }
        if msg == WM_EXITSIZEMOVE {
            TRACE.with(|s| {
                s.borrow_mut().moving.remove(&(window as usize));
            });
        }
        result
    }
    pub fn report() -> serde_json::Value {
        TRACE.with(|s| {
            let s=s.borrow();
            serde_json::json!({"ok":s.max_depth<=2 && s.events.len()>=4 && s.events.len()<=super::ZOOMS.len()*4 && s.events.iter().all(|v|v["ok"]==true) && [true,false].iter().all(|kind|s.events.iter().any(|v|v["floating"]==*kind)),"max_nested_dpi":s.max_depth,"dpi_events":s.events,"apply_events":s.apply_events})
        })
    }
}
#[cfg(not(windows))]
fn native_move(_: Option<[i32; 2]>, _: Option<bool>, _: bool) {}
#[cfg(not(windows))]
fn monitors() -> Vec<[i32; 4]> {
    vec![]
}
#[cfg(not(windows))]
fn owned_metrics() -> Option<[i32; 5]> {
    None
}
#[cfg(not(windows))]
fn root_metrics() -> Option<[i32; 5]> {
    None
}
#[cfg(not(windows))]
mod dpi_trace {
    pub fn report() -> serde_json::Value {
        serde_json::json!({"ok":false})
    }
}
