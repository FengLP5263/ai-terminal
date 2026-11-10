//! Native, read-only syntax fixtures. No repositories, interpreters or model calls.
use ai_terminal::{
    appearance,
    git::{FilePatch, PatchKind, PatchLine, PatchResult, PatchSource, syntax},
    git_ui, i18n,
};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

const CASES: &[(&str, &str)] = &[
    (
        "scripts/sample.ps1",
        r##"<#
.SYNOPSIS
只读脚本语法预览，不执行任何命令。
#>
param([switch]$List, [string]$Item)
$ErrorActionPreference = 'Stop'
function Get-Summary([string]$Name) {
    if ($List -or -not $Name) {
        Write-Host "当前项目: $env:USERPROFILE / $Name"
        return $null # 行尾注释
    }
    $count = 42
    $literal = '保留原文 $Name ❤️'
    $message = @"
Hello $Name, count = $count
"@
    return @{ Name = $Name; Count = $count }
}
Get-Summary -Name $Item
"##,
    ),
    (
        "modules/demo.psm1",
        "# PowerShell module\nfunction Get-Message {\n    param([string]$Name)\n    return \"Hello $Name\"\n}\nExport-ModuleMember -Function Get-Message",
    ),
    (
        "modules/demo.psd1",
        "# PowerShell module manifest\n@{\n    RootModule = 'demo.psm1'\n    ModuleVersion = '1.0.0'\n    FunctionsToExport = @('Get-Message')\n    PrivateData = @{ Enabled = $true }\n}",
    ),
    (
        "scripts/setup.cmd",
        "@echo off\nrem Batch variables and commands\nsetlocal EnableDelayedExpansion\nset \"NAME=demo\"\nif defined NAME (\n    echo Hello %NAME% !NAME!\n)\nfor %%F in (*.txt) do echo %%~nF\nexit /b 0",
    ),
    (
        "scripts/build.sh",
        "#!/usr/bin/env bash\n# Shell script preview\nset -eu\nname=\"demo\"\nif [ -n \"$name\" ]; then\n    printf 'Hello %s\\n' \"${name}\"\nfi\ncat <<EOF\nHello $name\nEOF",
    ),
    (
        "scripts/task.py",
        "# Python preview\nfrom pathlib import Path\n\ndef greet(name: str) -> str:\n    \"\"\"A multiline docstring.\n    This code is never executed.\n    \"\"\"\n    count = 42\n    return f\"Hello {name}: {count}\"",
    ),
    (
        "src/types.ts",
        "// Real TypeScript grammar\ninterface User {\n    name: string;\n    enabled: boolean;\n}\nexport function greet(user: User): string {\n    const count: number = 42;\n    return `Hello ${user.name}: ${count}`;\n}",
    ),
    (
        "src/App.tsx",
        "// TSX keeps both type and JSX syntax\ntype Props = { name: string };\nexport const App = ({ name }: Props) => (\n    <section className=\"card\">\n        <h1>Hello {name} ❤️</h1>\n    </section>\n);",
    ),
    (
        "src/Card.jsx",
        "// JSX preview\nexport function Card({ title }) {\n    return <article className=\"card\">\n        <h2>{title}</h2>\n    </article>;\n}",
    ),
    (
        "Dockerfile.dev",
        "# Dockerfile variant\nFROM alpine:3.20\nARG APP=demo\nENV APP_HOME=/app\nWORKDIR ${APP_HOME}\nCOPY . .\nRUN echo \"Building ${APP}\"\nCMD [\"sh\"]",
    ),
    (
        "Cargo.toml",
        "# TOML preview\n[package]\nname = \"demo\"\nversion = \"0.1.0\"\n[features]\ndefault = []\n[profile.release]\nopt-level = 3\nstrip = true",
    ),
    (
        ".env.production",
        "# Synthetic configuration; no credentials\nAPP_NAME=demo\nAPP_PORT=8080\nAPP_LABEL=\"Hello world\"\nAPP_ENABLED=true",
    ),
    (
        "config/build.yml",
        "# YAML preview\nname: demo\non: [push]\njobs:\n  build:\n    runs-on: windows-latest\n    steps:\n      - name: Build\n        run: cargo build --locked",
    ),
    (
        "config/settings.ini",
        "; INI preview\n[application]\nname=demo\nport=8080\n; No settings are loaded\nenabled=true",
    ),
    (
        "config/settings.jsonc",
        "{\n  // JSON with comments\n  \"name\": \"demo\",\n  \"count\": 42,\n  \"enabled\": true\n}",
    ),
    (
        "README.md",
        "# 文档语法预览\n\n> 只读展示，不运行围栏中的代码。\n\n- **粗体**与 `inline code`\n\n```powershell\n$name = 'demo'\nWrite-Host \"Hello $name\"\n```",
    ),
];

