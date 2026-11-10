# Development and delivery scripts

[简体中文](README.md) | English

The entry points are `scripts/windows/project.ps1 <action>` on Windows and `sh scripts/posix/project.sh <action>` on POSIX. They locate the repository from the script; relative artifact paths are relative to its root. Use PowerShell `Get-Help` or POSIX `--help` for full options. Python 3.11+ is required; `AI_TERMINAL_PYTHON` optionally selects it.

| Action | Purpose |
| --- | --- |
| `governance` | Check repository files, provenance, privacy, documentation, and governance. |
| `format-check` | Check cargo formatting without rewriting source. |
| `check` | Formatting plus Clippy with warnings denied. |
| `test` | Rust workspace and Python package-verification tests. |
| `ci` | Governance, check, and test. |
| `build`, `run` | Build the debug app, or build and open its interactive window. |
| `release` | Build the optimized app. |
| `smoke -Release` | Isolated native smoke with real PTYs, synthetic status, a GPU screenshot, and owned-process cleanup. |
| `measure-idle -Release` | Short-window manager CPU and memory measurement. |
| `package` | Build a release ZIP with manifest and licenses and verify it by reading it back. |
| `verify-package -Path ZIP` | Verify a ZIP without extracting or executing it. |
| `verify-claude -AllowModelRequest` | One potentially billed model check, only with explicit user authorization. |

Use the corresponding Windows/POSIX entry point for each action. POSIX governance and package verification run Python directly; format-check can invoke configured Rust. Git Bash delegates native operations to the Windows entry point. macOS/Linux native GUI, native tests, and model-helper actions fail explicitly; Git Bash checks do not establish native macOS/Linux support.

Windows builds compile embedded icon resources through `build.rs` and Windows SDK `rc.exe`, discovered on PATH/SDK or specified by `RC`. No separate image tool is required. The source identity includes `build.rs` and `assets/branding`.

## Options and side effects

- The default action is `governance`. `-Release` selects the release directory for smoke and measurements; build before running them.
- Labels accept letters, digits, and hyphens. The real-model helper supports debug/release profiles and defaults to release.
- Packaging requires a clean Git tree. `-AllowDirty` is only for local acceptance and marks the manifest dirty; do not present it as a clean release.
- Model verification requires both `-AllowModelRequest` and explicit user authorization; `ci` and `test` do not call it.
- Optional `.tools/env.ps1` and `.tools/cargo-network.toml` are ignored local configuration and are not packaged.
- Errors return nonzero. Default checks do not upgrade dependencies, modify user data, or access remotes. Packages use unique directories under `output/packages/` and do not overwrite running previews.
- Legacy scripts (`dev.ps1`, `smoke.ps1`, `measure-idle.ps1`, `verify-claude.py`) remain compatible and use the same safety boundaries.

## Verification boundaries

governance, ci, and package require a unique dated entry with categorized changes for the current Cargo version in both root changelogs, with matching dates. Packaging includes both complete files and their bytes in source identity. Windows/POSIX entry points and arguments are unchanged. Regressions cover missing translations, wrong/duplicate versions, date differences, and empty current entries.

`project.ps1 ci` is vendor-independent; a local run is not a GitHub Actions result. Standard CI does not require a GPU, model account, or proxy. Native GPU smoke and visual acceptance are separate interactive Windows checks. Historical Windows CI, native smoke, idle measurement, package, and Git Bash governance/format results are documented in the [Chinese record](README.md); native macOS/Linux remains unverified.

Since 0.3.7, test/CI also explicitly run `egui-winit`'s `paste_tests`. The `codex-image` and `claude-image` terminal probes insert but do not send an attachment. `lifecycle_probe offline` uses a local fake Responses service, not a real billed model turn.

Packaging respects Cargo's `target_directory` and `CARGO_TARGET_DIR`. If the default binary is in use, build with a separate ignored target directory; do not overwrite a running preview.

