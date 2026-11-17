//! Native scale/omission captures: owned repositories, production Git UI and one unchanged PTY.
use ai_terminal::{
    appearance,
    discovery::CliSearch,
    git::GitClient,
    git_ui::{self, GitView},
    i18n::{self, Language},
};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

struct Probe {
    _job: platform::ProcessJob,
    renderer: egui_term::TextRendererSession,
    client: GitClient,
    view: GitView,
    terminal: TerminalBackend,
    lock: Option<fs::File>,
    roots: Vec<PathBuf>,
    output: PathBuf,
    stage: usize,
    frames: usize,
    pending: bool,
    began: Instant,
    since: Instant,
    pid: u32,
    ready: bool,
    records: Vec<serde_json::Value>,
}
impl eframe::App for Probe {
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
            self.began.elapsed() < Duration::from_secs(90),
            "count probe timed out"
        );
        self.client.poll();
        // Keep readiness across narrow/zoomed layouts, which can wrap the marker.
        self.ready |= self.terminal.plain_text().contains("COUNT_PROBE_READY");
        assert!(self.client.error.is_none(), "{:?}", self.client.error);
        ctx.request_repaint_after(Duration::from_millis(40));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = format!("counts-{}.png", self.stage);
                let mut encoder = png::Encoder::new(
                    fs::File::create(self.output.join(&name)).unwrap(),
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
                let snapshot = self.client.snapshot.as_ref().unwrap();
                self.records.push(serde_json::json!({"image":name,"stage":self.stage,"added":snapshot.added,"deleted":snapshot.deleted,"total_files":snapshot.total_files,"displayed_files":snapshot.files.len(),"partial":snapshot.partial,"omitted_files":snapshot.omitted_files,"pty_pid":self.pid}));
                self.stage += 1;
                self.frames = 0;
                self.pending = false;
                self.since = Instant::now();
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == 4 {
            assert_eq!(self.terminal.pty_id(), self.pid);
            fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"real_local_git":true,"same_pty":true,"read_only":true,"model_requests":false,"synthetic_connection_status":true,"user_profiles":false,"screenshots":self.records})).unwrap()).unwrap();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let scale = [1.14, 1.14, 1.0, 2.0][self.stage];
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(scale);
            i18n::apply(
                ui.ctx(),
                if self.stage == 1 {
                    Language::En
                } else {
                    Language::ZhCn
                },
            );
            if self.stage == 3 {
                self.lock = None;
                self.client.refresh();
            } else {
                self.client.set_directory(Some(&self.roots[self.stage]));
            }
        }
        egui::Panel::top("count-header").show(ui, |ui| {
            let mut status = ai_terminal::status::Status::default();
            status.activity = ai_terminal::status::Activity::Working;
            git_ui::header(
                ui,
                git_ui::Header {
                    name: "Git 统计核查",
                    directory: &self.roots[self.stage],
                    scale,
                    connection: Some((ai_terminal::model::Agent::Codex, &status)),
                },
                Some(&self.client),
                &mut true,
            );
        });
        self.view.panel(ui, Some(&self.client), &mut true);
        egui::CentralPanel::default().show(ui, |ui| {
            let terminal = TerminalView::new(ui, &mut self.terminal)
                .set_font(appearance::terminal_font(14.0))
                .set_focus(false);
            ui.add(terminal);
        });
        self.frames += 1;
        assert_eq!(self.terminal.pty_id(), self.pid);
        if !self.pending
            && self.frames >= 8
            && self.since.elapsed() > Duration::from_millis(500)
            && self.ready
            && !self.client.loading
            && let Some(snapshot) = &self.client.snapshot
        {
            let expected = [
                (0, 13, false),
                (0, 10_001, false),
                (0, 2, false),
                (0, 2, false),
            ][self.stage];
            assert_eq!(
                (snapshot.added, snapshot.total_files, snapshot.partial),
                expected
            );
            if self.stage == 1 {
                assert_eq!(snapshot.files.len(), 10_000);
            }

            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn init(root: &Path) {
    fs::create_dir_all(root).unwrap();
    let mut cmd = Command::new(
        CliSearch::current()
            .git()
            .expect("Git for Windows required"),
    );
    cmd.args(["-c", "core.hooksPath=", "init", "-q", "-b", "main"])
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "NUL");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    assert!(cmd.status().unwrap().success());
}
fn main() -> eframe::Result {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("unique output directory required"),
    );
    assert!(!output.exists());
    fs::create_dir_all(&output).unwrap();
    let large = output.join("large-text");
    let many = output.join("many-files");
    let locked = output.join("locked-file");
    for root in [&large, &many, &locked] {
        init(root);
    }
    let body = "line\n".repeat(250_000);
    for n in 0..12 {
        fs::write(large.join(format!("large-{n:02}.txt")), &body).unwrap();
    }
    fs::File::create(large.join("image.bin"))
        .unwrap()
        .set_len(4 * 1024 * 1024)
        .unwrap();
    for n in 0..10_001 {
        fs::write(many.join(format!("file-{n:05}.txt")), "x\n").unwrap();
    }
    fs::write(locked.join("good.txt"), "one\ntwo\n").unwrap();
    fs::write(locked.join("locked.txt"), "one\ntwo\nthree\n").unwrap();
    #[cfg(windows)]
    let lock = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(locked.join("locked.txt"))
            .unwrap()
    };
    #[cfg(not(windows))]
    compile_error!("Native count probe requires Windows.");
    eframe::run_native(
        "ai-terminal Git count verification",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1160.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let (tx, _) = mpsc::channel();
            let terminal=TerminalBackend::new(951,cc.egui_ctx.clone(),tx,BackendSettings {shell:PathBuf::from(std::env::var_os("WINDIR").unwrap()).join("System32/WindowsPowerShell/v1.0/powershell.exe").to_string_lossy().into_owned(),args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),"Write-Output 'COUNT_PROBE_READY · 大文件与大量文件统计 · 终端保持连接'; Start-Sleep -Seconds 120".into()],working_directory:Some(output.clone()),..Default::default()}).unwrap();
            let pid = terminal.pty_id();
            let job = platform::ProcessJob::attach(pid).unwrap();
            Ok(Box::new(Probe {
                _job: job,
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                client: GitClient::new(cc.egui_ctx.clone()),
                view: Default::default(),
                terminal,
                lock: Some(lock),
                roots: vec![large, many, locked.clone(), locked],
                output,
                stage: 0,
                frames: 0,
                pending: false,
                began: Instant::now(),
                since: Instant::now(),
                pid,
                ready: false,
                records: vec![],
            }))
        }),
    )
}
