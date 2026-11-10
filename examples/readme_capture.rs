//! Public workspace capture using the production sidebar and terminal.
//! Synthetic metadata and fixed console text only; no Agent/model/user profile.
use ai_terminal::{
    appearance,
    floating::{Item, Snapshot},
    git::{
        Commit, CommitDetails, CommitFile, FileChange, GitClient, Inspection,
        Snapshot as GitSnapshot,
    },
    git_ui::{self, GitView},
    i18n::{self, Language, Locale},
    model::{Agent, SessionColor},
    sidebar::{self, Entry, Icons},
    status::{Activity, Status},
};
use egui_term::{BackendSettings, TerminalBackend, TerminalView};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Capture {
    renderer: egui_term::TextRendererSession,
    backend: TerminalBackend,
    icons: Icons,
    snapshot: Snapshot,
    git: GitClient,
    git_view: GitView,
    status: Status,
    history: bool,
    diff: bool,
    output: PathBuf,
    began: Instant,
    requested: bool,
    captured: bool,
    click_stage: u8,
}
impl eframe::App for Capture {
    #[cfg(feature = "renderer-glow")]
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.renderer.release();
    }
    #[cfg(not(feature = "renderer-glow"))]
    fn on_exit(&mut self) {
        self.renderer.release();
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.captured || self.began.elapsed() > Duration::from_secs(18) {
            std::fs::write(self.output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"ok":self.captured,"version":env!("CARGO_PKG_VERSION"),"synthetic_metadata":true,"synthetic_git":true,"history":self.history,"file_diff":self.diff,"example_paths":true,"model_requests":false,"user_profiles":false,"pty_pid":self.backend.pty_id()})).unwrap()).unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(80));
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        // Discard incidental typing/hover within this isolated capture window.
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        input.events.push(egui::Event::PointerGone);
        if self.history
            && self.click_stage < 2
            && self.began.elapsed() > Duration::from_secs(1)
            && let Some(commit) = self.git.snapshot.as_ref().and_then(|s| s.commits.first())
            && let Some(row) = ctx.read_response(egui::Id::new(("git-commit-row", &commit.id)))
        {
            let pos = row.rect.center();
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: self.click_stage == 0,
                modifiers: egui::Modifiers::NONE,
            });
            self.click_stage += 1;
        }
        for event in &input.events {
            if let egui::Event::Screenshot { image, .. } = event {
                let file = std::fs::File::create(self.output.join("workspace.png")).unwrap();
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
                self.captured = true;
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let paths: Vec<_> = [
            "web-app",
            "api-service",
            "desktop-app",
            "docs",
            "workspace",
            "workspace",
            "workspace",
        ]
        .map(|name| PathBuf::from(format!("C:\\Workspace\\{name}")))
        .into();
        let entries: Vec<_> = self
            .snapshot
            .items
            .iter()
            .zip(&paths)
            .map(|(item, directory)| Entry {
                id: &item.id,
                name: &item.name,
                directory,
                agent: item.agent,
                color: item.color,
                activity: item.activity,
                unread: item.unread,
                hint: String::new(),
            })
            .collect();
        sidebar::show(ui, &entries, Some("readme-0"), false, 238.0, &self.icons);
        let mut open = true;
        self.git_view.panel(ui, Some(&self.git), &mut open);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::BLACK)
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                git_ui::header(
                    ui,
                    git_ui::Header {
                        name: &self.snapshot.items[0].name,
                        directory: std::path::Path::new("C:/Workspace/web-app"),
                        scale: 1.0,
                        connection: Some((Agent::Codex, &self.status)),
                    },
                    Some(&self.git),
                    &mut open,
                );
                ui.add_space(8.0);
                if let Some(patch) = &self.git.patch {
                    git_ui::diff::show(ui, patch);
                } else {
                    let view = TerminalView::new(ui, &mut self.backend)
                        .set_font(appearance::terminal_font(14.0))
                        .set_size(ui.available_size())
                        .set_focus(true);
                    ui.add(view);
                }
            });
        if !self.requested
            && self.began.elapsed() > Duration::from_secs(3)
            && (self.diff
                || self
                    .backend
                    .last_content()
                    .cells
                    .iter()
                    .map(|cell| cell.cell.c)
                    // Wide-cell padding is present in the raw grid; compare the
                    // fixed sample without whitespace, for both locales.
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>()
                    .contains(if self.snapshot.locale == Locale::En {
                        "Returntoanysessionandpickupwhereyouleftoff."
                    } else {
                        "回到任何一个会话，继续你的工作。"
                    }))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.requested = true;
        }
    }
}
fn main() -> eframe::Result {
    ai_terminal::launch::prepare_gui_environment();
    let output = PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));
    std::fs::create_dir_all(&output).unwrap();
    assert!(
        !output.join("workspace.png").exists(),
        "Use a fresh directory"
    );
    let english = std::env::args().nth(2).as_deref() == Some("en");
    let history = std::env::args().nth(3).as_deref() == Some("history");
    let diff = std::env::args().nth(3).as_deref() == Some("diff");
    let verify = output.clone();
    eframe::run_native(
        "ai-terminal public README capture",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 720.0])
                .with_active(false)
                .with_transparent(true)
                .with_icon(appearance::window_icon()),
            ..Default::default()
        },
        Box::new(move |cc| {
            appearance::setup(&cc.egui_ctx);
            i18n::apply(
                &cc.egui_ctx,
                if english {
                    Language::En
                } else {
                    Language::ZhCn
                },
            );
            ai_terminal::window_frame::apply_dark(cc);
            let lines = if english {
                vec![
                    "AI Terminal  ·  Your agents, one workspace",
                    "",
                    "› Keep today's development tasks in view.",
                    "",
                    "• Six coding agents. Your familiar terminal.",
                    "",
                    "  Web app       Refining the interface and interactions",
                    "  API service   One action needs your approval",
                    "  Release       Build complete, ready for your review",
                    "",
                    "  Stay in the flow",
                    "  ✓ Branch, history and read-only file diffs",
                    "  ✓ Clear avatars and unread completion badges ✨",
                    "  ✓ Switch sessions without restarting your agents",
                    "",
                    "  Drag the history scrollbar to find earlier output",
                    "  Ctrl + wheel to zoom · Drag sessions to reorder",
                    "",
                    "Return to any session and pick up where you left off.",
                ]
            } else {
                vec![
                    "AI Terminal  ·  多会话，也可以从容",
                    "",
                    "› 帮我整理今天的开发任务。",
                    "",
                    "• 六种编程 Agent，在熟悉的终端里继续工作。",
                    "",
                    "  网站开发    正在完善界面与交互",
                    "  接口联调    有一项操作等待你确认",
                    "  发布准备    构建已完成，等你回来查看",
                    "",
                    "  保持专注",
                    "  ✓ 当前分支、提交历史与只读文件差异",
                    "  ✓ 清晰的头像与未读完成标记 ✨",
                    "  ✓ 随时切换会话，不用重新启动 Agent",
                    "",
                    "  拖动右侧历史条，快速定位之前的输出",
                    "  Ctrl + 滚轮缩放 · 拖动会话排序",
                    "",
                    "回到任何一个会话，继续你的工作。",
                ]
            };
            let expression = lines
                .into_iter()
                .map(|line| format!("'{}'", line.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join("+\"`r`n\"+");
            let archived = if english {
                "Earlier task output"
            } else {
                "之前的任务输出"
            };
            let script = format!(
                "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); for($captureLine=1;$captureLine -le 120;$captureLine++){{[Console]::WriteLine('{archived} '+$captureLine)}}; $e=[char]27; [Console]::Write($e+'[2J'+$e+'[H'+{expression}); [Console]::WriteLine(''); Start-Sleep -Seconds 25"
            );
            let (tx, _) = mpsc::channel();
            let backend = TerminalBackend::new(
                902,
                cc.egui_ctx.clone(),
                tx,
                BackendSettings {
                    shell: "powershell.exe".into(),
                    args: vec![
                        "-NoLogo".into(),
                        "-NoProfile".into(),
                        "-NonInteractive".into(),
                        "-Command".into(),
                        script,
                    ],
                    working_directory: Some(PathBuf::from(
                        std::env::var_os("SystemRoot").expect("Windows SystemRoot"),
                    )),
                    ..Default::default()
                },
            )
            .unwrap();
            let icons = Icons::new(&cc.egui_ctx);
            let mut git = GitClient::new(cc.egui_ctx.clone());
            let mut fixture = git_fixture();
            if diff {
                let patch = file_fixture(english);
                fixture.files[0].added = patch
                    .lines
                    .iter()
                    .filter(|l| l.kind == ai_terminal::git::PatchKind::Added)
                    .count() as u64;
                fixture.files[0].deleted = patch
                    .lines
                    .iter()
                    .filter(|l| l.kind == ai_terminal::git::PatchKind::Deleted)
                    .count() as u64;
                fixture.added = fixture.files.iter().map(|f| f.added).sum();
                fixture.deleted = fixture.files.iter().map(|f| f.deleted).sum();
                git.patch = Some(ai_terminal::git::PatchResult {
                    source: patch.source.clone(),
                    path: patch.path.clone(),
                    data: Some(std::sync::Arc::new(patch)),
                    error: None,
                    loading: false,
                });
            }
            if history {
                let commit = &fixture.commits[0];
                git.inspection = Some(Inspection {
                    id: commit.id.clone(),
                    loading: false,
                    error: None,
                    data: Some(std::sync::Arc::new(CommitDetails {
                        id: commit.id.clone(),
                        message: commit.subject.clone(),
                        parents: commit.parents.clone(),
                        files: fixture
                            .files
                            .iter()
                            .map(|f| CommitFile {
                                path: f.path.clone(),
                                status: if f.index == '?' { 'A' } else { 'M' },
                                added: f.added,
                                deleted: f.deleted,
                                binary: false,
                            })
                            .collect(),
                        total_files: 3,
                        added: 42,
                        deleted: 11,
                        partial: false,
                    })),
                });
            }
            git.snapshot = Some(std::sync::Arc::new(fixture));
            git.watching = true;
            let mut git_view = GitView::default();
            git_view.set_history(history);
            let mut status = Status::default();
            status.activity = Activity::Working;
            status.native_lifecycle = true;
            status.last_event = Some("UserPromptSubmit".into());
            let snapshot = Snapshot {
                items: [
                    (
                        if english {
                            "Web app ✨"
                        } else {
                            "网站开发 ✨"
                        },
                        Agent::Codex,
                        Activity::Working,
                        Some(SessionColor::Blue),
                    ),
                    (
                        if english {
                            "API service"
                        } else {
                            "接口联调"
                        },
                        Agent::Claude,
                        Activity::Waiting,
                        Some(SessionColor::Purple),
                    ),
                    (
                        if english {
                            "Release 🚀"
                        } else {
                            "发布准备 🚀"
                        },
                        Agent::QoderCn,
                        Activity::Completed,
                        Some(SessionColor::Green),
                    ),
                    (
                        if english {
                            "Documentation"
                        } else {
                            "文档整理"
                        },
                        Agent::Kimi,
                        Activity::Working,
                        None,
                    ),
                    (
                        if english {
                            "Code review"
                        } else {
                            "代码审查"
                        },
                        Agent::ZCode,
                        Activity::Idle,
                        None,
                    ),
                    (
                        if english {
                            "Desktop app"
                        } else {
                            "桌面应用"
                        },
                        Agent::Trae,
                        Activity::Waiting,
                        None,
                    ),
                    ("PowerShell", Agent::Shell, Activity::Idle, None),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (name, agent, activity, color))| Item {
                    id: format!("readme-{i}"),
                    name: name.into(),
                    agent,
                    color,
                    activity,
                    unread: matches!(activity, Activity::Completed | Activity::Waiting),
                })
                .collect(),
                active: Some("readme-0".into()),
                locale: if english { Locale::En } else { Locale::ZhCn },
            };
            Ok(Box::new(Capture {
                renderer: egui_term::TextRendererSession::new(&cc.egui_ctx),
                backend,
                icons,
                snapshot,
                git,
                git_view,
                status,
                history,
                diff,
                output,
                began: Instant::now(),
                requested: false,
                captured: false,
                click_stage: 0,
            }))
        }),
    )?;
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(verify.join("result.json")).expect("capture result"))
            .unwrap();
    assert_eq!(result["ok"], true);
    Ok(())
}

