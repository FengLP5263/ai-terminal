//! Independent native bar/main/hint scaling with fixed metadata and owned windows only.
use ai_terminal::{
    appearance,
    floating::{self, Action, FloatingBar, Item, Snapshot},
    git_ui,
    i18n::{self, Language, Locale},
    model::Agent,
    sidebar::Icons,
    status::Activity,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const SCALES: [(f32, f32); 6] = [
    (1.0, 1.0),
    (1.5, 1.0),
    (0.71, 1.0),
    (0.71, 1.5),
    (1.5, 1.5),
    (1.25, 0.75),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    bar: FloatingBar,
    icons: Icons,
    references: Vec<[f32; 4]>,
    output: PathBuf,
    stage: usize,
    frames: usize,
    since: Instant,
    began: Instant,
    pending: bool,
    shots: [bool; 3],
    clicked: u8,
    activated: bool,
    hovered_stage: Option<usize>,
    records: Vec<serde_json::Value>,
}
impl eframe::App for Probe {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        assert!(
            self.began.elapsed() < Duration::from_secs(45),
            "Scale capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(60));
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.stage >= SCALES.len() {
            return;
        }
        let target = if input.viewport_id == egui::ViewportId::ROOT {
            0
        } else if input.viewport_id == floating::viewport_id() {
            1
        } else {
            2
        };
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                if target == 2 {
                    let painted = image.pixels.iter().filter(|p| p.a() != 0).count();
                    assert!(
                        painted * 100 >= image.pixels.len() * 95,
                        "Hint retained a stale clip after zoom"
                    );
                }
                let name = format!(
                    "scale-{}-{}.png",
                    self.stage,
                    ["main", "bar", "hint"][target]
                );
                let mut encoder = png::Encoder::new(
                    std::fs::File::create(self.output.join(&name)).unwrap(),
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
                self.shots[target] = true;
            }
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        if target == 1 && self.frames > 4 {
            input.focused = true;
            input
                .viewports
                .entry(floating::viewport_id())
                .or_default()
                .focused = Some(true);
            {
                let pos =
                    floating::content_transform(ctx, SCALES[self.stage].1) * egui::pos2(76.0, 24.0);
                if self.hovered_stage != Some(self.stage) {
                    input.events.push(egui::Event::PointerMoved(pos));
                    self.hovered_stage = Some(self.stage);
                }
                if self.stage == 5 && self.clicked < 2 {
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: self.clicked == 0,
                        modifiers: egui::Modifiers::NONE,
                    });
                    self.clicked += 1;
                }
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= SCALES.len() {
            return;
        }
        let (main, bar) = SCALES[self.stage];
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(main);
            self.bar.set_scale(ui.ctx(), bar);
            self.since = Instant::now();
        }
        let snapshot = Snapshot {
            groups: Vec::new(),
            items: [
                Agent::Shell,
                Agent::Bash,
                Agent::Cmd,
                Agent::Codex,
                Agent::Claude,
                Agent::QoderCn,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, agent)| Item {
                id: i.to_string(),
                name: "Floating 👩🏽‍💻 ❤️".into(),
                agent,
                color: None,
                activity: Activity::Idle,
                unread: false,
            })
            .collect(),
            active: Some("0".into()),
            locale: Locale::En,
        };
        self.bar.update(snapshot, ui.ctx());
        for action in self.bar.actions() {
            if let Action::Activate(id) = action {
                assert_eq!(id, "1");
                self.activated = true;
            }
        }
        self.bar.show(ui.ctx(), Some([60.0, 60.0]));
        git_ui::header(
            ui,
            git_ui::Header {
                name: "Plain directory",
                directory: std::path::Path::new("C:/Workspace/plain"),
                scale: main,
                connection: None,
            },
            None,
            &mut false,
        );
        ui.add_space(18.0);
        ui.label(format!(
            "Main: {:.0}%  |  Floating: {:.0}%",
            main * 100.0,
            bar * 100.0
        ));
        ui.label("Main zoom changes must preserve floating size and position.");
        self.references.clear();
        let ratio = bar / ui.ctx().zoom_factor();
        ui.horizontal(|ui| {
            for agent in [Agent::Shell, Agent::Bash, Agent::Cmd] {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(32.0, 32.0) * ratio, egui::Sense::hover());
                self.icons.paint(ui.painter(), rect, agent);
                let ppp = ui.ctx().pixels_per_point();
                let side = (rect.width() * ppp).round();
                let min = (rect.center() * ppp - egui::vec2(side / 2.0, side / 2.0)).round();
                let icon = (side * 20.0 / 32.0).round();
                let start = (min + egui::vec2(side / 2.0, side / 2.0)
                    - egui::vec2(icon / 2.0, icon / 2.0))
                .round();
                self.references.push([start.x, start.y, icon, icon]);
            }
        });
        self.frames += 1;
        // Only the probe drives child frames while injecting its synthetic input.
        ui.ctx().request_repaint_of(floating::viewport_id());
        if self.shots.iter().all(|s| *s) {
            let dimensions = native("ai-terminal status").expect("Owned floating window");
            let hint = native("ai-terminal hint").expect("Owned hint window");
            let dpi = dimensions[2] as f32 / 96.0;
            assert!(
                (dimensions[0] as f32 - 296.0 * dpi * bar).abs() <= 1.0,
                "Bar width: {dimensions:?}"
            );
            assert!(
                (dimensions[1] as f32 - 48.0 * dpi * bar).abs() <= 1.0,
                "Bar height: {dimensions:?}"
            );
            self.records.push(serde_json::json!({"main_scale":main,"floating_scale":bar,"bar":dimensions,"hint":hint,"root_zoom":ui.ctx().zoom_factor(),"reference_icons":self.references,"main":native("ai-terminal independent scaling")}));
            self.stage += 1;
            self.frames = 0;
            self.pending = false;
            self.shots = [false; 3];
            if self.stage == SCALES.len() {
                assert!(self.activated, "Scaled avatar did not activate");
                let baseline = &self.records[0];
                for record in &self.records[1..3] {
                    assert_eq!(record["bar"], baseline["bar"]);
                    for dimension in 0..2 {
                        assert!(
                            (record["hint"][dimension].as_i64().unwrap()
                                - baseline["hint"][dimension].as_i64().unwrap())
                            .abs()
                                <= 2,
                            "Hint followed main zoom"
                        );
                    }
                }
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":false,"native_geometry_ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":18,"stages":self.records,"scaled_avatar_activated":self.activated,"model_requests":false,"synthetic_egui_input":true,"system_input_injected":false})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if !self.pending
            && self.frames > 12
            && self.since.elapsed() > Duration::from_millis(750)
            && ui.input(|i| i.raw.viewports.contains_key(&floating::hint_viewport_id()))
            && (self.stage != 5 || self.activated)
        {
            self.pending = true;
            for id in [
                egui::ViewportId::ROOT,
                floating::viewport_id(),
                floating::hint_viewport_id(),
            ] {
                ui.ctx().send_viewport_cmd_to(
                    id,
                    egui::ViewportCommand::Screenshot(egui::UserData::default()),
                );
            }
        }
    }
}
#[cfg(windows)]
fn native(title: &str) -> Option<[i32; 5]> {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    };
    struct Find<'a> {
        title: &'a str,
        handle: HWND,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == std::process::id() {
            let f = unsafe { &mut *(data as *mut Find<'_>) };
            let mut text = [0u16; 128];
            let n = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), 128) };
            if String::from_utf16_lossy(&text[..n as usize]) == f.title {
                f.handle = hwnd;
                return 0;
            }
        }
        1
    }
    let mut find = Find {
        title,
        handle: std::ptr::null_mut(),
    };
    unsafe { EnumWindows(Some(visit), &mut find as *mut _ as LPARAM) };
    if find.handle.is_null() {
        return None;
    }
    let mut outer = RECT::default();
    unsafe { GetWindowRect(find.handle, &mut outer) };
    let mut rect = RECT::default();
    (unsafe { GetClientRect(find.handle, &mut rect) } != 0).then(|| {
        [
            rect.right - rect.left,
            rect.bottom - rect.top,
            unsafe { GetDpiForWindow(find.handle) } as i32,
            outer.left,
            outer.top,
        ]
    })
}
#[cfg(not(windows))]
fn native(_: &str) -> Option<[i32; 5]> {
    None
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(!output.join("result.json").exists());
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal independent scaling",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_position([140.0, 140.0])
                .with_inner_size([720.0, 240.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, Language::En);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                bar: FloatingBar::new(Icons::new(&cc.egui_ctx)),
                icons: Icons::new(&cc.egui_ctx),
                references: vec![],
                output,
                stage: 0,
                frames: 0,
                since: Instant::now(),
                began: Instant::now(),
                pending: false,
                shots: [false; 3],
                clicked: 0,
                activated: false,
                hovered_stage: None,
                records: vec![],
            }))
        }),
    )?;
    verify_avatar_pixels(&verify);
    Ok(())
}

