//! Native project explorer captures: isolated fixtures, production views, one owned PowerShell PTY.
use ai_terminal::{
    appearance,
    git_ui::{self, GitView},
    i18n::{self, Language},
    project_files::FileClient,
    project_ui::{self, FileView},
};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

const CASES: &[(Language, f32, Option<&str>)] = &[
    (Language::ZhCn, 1.0, None),
    (Language::ZhCn, 1.0, Some("README.md")),
    (Language::En, 1.0, Some("src/main.rs")),
    (Language::ZhCn, 1.03, Some("package.json")),
    (Language::En, 1.5, Some("scripts/demo.ps1")),
    (Language::ZhCn, 2.0, Some("docs/使用说明.md")),
    (Language::En, 1.0, Some("empty.txt")),
    (Language::ZhCn, 1.0, Some("scripts/legacy.ps1")),
    (Language::En, 1.0, Some("image.bin")),
    (Language::ZhCn, 1.0, Some("large.dat")),
    (Language::En, 1.0, Some("invalid.txt")),
    (Language::ZhCn, 1.0, None),
];
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

struct Probe {
    _job: platform::ProcessJob,
    renderer: egui_term::TextRendererSession,
    client: FileClient,
    view: FileView,
    git: GitView,
    terminal: TerminalBackend,
    root: PathBuf,
    output: PathBuf,
    stage: usize,
    frames: usize,
    pending: bool,
    began: Instant,
    since: Instant,
    records: Vec<serde_json::Value>,
    initial_pid: u32,
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
            self.began.elapsed() < Duration::from_secs(65),
            "project explorer capture timeout"
        );
        self.client.poll();
        ctx.request_repaint_after(Duration::from_millis(40));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = format!("project-{}.png", self.stage);
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
                self.records.push(serde_json::json!({"image":name,"language":format!("{:?}",CASES[self.stage].0),"zoom":CASES[self.stage].1,"path":CASES[self.stage].2,"pty_pid":self.terminal.pty_id(),"lines":self.client.file.as_ref().and_then(|f|f.data.as_ref()).map(|d|d.content.lines.len())}));
                self.stage += 1;
                self.frames = 0;
                self.pending = false;
                self.since = Instant::now();
            }
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.stage == CASES.len() {
            assert_eq!(self.terminal.pty_id(), self.initial_pid);
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":self.records,"pty_pid":self.initial_pid,"same_pty":true,"model_requests":false,"user_profiles":false,"read_only":true,"fixture_ready":true})).unwrap()).unwrap();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let (language, zoom, path) = CASES[self.stage];
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(zoom);
            i18n::apply(ui.ctx(), language);
            if let Some(path) = path {
                if let Some(parent) = std::path::Path::new(path)
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                {
                    self.view.expand(parent, &mut self.client);
                }
                self.client.open_file(std::path::Path::new(path));
            } else {
                self.client.close_file();
            }
        }
        if self.frames == 1 {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(1160.0, 760.0) / ui.ctx().zoom_factor(),
            ));
        }
        let ctx = ui.ctx().clone();
        egui::Panel::top("project-header").show(ui, |ui| {
            git_ui::header_with_files(
                ui,
                git_ui::Header {
                    name: i18n::tr(&ctx, "项目文件预览", "Project file preview"),
                    directory: &self.root,
                    scale: zoom,
                    connection: None,
                },
                None,
                &mut false,
                &mut true,
            );
        });
        let actions = project_ui::panel(
            ui,
            &mut self.client,
            &mut self.view,
            None,
            &mut self.git,
            &mut false,
            &mut true,
        );
        assert!(actions.file.is_none());
        if let Some(file) = &self.client.file {
            project_ui::document(ui, file, Some(&self.root));
        } else {
            let view =
                TerminalView::new(ui, &mut self.terminal).set_font(appearance::terminal_font(14.0));
            ui.add(view);
        }
        self.frames += 1;
        let ready = self.terminal.plain_text().contains("PROJECT_READY")
            && self
                .client
                .directories
                .get(std::path::Path::new(""))
                .is_some_and(|s| s.data.is_some())
            && self.client.file.as_ref().is_none_or(|f| !f.loading);
        if !self.pending
            && ready
            && self.frames > 8
            && self.since.elapsed() > Duration::from_millis(450)
        {
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(!output.join("result.json").exists());
    let root = output.join("demo-project");
    std::fs::create_dir(&root).unwrap();
    for folder in ["src", "scripts", "docs"] {
        std::fs::create_dir(root.join(folder)).unwrap();
    }
    for (path, text) in [
        (
            "README.md",
            "# 项目文件预览 ❤️\n\n> 原生只读浏览，不执行文件。\n\n- 文件夹按需展开\n- 代码保持语法配色\n- 返回原来的终端 👩🏽‍💻\n",
        ),
        (
            "src/main.rs",
            "// Rust file preview\nfn main() {\n    let name = \"project\";\n    println!(\"Hello {name}\");\n}\n",
        ),
        (
            "package.json",
            "{\n  \"name\": \"project-preview\",\n  \"version\": \"1.0.0\",\n  \"enabled\": true,\n  \"count\": 42\n}\n",
        ),
        (
            "scripts/demo.ps1",
            "# PowerShell preview\nparam([string]$Name)\n$ErrorActionPreference = 'Stop'\nif ($Name) {\n    Write-Host \"Hello $Name\"\n}\n",
        ),
        (
            "docs/使用说明.md",
            "# 文件查看 👩🏽‍💻 ❤️\n\n这是中文与彩色 emoji 的只读预览。\n\n```json\n{ \"mode\": \"preview\" }\n```\n\n最后一行：继续使用原来的终端。\n",
        ),
        ("empty.txt", ""),
        (".env.example", "MODE=preview\n"),
    ] {
        std::fs::write(root.join(path), text).unwrap();
    }
    let mut legacy = vec![0xff, 0xfe];
    for c in "# UTF-16 示例\r\n$name = '项目'\r\n".encode_utf16() {
        legacy.extend_from_slice(&c.to_le_bytes());
    }
    std::fs::write(root.join("scripts/legacy.ps1"), legacy).unwrap();
    std::fs::write(root.join("image.bin"), [0, 1, 2, 3]).unwrap();
    std::fs::write(root.join("invalid.txt"), [0xff]).unwrap();
    std::fs::File::create(root.join("large.dat"))
        .unwrap()
        .set_len(ai_terminal::project_files::MAX_BYTES + 1)
        .unwrap();
    eframe::run_native(
        "ai-terminal project files preview",
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
            let mut client = FileClient::new(cc.egui_ctx.clone());
            client.set_root(Some(&root));
            client.ensure_directory(std::path::Path::new(""));
            let (tx, _) = mpsc::channel();
            let terminal=TerminalBackend::new(940,cc.egui_ctx.clone(),tx,BackendSettings {shell:std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap()).join("System32/WindowsPowerShell/v1.0/powershell.exe").to_string_lossy().into_owned(),args:vec!["-NoLogo".into(),"-NoProfile".into(),"-Command".into(),"Write-Output 'PROJECT_READY · 项目文件浏览 · 原终端保持连接'; Start-Sleep -Seconds 70".into()],working_directory:Some(root.clone()),..Default::default()}).unwrap();
            let initial_pid = terminal.pty_id();
            let job = platform::ProcessJob::attach(initial_pid).unwrap();
            Ok(Box::new(Probe {
                _job: job,
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                client,
                view: Default::default(),
                git: Default::default(),
                terminal,
                root,
                output,
                stage: 0,
                frames: 0,
                pending: false,
                began: Instant::now(),
                since: Instant::now(),
                records: vec![],
                initial_pid,
            }))
        }),
    )
}