fn file_fixture(english: bool) -> ai_terminal::git::FilePatch {
    use ai_terminal::git::{FilePatch, PatchKind::*, PatchLine, PatchSource, syntax};
    let sample = [
        (Context, "import { useState } from 'react';"),
        (Context, ""),
        (Context, "export function SignIn() {"),
        (Context, "  const [email, setEmail] = useState('');"),
        (Context, "  const [status, setStatus] = useState('idle');"),
        (Context, ""),
        (Context, "  async function submit() {"),
        (Context, "    setStatus('loading');"),
        (Deleted, "    await signIn(email);"),
        (Added, "    await signIn(email.trim());"),
        (Added, "    setStatus('complete');"),
        (Context, "  }"),
        (Context, ""),
        (
            Context,
            if english {
                "  // Ready for review ✅"
            } else {
                "  // 可以开始验收了 ✅"
            },
        ),
        (
            Context,
            "  return <Form onSubmit={submit} email={email} />;",
        ),
        (Context, "}"),
    ];
    let (mut old, mut new) = (1, 1);
    let lines: Vec<_> = sample
        .into_iter()
        .map(|(kind, text)| {
            let line = PatchLine {
                kind,
                old: (kind != Added).then_some(old),
                new: (kind != Deleted).then_some(new),
                text: text.into(),
            };
            if kind != Added {
                old += 1;
            }
            if kind != Deleted {
                new += 1;
            }
            line
        })
        .collect();
    let path = "src/components/SignIn.tsx";
    let presentation = syntax::prepare(path, &lines, || false);
    FilePatch {
        source: PatchSource::WorkingTree,
        path: path.into(),
        max_columns: 52,
        lines,
        binary: false,
        partial: false,
        first_parent: false,
        presentation,
    }
}