For 0.4.0 multi-window acceptance, build `island_probe` offline with `cargo build --offline --locked --example island_probe`, then run `target/debug/examples/island_probe.exe output/<unique-island-probe>`. It uses synthetic states and its own windows, writes PNG/JSON, and exits. Record whether the floating monitor and active status are enabled during performance checks.

For zoom, build `island_zoom_probe` with `cargo build --offline --locked --example island_zoom_probe`, then run `target/debug/examples/island_zoom_probe.exe output/<unique-zoom-probe>`. Verify physical viewport size, screen centering, vertical position, and screenshots at 50%, 71%, 100%, and 150%. The optional physical display X coordinate tests different-DPI monitors; the probe only reads its own windows and uses synthetic sessions.

For native Git Bash startup/screenshots (Windows only, from either PowerShell or Git Bash), build with `cargo build --offline --locked --bins --example bash_probe`, then run `target/debug/examples/bash_probe.exe output/<unique-bash-probe>`. It uses real Git Bash, a temporary HOME and synthetic user startup files; verifies directory metadata, configuration/Agent entries and a PNG, then exits. It reads no real user profile and starts no model. Missing Git or failed validation explicitly returns nonzero. Record synthetic missing-shell fallback checks separately from installed-Git native acceptance.

For native shell-avatar screenshots (Windows only, PowerShell/Git Bash), build with `cargo build --offline --locked --release --bins --example shell_switch_probe`, then run `target/release/examples/shell_switch_probe.exe output/<unique-shell-switch-probe>`. It uses the production App controls and B icon, isolated session data and the system directory, without user profiles or Agent/model startup. Synthetic input goes only to its own window: CMD→PowerShell→CMD→per-level exit. It writes five screenshots and exits normally. capture.json reports capture only; manually verify content, avatars and successful exit rather than treating it as automatic visual acceptance.

Append `codex-direct` to the same probe: `target/release/examples/shell_switch_probe.exe output/<unique-codex-direct> codex-direct`. In the current already trusted directory, launch real `codex --remote unix:// -C <cwd>` by absolute path and capture the production Shell/Codex UI twice. Submit no model prompt, accept no trust/hook and do not stop the shared server. Exit reaps only the probe's terminal. POSIX native limitations are unchanged; no new project action is added.

For sidebar overflow/scrollbar screenshots (Windows only, PowerShell/Git Bash), build with `cargo build --offline --locked --release --bins --example workspace_probe`, then run `target/release/examples/workspace_probe.exe output/<unique-sidebar-probe> 1160 many`. The many mode creates 30 synthetic sessions and captures expanded, zoomed and collapsed/scrollbar-hover states as PNG/JSON. It uses only its own ANSI-fixture PTY, without an Agent or model. Verify the bar stays narrow and clear of avatars, and confirm normal exit.

For fullscreen wheel/drag-selection screenshots (Windows only), build with `cargo build --offline --locked --release --example scroll_probe`, then compile the local fixture with `rustc --edition=2024 --crate-name mouse_console tests/fixtures/mouse_console.rs -o output/mouse-console.exe`. Run `target/release/examples/scroll_probe.exe output/mouse-console.exe output/<unique-selection-probe>`. Verify scrolling in both directions, ordinary left-drag selection highlighting, the connection popup and successful exit. Real ConPTY regressions in `tests/terminal_interaction.rs` cover copying. The native probe sends input only to its own window, never writes the system clipboard and starts no Agent/model.

Artifact source identity also includes the local egui_glow Cargo manifest, Rust sources, and GLSL shaders so the graphics patch is covered.

For Windows compatibility probes, build with `cargo build --offline --locked --release --bins --example composer_probe --example titlebar_probe --example discovery_probe` and compile output/mouse-console.exe with the rustc command above. Run `target/release/examples/composer_probe.exe output/mouse-console.exe output/<unique-composer-probe>`, `target/release/examples/titlebar_probe.exe output/<unique-titlebar-probe>` and `target/release/examples/discovery_probe.exe output/<unique-discovery-probe>`. The first two use only owned windows/local fixtures; the last runs installed CLIs with --version only. Check JSON, PNG and exit codes, recording Windows 10/11 separately. Builds stage pinned native-console companions; copying only a probe EXE to another device is incomplete.

