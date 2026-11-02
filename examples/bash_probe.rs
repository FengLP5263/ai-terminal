//! Isolated native Git Bash startup and rendering check; no model or real user profile.
use ai_terminal::{
    appearance,
    bridge::Bridge,
    model::{Agent, SessionSpec},
    shell,
};
use egui_term::{BackendCommand, BackendSettings, PtyEvent, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

struct Probe {
    renderer: egui_term::TextRendererSession,
    _job: platform::ProcessJob,
    terminal: TerminalBackend,
    receiver: mpsc::Receiver<(u64, PtyEvent)>,
    _bridge: Bridge,
    home: tempfile::TempDir,
    generation: String,
    output: PathBuf,
    began: Instant,
    directory_seen: bool,
    ready_frame: Option<u64>,
    capture_sent: bool,
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
        for (_, event) in self.receiver.try_iter().take(64) {
            if let PtyEvent::Title(title) = event
                && shell::directory_from_title(&title, &self.generation).is_some_and(|directory| {
                    directory.canonicalize().ok() == self.home.path().canonicalize().ok()
                })
                && !self.directory_seen
            {
                self.directory_seen = true;
                self.terminal.process_command(BackendCommand::Write(r#"printf '\033[2J\033[H\033[36mGit Bash\033[0m | %s\n\n' "$BASH_VERSION"; printf 'System: %s\nUser startup: %s\nPrompt hooks: %s\n\n' "$MSYSTEM" "$BASH_PROBE_PROFILE" "${#PROMPT_COMMAND[@]}"; alias startup_alias; complete -p startup_command; [[ "$BASH_PROBE_PROFILE" == loaded && "$(command -v codex)" == */bin/codex* ]] && printf '\nBASH_CONFIG_OK\n'; printf '\nNative terminal / 中文输入 / 👋\nBASH_PROBE_READY\n'
"#.bytes().map(|c| if c == b'\n' { b'\r' } else { c }).collect()));
            }
        }
        if self.began.elapsed() > Duration::from_secs(25) {
            std::fs::write(
                self.output.join("result.json"),
                r#"{"ok":false,"timeout":true}"#,
            )
            .unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.label("Git Bash · isolated startup configuration");
        let view = TerminalView::new(ui, &mut self.terminal)
            .set_font(appearance::terminal_font(14.0))
            .set_focus(false);
        ui.add(view);
        let ctx = ui.ctx();
        let text = self.terminal.plain_text();
        if text.lines().any(|line| line.trim() == "BASH_PROBE_READY") {
            let first = *self.ready_frame.get_or_insert(ctx.cumulative_frame_nr());
            if ctx.cumulative_frame_nr().saturating_sub(first) >= 3 && !self.capture_sent {
                self.capture_sent = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            let file = std::fs::File::create(self.output.join("bash.png")).unwrap();
            let mut encoder = png::Encoder::new(file, image.width() as u32, image.height() as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(
                    &image
                        .pixels
                        .iter()
                        .flat_map(|pixel| pixel.to_array())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let ok = self.directory_seen
                && text.lines().any(|line| line.trim() == "BASH_CONFIG_OK")
                && text.contains("demo ~/workspace")
                && !text.contains("command not found");
            std::fs::write(
                self.output.join("result.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "ok":ok, "isolated_home":true, "model_request_sent":false,
                    "cwd_metadata_seen":self.directory_seen, "pty_pid":self.terminal.pty_id(),
                    "width":image.width(), "height":image.height()
                }))
                .unwrap(),
            )
            .unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("bash_probe output-directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let report = output.join("result.json");
    assert!(!report.exists(), "Use a fresh output directory");
    let result = eframe::run_native(
        "ai-terminal Git Bash probe",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_inner_size([960.0, 540.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            let home = tempfile::tempdir().unwrap();
            std::fs::write(home.path().join(".bash_profile"), ". \"$HOME/.bashrc\"\n").unwrap();
            std::fs::write(
                home.path().join(".bashrc"),
                r#"export BASH_PROBE_PROFILE=loaded
PS1='\n\[\e[32m\]demo\[\e[0m\] \[\e[33m\]~/workspace\[\e[0m\]\n$ '
alias startup_alias='printf ALIAS_OK'
complete -W 'alpha beta' startup_command
PROMPT_COMMAND=('true')
"#,
            )
            .unwrap();
            let bridge = Bridge::start(Arc::new(|| {})).unwrap();
            let helper = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("ai-terminal-hook.exe");
            let spec = SessionSpec {
                id: "bash-probe".into(),
                name: "Git Bash".into(),
                directory: home.path().into(),
                agent: Agent::Bash,
                agent_session_id: None,
                color: None,
            };
            let mut plan =
                shell::build(&spec, &bridge, &home.path().join("runtime"), &helper).unwrap();
            plan.environment.insert(
                "HOME".into(),
                home.path().to_string_lossy().replace('\\', "/"),
            );
            let (sender, receiver) = mpsc::channel();
            let terminal = TerminalBackend::new(
                940,
                cc.egui_ctx.clone(),
                sender,
                BackendSettings {
                    shell: plan.executable,
                    args: plan.arguments,
                    env: plan.environment,
                    working_directory: Some(home.path().into()),
                },
            )
            .unwrap();
            let job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                _job: job,
                terminal,
                receiver,
                _bridge: bridge,
                home,
                generation: plan.generation,
                output,
                began: Instant::now(),
                directory_seen: false,
                ready_frame: None,
                capture_sent: false,
            }))
        }),
    );
    if result.is_ok() {
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(report).expect("Missing native report")).unwrap();
        assert_eq!(report["ok"], true, "Native Git Bash probe failed: {report}");
    }
    result
}