fn verify_avatar_pixels(directory: &std::path::Path) {
    fn image(path: std::path::PathBuf) -> (Vec<u8>, usize, usize) {
        let mut reader =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()))
                .read_info()
                .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        (pixels, info.width as usize, info.height as usize)
    }
    let file = directory.join("result.json");
    let mut report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    let mut comparisons = Vec::new();
    for (stage, data) in report["stages"].as_array().unwrap().iter().enumerate() {
        let (root, rw, rh) = image(directory.join(format!("scale-{stage}-main.png")));
        let (bar, bw, bh) = image(directory.join(format!("scale-{stage}-bar.png")));
        let scale =
            data["floating_scale"].as_f64().unwrap() * data["bar"][2].as_f64().unwrap() / 96.0;
        for (icon, reference) in data["reference_icons"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let reference: Vec<_> = reference
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap().round() as usize)
                .collect();
            let (rx, ry, w, h) = (reference[0], reference[1], reference[2], reference[3]);
            assert!(
                rx + w <= rw && ry + h <= rh,
                "Reference must be fully visible"
            );
            let side = (32.0 * scale).round();
            let size = (side * 20.0 / 32.0).round();
            let x = (((28.0 + 48.0 * icon as f64) * scale - side / 2.0).round() + side / 2.0
                - size / 2.0)
                .round() as i32;
            let y = ((24.0 * scale - side / 2.0).round() + side / 2.0 - size / 2.0).round() as i32;
            let mut best = f64::INFINITY;
            for dx in -2..=2 {
                for dy in -2..=2 {
                    let (x, y) = (x + dx, y + dy);
                    if x < 0 || y < 0 || x as usize + w > bw || y as usize + h > bh {
                        continue;
                    }
                    let mut error = 0.0;
                    for yy in 0..h {
                        for xx in 0..w {
                            let a = ((ry + yy) * rw + rx + xx) * 4;
                            let b = ((y as usize + yy) * bw + x as usize + xx) * 4;
                            for (left, right) in root[a..a + 3].iter().zip(&bar[b..b + 3]) {
                                error += (f64::from(*left) - f64::from(*right)).powi(2);
                            }
                        }
                    }
                    best = best.min(error / (w * h * 3) as f64);
                }
            }
            assert!(
                best < 1.0,
                "Floating icon differs from a direct-size reference: stage={stage} icon={icon} mse={best}"
            );
            comparisons.push(serde_json::json!({"stage":stage,"icon":icon,"rgb_mse":best}));
        }
    }
    report["pixel_comparison"] = serde_json::json!(comparisons);
    report["ok"] = serde_json::json!(true);
    std::fs::write(file, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