Floating cross-monitor/DPI check (at least two physical monitors): cargo build --offline --locked --release --example island_move_probe, then target/release/examples/island_move_probe.exe output/unique-island-move. It moves only its own floating viewport, checking native client dimensions and writing stage PNG/JSON at zoom levels including 71%, 142%, and 200%. Missing a second monitor fails explicitly. No user-window operations, Agent or model requests. Inspect screenshots manually; see [UI rendering](../docs/ui-rendering.en.md) for causes and the Codex agents-entry boundary.
Actual Claude manual mode: run terminal_probe.exe claude-manual output/unique-manual-probe from an existing trusted directory, using the built examples executable path. It adds --permission-mode default only to this CLI invocation, does not change user preferences, and submits no model request. Missing screenshots/results cause nonzero exit; a trust prompt is not a successful capture.

Collapsed-sidebar threshold layout: cargo build --offline --locked --example workspace_probe, then target/debug/examples/workspace_probe.exe output/unique-sidebar-threshold 900 threshold. Seven synthetic sessions and an isolated fixture PTY compare session geometry across consecutive frames at normal size, after zoom, and after resizing. It writes three PNGs and a stability_checks count; oscillating geometry fails the probe. The existing many mode covers long-list scrollbar screenshots and does not replace threshold checks.

The preview.3 island_move_probe also checks native movement enter/exit notifications, multi-frame position retention and oversized-client recovery. Notifications and programmatic monitor transitions run separately and do not simulate the system mouse modal loop; physical_mouse_drag is false in the result JSON, and physical cross-monitor dragging requires separate acceptance.

The preview.4 cross-monitor probe uses the same commands and moves its own main and floating windows while native movement state spans DPI changes, saving main-*.png/move-*.png and dpi_trace. Check suggested/actual rectangle error, nested DPI depth, deferred mid-move size repair and stable dimensions on repeated crossings. physical_mouse_drag remains false; physical mouse-modal dragging requires separate acceptance. Local winit build sources/embedded assets are included in package source identity and its license is collected in the dependency inventory.

Public README captures (Windows only; PowerShell / Git Bash):

```powershell
cargo build --offline --locked --release --example readme_capture --example island_probe
target/release/examples/readme_capture.exe output/<unique-workspace-zh> zh
target/release/examples/readme_capture.exe output/<unique-workspace-en> en
target/release/examples/readme_capture.exe output/<unique-history-zh> zh history
target/release/examples/readme_capture.exe output/<unique-history-en> en history
target/release/examples/readme_capture.exe output/<unique-diff-zh> zh diff
target/release/examples/readme_capture.exe output/<unique-diff-en> en diff
target/release/examples/readme_capture.exe output/<unique-files-zh> zh files
target/release/examples/readme_capture.exe output/<unique-files-en> en files
target/release/examples/island_probe.exe output/<unique-floating>
```

Use a fresh directory each time. `readme_capture <output> [zh|en] [history|diff|files]` defaults to Changes; the third argument selects History, a read-only diff or the project file tree. History injects a click only into its own egui context to expand a fixture commit; diff uses fixed source lines and production syntax preparation. All four modes produce workspace.png. Production sidebar, font, header, shared Files/Git panel and terminal components use only fixed text, example paths and synthetic Git/task metadata, without querying user repositories. `island_probe` produces Chinese/English floating-bar and Settings captures. Neither launches Agents/models or reads user profiles; only owned windows and PTYs are managed. Verify exit codes, result.json, owned PTY cleanup, actual PNGs and privacy, then copy both workspace.png files, the history/diff-mode workspace.png files and island-zh.png/island-en.png unchanged to their workspace, git-history, git-diff, project-files and floating bilingual names in assets/readme. Update version, source, dimensions and SHA-256 in SOURCE.json. Display fixtures do not replace actual Git or Agent acceptance.

