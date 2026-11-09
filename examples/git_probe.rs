//! Native screenshots of production Git controls and a real, isolated repository/PTY.
use ai_terminal::{
    appearance,
    discovery::CliSearch,
    font_choice::FontChoice,
    git::GitClient,
    git_ui::{self, GitView},
    i18n::{self, Language},
};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};

const STATES: [(f32, bool, bool, Language, FontChoice); 12] = [
    (1.0, false, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.0, true, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.03, false, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.03, true, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.14, false, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.14, true, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (2.0, true, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.0, true, true, Language::En, FontChoice::JetBrainsMono),
    (1.0, false, true, Language::En, FontChoice::Hack),
    (1.0, false, false, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.0, true, true, Language::ZhCn, FontChoice::JetBrainsMono),
    (1.0, true, true, Language::En, FontChoice::JetBrainsMono),
];
struct Probe {
    renderer: egui_term::TextRendererSession,
    terminal: TerminalBackend,
    client: GitClient,
    view: GitView,
    output: PathBuf,
    began: Instant,
    since: Instant,
    stage: usize,
    frames: usize,
    pending: bool,
    pid: u32,
    records: Vec<serde_json::Value>,
    font_choice: FontChoice,
    interaction: u8,
    clicked_at: Instant,
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
            self.began.elapsed() < Duration::from_secs(60),
            "Git capture timed out"
        );
        self.client.poll();
        assert!(
            self.client.error.is_none(),
            "Git query failed: {:?}",
            self.client.error
        );
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        input.focused = true;
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        if self.stage >= STATES.len() || !STATES[self.stage].1 || self.frames < 4 {
            input.events.push(egui::Event::PointerGone);
            return;
        }
        let Some(commit) = self
            .client
            .snapshot
            .as_ref()
            .and_then(|s| s.commits.first())
        else {
            return;
        };
        let Some(row) = ctx.read_response(egui::Id::new(("git-commit-row", &commit.id))) else {
            return;
        };
        let pos = row.rect.center();
        if self.interaction < 2 {
            if self.interaction == 1 {
                self.clicked_at = Instant::now();
            }
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: self.interaction == 0,
                modifiers: egui::Modifiers::NONE,
            });
            self.interaction += 1;
        } else if self.stage >= 10 {
            if self.interaction == 2
                && self.clicked_at.elapsed() > Duration::from_millis(250)
                && self
                    .client
                    .inspection
                    .as_ref()
                    .is_some_and(|i| i.data.is_some())
            {
                input
                    .events
                    .push(egui::Event::PointerMoved(pos + egui::vec2(1.0, 0.0)));
                self.interaction = 3;
            }
        } else {
            input.events.push(egui::Event::PointerGone);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage >= STATES.len() {
            return;
        }
        let (scale, history, mut open, language, font_choice) = STATES[self.stage];
        if self.frames == 0 {
            self.view.reset();
            if self.font_choice != font_choice {
                appearance::set_terminal_font(ui.ctx(), font_choice);
                self.font_choice = font_choice;
            }
            ui.ctx().set_zoom_factor(scale);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1100.0, 680.0)));
            i18n::apply(ui.ctx(), language);
            self.view.set_history(history);
        }
        egui::Panel::top("git-probe-summary")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_gray(18))
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                git_ui::header(
                    ui,
                    git_ui::Header {
                        name: "PowerShell ❤️",
                        directory: std::path::Path::new("C:/Projects/repository"),
                        scale,
                        connection: None,
                    },
                    Some(&self.client),
                    &mut open,
                );
            });
        let actions = self.view.panel(ui, Some(&self.client), &mut open);
        assert!(!actions.close && !actions.more && !actions.refresh);
        if let Some(id) = actions.inspect {
            self.client.inspect(&id);
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(14),
            )
            .show(ui, |ui| {
                let terminal = TerminalView::new(ui, &mut self.terminal)
                    .set_focus(false)
                    .set_font(appearance::current_terminal_font(ui.ctx(), 14.0));
                ui.add(terminal);
            });
        self.frames += 1;
        assert_eq!(
            self.terminal.pty_id(),
            self.pid,
            "Panel changes restarted the PTY"
        );
        if self.pending
            && let Some(image) = ui.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            })
        {
            let name = format!(
                "git-{}-{}-{}.png",
                self.stage,
                (scale * 100.0).round() as u32,
                if history { "history" } else { "changes" }
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
            self.records.push(serde_json::json!({"image":name,"scale":scale,"history":history,"open":open,"pty_pid":self.pid,"font":font_choice,"expanded":history,"tooltip":self.stage>=10,"commit_details":self.client.inspection.as_ref().and_then(|i| i.data.as_deref())}));
            self.stage += 1;
            if self.stage < STATES.len() {
                ui.ctx().set_zoom_factor(STATES[self.stage].0);
            }
            self.frames = 0;
            self.pending = false;
            self.interaction = 0;
            self.since = Instant::now();
            if self.stage == STATES.len() {
                std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"real_local_git":true,"model_requests":false,"screenshots":self.records,"snapshot":self.client.snapshot.as_deref()})).unwrap()).unwrap();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if !self.pending
            && self.frames >= 5
            && self.since.elapsed() > Duration::from_millis(600)
            && self.terminal.plain_text().contains("GIT_PROBE_READY")
            && let Some(snapshot) = self.client.snapshot.as_ref()
            && !self.client.loading
            && (!history
                || (self.interaction >= 2
                    && self
                        .client
                        .inspection
                        .as_ref()
                        .is_some_and(|i| i.data.is_some())))
            && (self.stage < 10
                || self
                    .client
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.commits.first())
                    .and_then(|c| {
                        ui.ctx()
                            .read_response(egui::Id::new(("git-commit-row", &c.id)))
                    })
                    .is_some_and(|r| r.is_tooltip_open()))
        {
            assert_eq!(
                (snapshot.added, snapshot.deleted, snapshot.total_files),
                (5, 1, 3)
            );
            assert_eq!(snapshot.branch, "feature/native-git");
            assert_eq!(snapshot.commits.len(), 4);
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn fixture(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    let git = CliSearch::current()
        .git()
        .expect("Git for Windows required");
    let run = |args: &[&str]| {
        let mut command = Command::new(&git);
        command
            .args([
                "-c",
                "user.name=Local Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "NUL")
            .env("GIT_AUTHOR_DATE", "2026-11-09T01:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-11-09T01:00:00Z");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let out = command.output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    assert!(
        !root.join(".git").exists(),
        "Use a new probe output directory"
    );
    run(&["init", "-b", "feature/native-git"]);
    std::fs::write(root.join("terminal.rs"), "first\nold\nthird\n").unwrap();
    run(&["add", "--all"]);
    run(&["commit", "-m", "feat: create native terminal workspace"]);
    for message in [
        "fix: preserve active terminal during layout changes",
        "feat: add branch and line-count summary",
        "feat: show current branch commit history",
    ] {
        if message == "feat: show current branch commit history" {
            std::fs::write(
                root.join("history.md"),
                "Commit details stay local.\nNo model requests.\n",
            )
            .unwrap();
            for (path, content) in [
                ("settings.json", "{\"enabled\":true}\n"),
                ("build.ps1", "Write-Output 'fixture only'\n"),
                (".gitignore", "ignored-output/\n"),
                ("icon.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n"),
            ] {
                std::fs::write(root.join(path), content).unwrap();
            }
            run(&["add", "--all"]);
        }
        run(&[
            "commit",
            "--allow-empty",
            "-m",
            message,
            "-m",
            "Inspect author, date, full commit ID and changed files.\nOnly the isolated fixture repository is read.",
        ]);
    }
    std::fs::write(root.join("terminal.rs"), "first\nnew\nthird\nfourth\n").unwrap();
    std::fs::write(
        root.join("新增说明.md"),
        "Git workspace\nLocal fixture only\nNo model requests\n",
    )
    .unwrap();
    std::fs::write(root.join("icon.bin"), [0, 1, 2, 3]).unwrap();
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("git_probe output-directory"));
    std::fs::create_dir_all(&output).unwrap();
    let root = std::fs::canonicalize(&output).unwrap().join("repository");
    fixture(&root);
    eframe::run_native(
        "ai-terminal Git acceptance",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1100.0, 680.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, Language::ZhCn);
            let (tx, _) = mpsc::channel();
            let terminal=TerminalBackend::new(1199,cc.egui_ctx.clone(),tx,BackendSettings {shell:"powershell.exe".into(),args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),"[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); [Console]::WriteLine('PowerShell | Native Git workspace'); [Console]::WriteLine(''); [Console]::WriteLine('Branch: feature/native-git'); [Console]::WriteLine('Git 面板随当前终端目录更新，收起时终端继续运行。'); [Console]::WriteLine('Local fixture only. No Agent or model requests.'); [Console]::WriteLine(''); [Console]::WriteLine('GIT_PROBE_READY'); Start-Sleep -Seconds 120".into()],..Default::default()})?;
            let mut client = GitClient::new(cc.egui_ctx.clone());
            client.set_directory(Some(&root));
            let pid = terminal.pty_id();
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                terminal,
                client,
                view: Default::default(),
                output,
                began: Instant::now(),
                since: Instant::now(),
                stage: 0,
                frames: 0,
                pending: false,
                pid,
                records: vec![],
                font_choice: FontChoice::default(),
                interaction: 0,
                clicked_at: Instant::now(),
            }))
        }),
    )
}
