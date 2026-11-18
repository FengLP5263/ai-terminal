//! Real console CLI -> authenticated IPC -> production App, isolated data and local
//! Agent stand-in. No user sessions, profiles, model requests or system input.
#[path = "../src/app.rs"]
mod app;
#[path = "../src/platform.rs"]
mod platform;
#[path = "../src/shutdown.rs"]
mod shutdown;
#[path = "../src/window_geometry.rs"]
mod window_geometry;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

enum Step {
    Capture(&'static str, mpsc::Sender<()>),
    Minimize(mpsc::Sender<()>),
    Done(Value),
}
struct Probe {
    inner: app::App,
    output: PathBuf,
    steps: mpsc::Receiver<Step>,
    capture: Option<(&'static str, mpsc::Sender<()>)>,
    minimize: Option<mpsc::Sender<()>>,
    finished: bool,
    started: Instant,
}
impl eframe::App for Probe {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.inner.logic(ctx, frame);
        if let Some(ack) = &self.minimize
            && ctx.input(|i| i.viewport().minimized.unwrap_or(false))
        {
            let _ = ack.send(());
            self.minimize = None;
        }
        if let Ok(step) = self.steps.try_recv() {
            match step {
                Step::Capture(name, ack) => {
                    self.capture = Some((name, ack));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                        egui::UserData::default(),
                    ));
                }
                Step::Minimize(ack) => {
                    self.minimize = Some(ack);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                Step::Done(result) => {
                    std::fs::write(
                        self.output.join("result.json"),
                        serde_json::to_vec_pretty(&result).unwrap(),
                    )
                    .unwrap();
                    self.finished = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if self.started.elapsed() > Duration::from_secs(75) {
            panic!("CLI probe exceeded its deadline");
        }
        // Probe-only observation cadence; production control uses event wakes.
        ctx.request_repaint_after(Duration::from_millis(60));
    }
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.finished {
            return;
        }
        self.inner.ui(ui, frame);
        let image = ui.ctx().input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = image
            && let Some((name, ack)) = self.capture.take()
        {
            let file = std::fs::File::create(self.output.join(name)).unwrap();
            let mut encoder = png::Encoder::new(file, image.width() as u32, image.height() as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&bytes)
                .unwrap();
            let _ = ack.send(());
        }
    }
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.inner.on_exit(gl);
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.inner.on_exit();
    }
}
fn cli(output: &Path, arguments: &[&str]) -> (i32, Value) {
    let result = std::process::Command::new(output.join("ai-terminal-cli.exe"))
        .arg("--data-dir")
        .arg(output.join("data"))
        .args(arguments)
        .output()
        .unwrap();
    let value = serde_json::from_slice(&result.stdout).unwrap_or_else(|_| {
        panic!(
            "CLI did not return JSON: {}",
            String::from_utf8_lossy(&result.stderr)
        )
    });
    (result.status.code().unwrap_or(-1), value)
}
fn ok(output: &Path, arguments: &[&str]) -> Value {
    let (code, value) = cli(output, arguments);
    assert_eq!(code, 0, "{arguments:?}: {value}");
    assert_eq!(value["ok"], true);
    value["result"].clone()
}
fn capture(sender: &mpsc::Sender<Step>, name: &'static str) {
    let (ack, ready) = mpsc::channel();
    sender.send(Step::Capture(name, ack)).unwrap();
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
}
fn until(output: &Path, args: &[&str], ready: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let value = ok(output, args);
        if ready(&value) {
            return value;
        }
        assert!(Instant::now() < deadline, "Timed out: {args:?}: {value}");
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn native_driver(output: &Path, leader: &str) -> Value {
    assert_eq!(
        ok(output, &["instance", "capabilities"])["capabilities"]["native_controls"],
        true
    );
    let pid = ok(output, &["session", "show", leader])["process_id"].clone();
    ok(
        output,
        &[
            "floating",
            "set",
            "--enabled",
            "true",
            "--transparency",
            "50",
            "--border",
            "true",
            "--scale",
            "100",
        ],
    );
    assert_eq!(
        ok(output, &["floating", "status"])["floating_transparency_percent"],
        50.0
    );
    ok(output, &["sidebar", "set", "--width", "220"]);
    ok(output, &["sidebar", "hide"]);
    ok(output, &["sidebar", "show"]);
    ok(output, &["window", "minimize"]);
    until(output, &["window", "show"], |v| v["minimized"] == true);
    ok(
        output,
        &["settings", "set", "--language", "en", "--scale", "114"],
    );
    assert_eq!(ok(output, &["session", "show", leader])["process_id"], pid);
    ok(output, &["window", "restore"]);
    until(output, &["window", "show"], |v| v["minimized"] == false);
    ok(output, &["panel", "open", "settings"]);
    ok(output, &["panel", "close", "settings"]);
    assert!(
        !ok(output, &["settings", "fonts"])["fonts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let project = output.join("native-project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(
        project.join("sample.md"),
        "# Native CLI\n\n**Markdown** and 中文 😀\n",
    )
    .unwrap();
    std::fs::write(project.join("large.txt"), "0123456789\n".repeat(240_000)).unwrap();
    image::RgbaImage::from_pixel(64, 48, image::Rgba([30, 140, 220, 255]))
        .save(project.join("image.png"))
        .unwrap();
    let mut wav = Vec::new();
    let data_bytes = 96_000u32;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(data_bytes + 36).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&8_000u32.to_le_bytes());
    wav.extend_from_slice(&16_000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    wav.resize(44 + data_bytes as usize, 0);
    std::fs::write(project.join("sample.wav"), wav).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "sample.md"],
        vec![
            "-c",
            "user.name=CLI Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "Native fixture",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .current_dir(&project)
                .env("GIT_CONFIG_GLOBAL", project.join("no-global-config"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    std::fs::write(
        project.join("sample.md"),
        "# Native CLI updated\n\n**Markdown** and 中文 😀\n",
    )
    .unwrap();
    let created = ok(
        output,
        &[
            "session",
            "create",
            "--name",
            "Native controls fixture",
            "--directory",
            project.to_str().unwrap(),
            "--shell",
            "cmd",
        ],
    );
    let io = created["results"][0]["id"].as_str().unwrap();
    ok(output, &["session", "start", io, "--activate"]);
    let io_pid = ok(output, &["session", "show", io])["process_id"].clone();
    until(output, &["project", "show"], |v| {
        v["root"].as_str() == project.to_str()
    });
    ok(output, &["panel", "open", "files"]);
    let listed = until(output, &["file", "list"], |v| v["loading"] == false);
    assert!(listed["total"].as_u64().unwrap() >= 4);
    assert_eq!(cli(output, &["file", "open", "--path", "../secret"]).0, 2);
    ok(output, &["file", "open", "--path", "sample.md"]);
    until(output, &["file", "show"], |v| v["loading"] == false);
    let read = ok(output, &["file", "read", "--limit", "1"]);
    assert_eq!(read["content"]["items"].as_array().unwrap().len(), 1);
    ok(output, &["file", "source"]);
    ok(output, &["file", "preview"]);
    ok(output, &["file", "open", "--path", "large.txt"]);
    until(output, &["file", "show"], |v| {
        v["loading"] == false && v["document"]["has_next_page"] == true
    });
    ok(output, &["file", "next"]);
    let next = until(output, &["file", "show"], |v| v["loading"] == false);
    assert!(next["document"]["page_offset"].as_u64().unwrap() > 0);
    ok(output, &["file", "previous"]);
    ok(output, &["file", "open", "--path", "image.png"]);
    until(output, &["file", "show"], |v| v["loading"] == false);
    ok(output, &["image", "zoom", "--percent", "200"]);
    ok(output, &["image", "fit"]);
    ok(output, &["file", "open", "--path", "sample.wav"]);
    until(output, &["file", "show"], |v| v["loading"] == false);
    until(output, &["media", "status"], |v| {
        v["duration_seconds"].as_f64().is_some_and(|n| n > 1.)
    });
    ok(output, &["media", "volume", "--percent", "0"]);
    ok(output, &["media", "play"]);
    until(output, &["media", "status"], |v| {
        v["position_seconds"].as_f64().is_some_and(|n| n > 0.1)
    });
    ok(output, &["media", "pause"]);
    ok(output, &["media", "seek", "--seconds", "1.5"]);
    ok(output, &["file", "close"]);
    until(output, &["git", "status"], |v| {
        v["loading"] == false && v["status"]["total_files"].as_u64().is_some_and(|n| n > 0)
    });
    let history = ok(output, &["git", "history"]);
    let sha = history["commits"]["items"][0]["id"].as_str().unwrap();
    ok(output, &["panel", "open", "history"]);
    ok(output, &["git", "inspect", "--commit", sha]);
    until(output, &["git", "commit"], |v| {
        v["loading"] == false && v["commit"].is_object()
    });
    ok(output, &["git", "open-diff", "--path", "sample.md"]);
    until(output, &["git", "diff"], |v| {
        v["loading"] == false && v["lines"].is_object()
    });
    ok(output, &["git", "close-diff"]);
    ok(
        output,
        &[
            "terminal",
            "write",
            io,
            "--text",
            "echo CLI_NATIVE_FIXTURE",
            "--submit",
        ],
    );
    until(output, &["terminal", "read", io], |v| {
        v["items"].as_array().is_some_and(|items| {
            items.iter().any(|s| {
                s.as_str().is_some_and(|s| {
                    s.trim().ends_with("CLI_NATIVE_FIXTURE") && !s.contains("echo ")
                })
            })
        })
    });
    ok(output, &["terminal", "scroll", io, "--offset", "0"]);
    ok(output, &["terminal", "key", io, "--key", "escape"]);
    assert_eq!(ok(output, &["session", "show", io])["process_id"], io_pid);
    assert_eq!(cli(output, &["session", "stop", io]).0, 5);
    ok(output, &["session", "stop", io, "--force"]);
    assert_eq!(ok(output, &["session", "show", io])["running"], false);
    ok(output, &["session", "restart", io]);
    assert_ne!(ok(output, &["session", "show", io])["process_id"], io_pid);
    ok(output, &["session", "acknowledge", io]);
    ok(output, &["session", "close", io, "--force"]);
    ok(output, &["session", "activate", leader]);
    let image = output.join("native-controls.png");
    ok(
        output,
        &["window", "capture", "--output", image.to_str().unwrap()],
    );
    until(output, &["window", "capture-status"], |v| {
        v["state"] == "complete"
    });
    assert!(image.is_file());
    ok(output, &["floating", "hide"]);
    json!({"ok":true,"settings_minimized":true,"live_pid_preserved":true,"float_transparency_percent":50,"files_paged":true,"markdown":true,"image":true,"audio_play_pause_seek":true,"git_readonly":true,"explicit_terminal_io":true,"owned_stop_restart":true,"capture":true,"model_requests":false})
}
fn driver(output: &Path, leader: &str, steps: &mpsc::Sender<Step>) -> Value {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if output.join("data/runtime/control.json").is_file()
            && cli(output, &["instance", "info"]).0 == 0
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(50));
    }
    let info = ok(output, &["instance", "info"]);
    assert!(info["caller_session"].is_null());
    assert_eq!(cli(output, &["session", "show", "current"]).0, 4);
    let original = ok(output, &["workspace", "show"]);
    let plan = json!({"schema_version":1,"operations":[
        {"op":"session.create","key":"frontend","name":"前端 ✨","directory":output,"color":"blue"},
        {"op":"session.create","key":"backend","name":"后端 🛠️","directory":output,"color":"purple"},
        {"op":"group.create","name":"原生 CLI 团队","members":[leader,"$frontend","$backend"]},
        {"op":"session.move","id":"$frontend","after":leader},
        {"op":"session.move","id":"$backend","after":"$frontend"}
    ]});
    let plan_path = output.join("team.json");
    std::fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    let path = plan_path.to_str().unwrap();
    assert_eq!(
        ok(output, &["workspace", "apply", "--file", path, "--dry-run"])["dry_run"],
        true
    );
    assert_eq!(
        ok(output, &["workspace", "show"])["sessions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let request = uuid::Uuid::new_v4().to_string();
    let receipt = ok(
        output,
        &[
            "--request-id",
            &request,
            "workspace",
            "apply",
            "--file",
            path,
            "--if-revision",
            original["revision"].as_str().unwrap(),
        ],
    );
    let again = ok(
        output,
        &[
            "--request-id",
            &request,
            "workspace",
            "apply",
            "--file",
            path,
            "--if-revision",
            original["revision"].as_str().unwrap(),
        ],
    );
    assert_eq!(receipt, again);
    let front = receipt["results"][0]["id"].as_str().unwrap();
    let back = receipt["results"][1]["id"].as_str().unwrap();
    let state = ok(output, &["workspace", "show"]);
    assert_eq!(state["sessions"].as_array().unwrap().len(), 3);
    assert_eq!(state["groups"][0]["members"], json!([leader, front, back]));
    let saved = ai_terminal::model::load(&output.join("data/sessions.json")).unwrap();
    assert_eq!(saved.groups.len(), 1);
    assert_eq!(
        cli(
            output,
            &[
                "workspace",
                "apply",
                "--file",
                path,
                "--if-revision",
                original["revision"].as_str().unwrap()
            ]
        )
        .0,
        5
    );
    capture(steps, "team-created.png");
    let (ack, ready) = mpsc::channel();
    steps.send(Step::Minimize(ack)).unwrap();
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    ok(
        output,
        &["session", "rename", back, "--name", "后台修改已同步"],
    );
    assert_eq!(
        ok(output, &["session", "show", back])["name"],
        "后台修改已同步"
    );
    ok(output, &["session", "activate", leader]);
    ok(
        output,
        &["session", "run", front, "--agent", "codex", "--activate"],
    );
    let caller_path = output.join("fixture-caller.json");
    let deadline = Instant::now() + Duration::from_secs(8);
    while !caller_path.is_file() {
        assert!(
            Instant::now() < deadline,
            "stand-in did not reach the inherited CLI"
        );
        std::thread::sleep(Duration::from_millis(40));
    }
    let caller: Value = serde_json::from_slice(&std::fs::read(&caller_path).unwrap()).unwrap();
    assert_eq!(caller["info"]["result"]["caller_session"], front);
    assert_eq!(caller["info"]["instance_id"], info["instance_id"]);
    assert_eq!(
        cli(output, &["session", "run", front, "--agent", "codex"]).0,
        5
    );
    assert_eq!(cli(output, &["session", "close", front]).0, 5);
    capture(steps, "agent-fixture.png");
    ok(output, &["session", "close", front, "--force"]);
    let pid = caller["pid"].as_u64().unwrap() as u32;
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) {
        assert!(
            Instant::now() < deadline,
            "owned fixture survived session close"
        );
        std::thread::sleep(Duration::from_millis(40));
    }
    ok(output, &["session", "close", back]);
    let native = native_driver(output, leader);
    let final_state = ok(output, &["workspace", "show"]);
    assert_eq!(final_state["sessions"].as_array().unwrap().len(), 1);
    json!({"ok":true,"native_controls":native,"synthetic_agent":true,"model_requests":false,"system_input":false,"checks":["external_instance","current_is_caller","batch_dry_run","batch_commit_and_save","receipt_deduplication","revision_conflict","group_global_order","native_team_screenshot","minimized_background_mutation","explicit_agent_shim_launch","inherited_instance_and_session","live_run_refused","live_close_requires_force","native_agent_screenshot","owned_agent_process_cleanup","unstarted_member_close"],"final_workspace":final_state})
}
#[cfg(windows)]
fn alive(pid: u32) -> bool {
    use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code = 0;
        let active = GetExitCodeProcess(h, &mut code) != 0 && code == 259;
        CloseHandle(h);
        active
    }
}
#[cfg(not(windows))]
fn alive(_pid: u32) -> bool {
    false
}

fn main() -> eframe::Result {
    if std::env::current_exe().unwrap().file_stem().unwrap() == "codex" {
        println!("LOCAL AGENT STAND-IN — no model request or trust approval");
        let result = std::process::Command::new(std::env::var_os("AI_TERMINAL_CLI").unwrap())
            .args(["instance", "info"])
            .output()
            .unwrap();
        let info: Value = serde_json::from_slice(&result.stdout).unwrap();
        let root = PathBuf::from(std::env::var_os("AI_TERMINAL_CLI_PROBE").unwrap());
        std::fs::write(
            root.join("fixture-caller.json"),
            serde_json::to_vec(&json!({"pid":std::process::id(),"info":info})).unwrap(),
        )
        .unwrap();
        std::thread::sleep(Duration::from_secs(60));
        return Ok(());
    }
    let output = std::path::absolute(
        std::env::args_os()
            .nth(1)
            .expect("cli_probe <fresh-output>"),
    )
    .unwrap();
    if !std::env::args_os().any(|a| a == "--native") {
        std::fs::create_dir(&output).unwrap();
        let exe = std::env::current_exe().unwrap();
        let profile = exe.parent().unwrap().parent().unwrap();
        for name in [
            "ai-terminal-hook.exe",
            "ai-terminal-cli.exe",
            "conpty.dll",
            "OpenConsole.exe",
        ] {
            std::fs::copy(profile.join(name), output.join(name)).unwrap();
        }
        let own = output.join("cli-probe.exe");
        std::fs::copy(&exe, &own).unwrap();
        let status = std::process::Command::new(own)
            .arg(&output)
            .arg("--native")
            .status()
            .unwrap();
        assert!(status.success(), "native probe failed");
        let result: Value =
            serde_json::from_slice(&std::fs::read(output.join("result.json")).unwrap()).unwrap();
        assert_eq!(result["ok"], true);
        return Ok(());
    }
    let fixtures = output.join("fixtures");
    std::fs::create_dir(&fixtures).unwrap();
    std::fs::copy(std::env::current_exe().unwrap(), fixtures.join("codex.exe")).unwrap();
    let mut paths = vec![fixtures];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    // Before threads: only this isolated process and its owned children see the fixture.
    unsafe {
        std::env::set_var("PATH", std::env::join_paths(paths).unwrap());
        std::env::set_var("AI_TERMINAL_CLI_PROBE", &output);
    }
    let leader = uuid::Uuid::new_v4().to_string();
    ai_terminal::model::save(
        &output.join("data/sessions.json"),
        &ai_terminal::model::Workspace {
            sessions: vec![ai_terminal::model::SessionSpec {
                id: leader.clone(),
                name: "主协调 ❤️".into(),
                directory: output.clone(),
                agent: ai_terminal::model::Agent::Shell,
                agent_session_id: None,
                color: Some(ai_terminal::model::SessionColor::Green),
            }],
            active: Some(leader.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    let (sender, steps) = mpsc::channel();
    let task_output = output.clone();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| driver(&task_output, &leader, &sender))
            .unwrap_or_else(|_| json!({"ok":false}));
        let _ = sender.send(Step::Done(result));
    });
    eframe::run_native(
        "AI Terminal — native CLI probe",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(ai_terminal::appearance::window_icon())
                .with_inner_size([1100.0, 720.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Probe {
                inner: app::App::new(cc, output.join("data"), false, false),
                output,
                steps,
                capture: None,
                minimize: None,
                finished: false,
                started: Instant::now(),
            }))
        }),
    )
}