Native Codex lifecycle/overview and mode badges (Windows only): build `cargo build --offline --locked --release --bins --example agents_status_probe`. From an existing trusted directory, run `target/release/examples/agents_status_probe.exe codex output/<unique-agents-probe>`; only Left/overview and Esc/return are sent, producing three PNGs and finite-state JSON. No prompts, project/hook trust acceptance or shared-server stop. `target/release/examples/agents_status_probe.exe badges output/<unique-badge-probe>` uses fixed PowerShell text to capture adjacent triangles/pause/emoji at 100%/146%/200%. Check JSON, exit code and actual images; startup/navigation is not a model turn. Existing POSIX native restrictions apply; no new project action is added.

Sidebar drag and unread-failure captures (Windows only; PowerShell/Git Bash): build with `cargo build --offline --locked --release --example sidebar_drag_probe`, then run `target/release/examples/sidebar_drag_probe.exe output/<unique-sidebar-drag-probe>`. Production sidebar/floating components, the standard B icon and a fixed sample directory produce eight native PNGs plus result.json for expanded/collapsed unread, read, dragged-card and committed-order states. Synthetic input goes only to the probe's own egui instance; it starts no PTY, Agent or model, reads no user configuration and injects no system mouse input. Check the process exit code, grab point, destination gap and cleared red badge; capture completion does not replace visual review.

Qoder CN startup/synthetic status captures (Windows only, PowerShell/Git Bash): build with `cargo build --offline --locked --release --bins --example qoder_cn_probe`. Run `target/release/examples/qoder_cn_probe.exe output/<unique-qoder-fixture> fixture` for seven synthetic state captures. Omitting fixture only starts the installed Qoder CN and captures it, without prompts or trust approval. Default mode uses the current directory; run in an already trusted one. result.json records synthetic, hook_connected, event names and absence of model prompts separately. Check the exit code and actual images.

package also copies icon provenance and available original license texts into the ZIP's agent-icons/ and includes bilingual THIRD_PARTY_NOTICES. Brand assets have separate source declarations rather than a blanket Lobe MIT declaration. Windows/POSIX entry points and arguments are unchanged.

0.4.7 adds native startup checks for installed CLIs: build `cargo build --offline --locked --release --bins --example shell_switch_probe`, then run `target/release/examples/shell_switch_probe.exe output/<unique-cli-probe> kimi` (the last argument also accepts `zcode` or `traecli`). Only the launch command, an empty-terminal focus click and Esc/Ctrl+C are sent; no prompt or trust approval. Inspect three captures for the avatar, awaiting-events status and return to shell, recording the actual distribution. README avatars are generated with `cargo build --offline --locked --release --example agent_icons_capture` and `target/release/examples/agent_icons_capture.exe output/<unique-agent-icons>`. They reuse the production avatar painter and B application icon, with no PTY, Agent or model. Inspect all six PNGs, copy them unchanged into assets/readme, and update SOURCE.json. These native modes support Windows only; no project action or argument is added.

0.4.8 preview README captures: after building Release `readme_capture`, run `target/release/examples/readme_capture.exe output/readme-zh` and `target/release/examples/readme_capture.exe output/readme-en en` with fresh directories. English mode changes controls, names and synthetic terminal content. `island_probe` generates Chinese/English floating bars with six Agents in one run. Check exit codes, result.json and all four actual PNGs before copying them unchanged into assets/readme and updating SOURCE.json. These are not model acceptance tests.

Native floating-hint check (Windows): build `cargo build --offline --locked --release --example island_hint_probe`, then run `target/release/examples/island_hint_probe.exe output/<unique-hint-probe>`. It sends synthetic hover input only to its own egui window. At 100%/142%/200% zoom it checks full Chinese/English hints beyond the capsule, long-title wrapping, ownership, no activation, mouse passthrough and hiding on leave. Native hint PNGs, a hook Working-state capture and JSON are generated without Agents/models or user-window operations. Pure placement tests also cover negative-coordinate monitors and work-area edges. Inspect images manually; synthetic hook states are not real model requests.