fn fixture(path: &str, text: &str) -> FilePatch {
    let mut lines = Vec::new();
    let count = text.lines().count();
    for (i, text) in text.lines().enumerate() {
        let n = Some(i as u64 + 1);
        if i == count / 2 {
            lines.push(PatchLine {
                kind: PatchKind::Deleted,
                old: n,
                new: None,
                text: format!("{text} "),
            });
            lines.push(PatchLine {
                kind: PatchKind::Added,
                old: None,
                new: n,
                text: text.into(),
            });
        } else {
            lines.push(PatchLine {
                kind: PatchKind::Context,
                old: n,
                new: n,
                text: text.into(),
            });
        }
    }
    FilePatch {
        source: PatchSource::WorkingTree,
        path: path.into(),
        max_columns: lines
            .iter()
            .map(|l| l.text.chars().count())
            .max()
            .unwrap_or(0),
        presentation: syntax::prepare(path, &lines, || false),
        lines,
        binary: false,
        partial: false,
        first_parent: false,
    }
}
struct Probe {
    renderer: egui_term::TextRendererSession,
    icons: ai_terminal::sidebar::Icons,
    files: Vec<Arc<FilePatch>>,
    output: PathBuf,
    stage: usize,
    frames: usize,
    since: Instant,
    began: Instant,
    pending: bool,
    captures: Vec<serde_json::Value>,
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
            "Syntax capture timeout"
        );
        ctx.request_repaint_after(Duration::from_millis(50));
    }
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let name = format!(
                    "syntax-{}-{}.png",
                    self.stage / 2,
                    if self.stage.is_multiple_of(2) {
                        100
                    } else {
                        150
                    }
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
                self.captures.push(serde_json::json!({"image":name,"path":CASES[self.stage/2].0,"scale":if self.stage.is_multiple_of(2){1.0}else{1.5}}));
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
        if self.stage == CASES.len() * 2 {
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":true,"version":env!("CARGO_PKG_VERSION"),"screenshots":self.captures,"source":"synthetic fixtures via production diff renderer","model_requests":false,"scripts_executed":false})).unwrap()).unwrap();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.frames == 0 {
            ui.ctx().set_zoom_factor(if self.stage.is_multiple_of(2) {
                1.0
            } else {
                1.5
            });
        }
        ui.horizontal(|ui| {
            for agent in [
                ai_terminal::model::Agent::Shell,
                ai_terminal::model::Agent::Bash,
                ai_terminal::model::Agent::Cmd,
            ] {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                self.icons.paint(ui.painter(), rect, agent);
                ui.label(agent.label());
                ui.add_space(12.0);
            }
            ui.weak("只读语法预览 · 固定样例 · 不执行脚本");
        });
        let file = self.files[self.stage / 2].clone();
        git_ui::diff::show(
            ui,
            &PatchResult {
                source: file.source.clone(),
                path: file.path.clone(),
                data: Some(file),
                loading: false,
                error: None,
            },
        );
        self.frames += 1;
        if !self.pending && self.frames > 5 && self.since.elapsed() > Duration::from_millis(350) {
            self.pending = true;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
    }
}
fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--export-notices") {
        let data =
            serde_json::to_vec_pretty(two_face::acknowledgement::listing().for_syntaxes()).unwrap();
        std::fs::write(args.get(2).expect("license output path"), data).unwrap();
        return Ok(());
    }
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(args.get(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(!output.join("result.json").exists());
    let files = CASES.iter().map(|(p, t)| Arc::new(fixture(p, t))).collect();
    eframe::run_native(
        "ai-terminal syntax preview",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_icon(appearance::window_icon())
                .with_active(false)
                .with_inner_size([1100.0, 700.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(&cc.egui_ctx, i18n::Language::ZhCn);
            Ok(Box::new(Probe {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                icons: ai_terminal::sidebar::Icons::new(&cc.egui_ctx),
                files,
                output,
                stage: 0,
                frames: 0,
                since: Instant::now(),
                began: Instant::now(),
                pending: false,
                captures: vec![],
            }))
        }),
    )
}