fn git_fixture() -> GitSnapshot {
    GitSnapshot {
        root: "C:/Workspace/web-app".into(),
        git_dir: "C:/Workspace/web-app/.git".into(),
        common_dir: "C:/Workspace/web-app/.git".into(),
        branch: "feature/sign-in".into(),
        detached: false,
        unborn: false,
        added: 42,
        deleted: 11,
        total_files: 3,
        files: [
            ("src/components/SignIn.tsx", ' ', 'M', 24, 6),
            ("tests/sign-in.spec.ts", 'M', ' ', 12, 5),
            ("docs/sign-in.md", '?', '?', 6, 0),
        ]
        .into_iter()
        .map(|(path, index, worktree, added, deleted)| FileChange {
            path: path.into(),
            index,
            worktree,
            added,
            deleted,
            binary: false,
            omitted: false,
        })
        .collect(),
        partial: false,
        commits: [
            (
                "a24df891c7f34985b6e219d078ac523946afe103",
                "Alex",
                "feat: add the sign-in form",
            ),
            (
                "39b8a210d467ef93c08a174b5fa6298d30be65c2",
                "Sam",
                "fix: preserve session navigation",
            ),
            (
                "5ef731a892b4c06de19f3ac845ba1d702c69e083",
                "Alex",
                "test: cover sign-in validation",
            ),
            (
                "b73a54f091cd268e37ba8f06c245e9d1a30478bc",
                "Sam",
                "docs: explain workspace setup",
            ),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (id, author, subject))| Commit {
            id: id.into(),
            parents: vec![],
            author: author.into(),
            timestamp: 1794182400 - i as i64 * 86400,
            subject: subject.into(),
        })
        .collect(),
        more: false,
    }
}