0.4.9: qoder_cn_probe fixture produces 11 status frames covering automatic approval, genuine questions, manual waits, tool-failure recovery and delayed notices. island_hint_probe captures the actual egui hint viewport and checks colored hearts/ZWJ emoji, ownership, no activation, passthrough and hiding on leave. Build `cargo build --offline --locked --release --example typography_probe`, then run `target/release/examples/typography_probe.exe output/<unique-typography-probe>` for a real PowerShell ConPTY fixed sample at 100%/103%/110%/150%/200%: Chinese/English, color emoji and a black wide-character cursor on gray input. No Agent/model or user profile is involved. terminal_interaction separately verifies ordinary/private cursor responses and background/cursor/inverted-glyph ordering. Review each native image; test counts alone cannot prove visual quality.

Qoder input check in an already trusted directory: `target/release/examples/qoder_cn_probe.exe output/<unique-qoder-input> input`. Only fixed unsent markers, 100%/103%/110% zoom and retyping after a single Ctrl+C; four native images and visibility records. Never sends Enter or confirms trust; this is not real task-completion acceptance.

Font comparison: `typography_probe output/<unique-font-check> compare` opens an interactive window for three Latin/CJK faces and zoom levels. It does not save choices or modify the user workspace. Automated capture accepts `hack`/`consolas`/`cascadia` as the second argument, optionally `yahei`/`dengxian` as the third (Microsoft YaHei UI by default), retaining five scale captures. Candidate system fonts must already be installed; fallback glyphs must not be reported as the selected font. The production App still uses appearance.rs defaults and 0.4.8 native parameters.

0.4.9-preview.3 embeds four static Cascadia Mono faces by default and uses system Microsoft YaHei UI for CJK; preview.1 GDI classic fitting remains reverted. package checks all asset bytes/hashes against assets/fonts/SOURCE.json and copies the original Cascadia OFL/provenance. verify-package retains old-archive compatibility and requires provenance/license files when new fonts metadata is declared. Windows/POSIX entry points and arguments are unchanged.

Qoder sizing diagnostics: after building qoder_cn_probe, run `target/release/examples/qoder_cn_probe.exe output/<unique-qoder-trace> trace <isolated-directory>`. Start the CLI once, programmatically resize through four window sizes at 103%, and record at most 128 geometry samples plus four screenshots. No prompt or trust approval is sent. Trust/authentication pages are distinct from real model tasks.

Windows transient-render check: build with `cargo build --offline --locked --release --example render_update_probe`, then run `target/release/examples/render_update_probe.exe output/<unique-render-update> 1.03 48`. Optional arguments select zoom (0.5–2.5) and 8–512 screenshot comparisons. This probe checks a fixed table in one window; island_hint_probe separately checks the multiple-window hover path. Two local PowerShell fixtures supply a fixed file table and a changing-size glyph region; no Agent/model or user profile is used. Compare fixed-region pixels and visible-grid hashes, saving baseline/stage/first-failure images and JSON; timeout or any pixel/grid difference fails. Root size follows the chosen zoom, so large scales may exceed the display; captures come from its own GPU framebuffer. Production gains no polling or idle repaint.

## Windows installer (0.4.10 preview)

```powershell
.\scripts\windows\project.ps1 installer -AllowDirty
.\scripts\windows\project.ps1 installer -Path output/packages/<package>.zip -Acceptance -AcceptanceId <unique-id> -AllowDirty
.\scripts\windows\project.ps1 verify-installer -Path output/installers/<build>/<setup>.json
.\scripts\windows\project.ps1 installer-smoke -Path output/installers/<new>/<setup>.json -Previous output/installers/<old>/<setup>.json
```

POSIX provides matching actions/arguments. `verify-installer` is portable and read-only; other installer actions require Windows/Git Bash. Without `-Path`, `installer` builds the existing ZIP first. Stable builds reject dirty source/payloads; local previews use explicit `-AllowDirty`. `-Acceptance` uses an isolated identity; pair two versions with the same `-AcceptanceId` (6–64 lowercase letters, digits or hyphens). Native acceptance refuses production installers and owns only its test files/registry while checking fresh install, upgrade, busy-file rejection and uninstall. Unrelated files and session data are retained.

Builds require the pinned official NSIS 3.13 ZIP. Obtain `nsis-3.13.zip` from the [official NSIS download](https://nsis.sourceforge.io/Download), extract it completely under ignored `.tools/nsis/` (compiler: `.tools/nsis/nsis-3.13/makensis.exe`), or point `AI_TERMINAL_NSIS` at the compiler from that same tree. Verify the archive and complete file-tree SHA-256 recorded in `scripts/installer/TOOLCHAIN.json`. Builds do not download/install tools or execute user NSIS configuration. Original licenses remain in `scripts/installer/NSIS-LICENSE.txt`; TOOLCHAIN.json records the source-code URL. End users do not need the compiler.

Outputs are an EXE, `.sha256` and `.json`. Digest verification does not establish signing or successful installation. Records identify the ZIP, installer scripts, compiler and preview state without personal paths. See [installation and font selection](../docs/installation.en.md).

Native font captures: use the production default JetBrains Mono when no font is specified. Build `typography_probe`, then use `cascadia`, `hack`, `consolas`, `courier_new` or `jetbrains_mono` as its second argument to capture 100%/103%/110%/150%/200%. A fourth argument `settings-zh`/`settings-en` (third argument `default`) captures the real settings controls. Missing fonts fail explicitly rather than presenting a fallback as the requested family. Inspect the actual screenshots; fixtures contain only fixed local text.

Session-color captures: build `cargo build --offline --locked --release --example session_color_probe`, then run `target/release/examples/session_color_probe.exe output/<unique-session-colors>`. Capture production palette/sidebar selected, hovered, No color and collapsed states at 100%/103%/200%. Fixed sessions and synthetic pointers only; no user-window operations or Agent launches. Check JSON, PNG and exit status.

## Native Git panel acceptance

```powershell
cargo build --offline --locked --release --example git_probe
target/release/examples/git_probe.exe output/<unique-git-probe>
```

Run on Windows with Git for Windows installed; Git Bash can use the same commands. Native builds on macOS/Linux are unsupported. The probe creates an isolated fixture repository, uses production Git components and its own real PowerShell PTY, and writes twelve Chinese/English changes/history/collapsed screenshots with actual file-expansion clicks and Chinese/English hover cards at 100%/103%/114%/200%, including JetBrains Mono and Hack font inheritance plus result.json, checking totals and PTY PID. Synthetic pointer events target only its own egui context, without injecting system input or changing the clipboard; git_panel verifies full-ID copy output commands. Check normal exit and actual images; no model calls or user terminals/repositories are involved.

Git preview.2 keeps the git_probe commands above. Its commit fixture includes Markdown, JSON, configuration, PowerShell and media files; inspect file-type symbols, alignment with commit text and inline reading status. git_panel, git_repository and watch-module regressions cover layout and refresh gating.

Native terminal-history captures (Windows only; PowerShell/Git Bash): build `cargo build --offline --locked --release --example history_probe`, then run `target/release/examples/history_probe.exe output/<unique-history-probe>`. Fixed no-profile PowerShell output and probe-local egui drags generate nine oldest/middle/latest PNGs at 100%/103%/200%, checking offsets and the same PTY. No user windows, Agents or models are used. git_probe expands to 16 images, adding committed-file clicks and Chinese/English diffs at multiple scales. Check normal exit, owned PTY cleanup and actual images visually. Unified Windows/POSIX entry points and parameters are unchanged; native macOS/Linux actions remain explicitly unsupported.

git_probe keeps its existing command and now produces 20 images, adding Chinese/English settings.json and history.md file views at multiple scales. Verify that raw @@/mode rows are absent, syntax colors are distinct and indent guides are visible. Grammars come from the locked Syntect crate; builds need cached dependencies, while runtime uses no network or project syntax scripts. Unified Windows/POSIX entry points and parameters are unchanged.

git_probe now includes UTF-8-BOM JSON with normal Chinese. Its command and 20-capture count remain unchanged; check correct text and the active-file gray background/thin marker after the pointer leaves. Record invalid encoding separately from already-corrupted historical source; guessed substitutions are not source restoration.

For 0.7.0-preview.5, git_probe uses the same command and produces 22 images. Markdown table color emoji and text variants use the production file view at 100%/103%/114%/200% with JetBrains Mono/Hack. It uses only its isolated repository and owned PTY; verify normal exit and process reaping after captures.

For 0.7.0-preview.6, the same git_probe command produces 26 images, adding Changes file clicks and read-only additions/modifications/binary views. Check Untracked labels, the active-file marker, fonts and multiple scales. Commands and entry-point arguments are unchanged.

Native independent-scale checks: build with `cargo build --offline --locked --release --bins --example floating_scale_probe --example island_probe --example island_hint_probe`, then run `target/release/examples/floating_scale_probe.exe output/<unique-independent-scale>`. It generates 18 main/bar/hint images at six scale pairs, checking native dimensions, hint size, scaled-avatar activation and a non-repository header. Only synthetic metadata/own egui input is used, with no PTY, model or system input injection. island_probe retains minimize/close/reopen checks; island_hint_probe checks long hints. Entry points/arguments are unchanged and native actions remain Windows-only.

Native syntax verification (Windows only): build `cargo build --offline --locked --release --example git_syntax_probe`, then run `target/release/examples/git_syntax_probe.exe output/<unique-syntax-probe>` for 32 images covering 16 fixtures and two scales through the production read-only viewer. No scripts or models run. When maintaining syntax dependencies, `cargo run --offline --locked --example git_syntax_probe -- --export-notices assets/syntaxes/LICENSES.json` exports upstream notices without a GUI; then update SOURCE.json version, Cargo checksum and file hash. Existing Windows/POSIX package/verify-package actions carry/check these files without new parameters.

Native form verification (Windows): build `cargo build --offline --locked --release --example settings_probe --example floating_scale_probe`, then run `target/release/examples/settings_probe.exe output/<unique-forms>` and `target/release/examples/floating_scale_probe.exe output/<unique-pixels>`. The former captures 10 bilingual wide/narrow/confirmation images; the latter captures 18 independent-scale images and root-window same-size reference avatar coordinates for pixel comparison. No user tasks, real workspace edits or model calls; existing Windows/POSIX action parameters are unchanged.

Native Windows settings entry: `ai-terminal.exe --settings`, optionally with `--data-dir <directory>` for an isolated preview. Existing Windows/POSIX project actions and parameters are unchanged; non-Windows POSIX hosts still explicitly reject native GUI actions.

Native project-file checks (Windows): build with `cargo build --offline --locked --release --bins --example project_files_probe`, then run `target/release/examples/project_files_probe.exe output/<unique-project-files>`. Production tree/shared-panel/read-only document components and owned fixtures generate twelve bilingual, scaled and error-state captures. One real no-profile PowerShell PTY retains identity while viewing; the probe Job reaps only owned processes. No user configuration, fixture code or model is executed. Existing Windows/POSIX actions and arguments are unchanged; non-Windows native actions still fail explicitly.

The 0.9.0 README `files` mode creates fixed sample files only inside its owned output directory and reuses the production file tree and read-only document components. The public header uses a fixed example path, without reading user projects. Inspect both project-files native PNGs for the open-file marker, folder icons, syntax colors, indentation and emoji before copying unchanged and recording hashes.
