# Architecture overview

[简体中文](overview.md) | English

The current version is in `Cargo.toml`; the initial baseline was 0.1.2. Windows x64, Rust 2024, eframe/egui 0.35, and Alacritty 0.26 are used. The default renderer is glow/OpenGL; wgpu/DX12 is optional for local comparison. `Cargo.lock` pins dependencies.

| Boundary | Responsibility |
| --- | --- |
| `build.rs` | Compile Windows icon resources into the app and debug examples; keep the helper independent. |
| `src/main.rs`, `app.rs` | Native windows, session list, terminal controls, background event reduction. |
| `src/sidebar.rs`, `zoom.rs` | UUID navigation, drag ordering, avatars, whole-window zoom. |
| `src/appearance.rs`, `chrome.rs`, `ui_text.rs` | Shared fonts/theme, avatars, vector controls, compact dimensions, and color emoji in names. |
| `src/status.rs`, `connection_ui.rs` | Lifecycle state, parallel waiting, stale-turn/child filtering, unread status and connection guidance. |
| `src/bridge.rs`, `bin/ai-terminal-hook.rs` | Authenticated loopback metadata bridge and independent hook helper. |
| `src/discovery.rs`, `launch.rs`, `shell.rs` | Public CLI/shell entry points, session-scoped integration, directory updates, Agent lifecycle metadata. |
| `src/model.rs`, `platform.rs` | Atomic session metadata persistence; Windows Job Objects and manager memory observation. |
| `vendor/egui_term`, `vendor/egui-winit` | Alacritty/ConPTY and native text/input rendering; pinned egui-winit 0.35.0 paste patch. |
| `assets/fonts`, `assets/icons`, `assets/branding` | Pinned default JetBrains Mono and optional Cascadia Mono/Hack fonts, Agent icons, and the selected original B application icon. |
| `examples/terminal_probe.rs`, `workspace_probe.rs` | Real TUI or local ANSI probes, controlled input, GPU screenshots and exit. |
| `examples/bash_probe.rs` | Real Git Bash and isolated HOME: startup configuration, directory notifications, Agent entries and native screenshots; no model request. |
| `examples/shell_switch_probe.rs` | Production App screenshots of nested-shell avatars and per-level return; isolated sessions, input only into its own window, no Agent/model startup. |

Data flow: ConPTY I/O thread → Alacritty grid → active viewport → GPU. A separate path carries session CLI events and Agent hooks → independent helper → authenticated TCP → bounded queue → `App::logic` → sidebar status. Shell prompts report directories through an OSC title with generation and sequence; there are no disk, process-list, or terminal-history scans. Event reduction must not depend on drawing or selection.

Preserve `src/`, `vendor/`, `tests/`, and `examples/` layout. Module tests stay with modules; cross-component Rust tests are in `tests/`. Python governance and package tests are also in `tests/`.

The 0.4.0 `i18n.rs` manages system/Chinese/English preferences; `settings.rs` provides settings; `floating.rs` creates a transparent always-on-top deferred viewport. Windows share only a session metadata snapshot and a bounded 32-item message channel, not terminal contents. Root logic continues when minimized; PTY and bridge notifications explicitly wake ROOT so events continue to be reduced.

Known limits: all physical IME combinations, all image clipboard sources, sustained high session concurrency, and future CLI protocol versions need further acceptance. Claude fullscreen startup and MOUSE_MOTION/SGR wheel protocol were checked, not all long-term interactions. The [GitHub repository](https://github.com/FengLP5263/ai-terminal) uses MIT; see the [contribution guide](../../CONTRIBUTING.en.md). Windows releases are on GitHub Releases; no cloud CI, automatic publishing, or product server is established. Earlier measurement records are not evidence for the current build.

## Windows compatibility fixes (unreleased)

`src/window_frame.rs` requests dark DWM captions for the application's own window and restores the policy after native theme/settings notifications. `src/native_console.rs` checks bundled companions before creating a session. `assets/windows/conpty/` contains the pinned Microsoft native console, license and provenance. Alacritty loads it from the executable directory; build.rs stages the same version for the app, tests and examples.

`examples/composer_probe.rs` covers restored 151.13608% zoom followed by 125%, 150% and 151.13608% fullscreen composers. `titlebar_probe.rs` captures only its own native window and caption. `discovery_probe.rs` calls installed CLIs only with public --version. Record synthetic fixtures, public version checks and actual model turns separately.

`vendor/winit` is an Apache-2.0 local snapshot pinned to 0.30.13. During Windows native movement, use the OS-suggested DPI rectangle to avoid repeated DPI transitions caused by upstream nudging toward the old monitor. Main and floating windows share this path; non-drag behavior remains upstream. Provenance, archive hash and scope are in that directory's UPSTREAM.md.

0.4.8 preview adds src/integration.rs for invocation-only lifecycle environment and credential-free basic settings snapshots. Native notifications retain Codex/Kimi app identity. TRAE/community ZCode keep the independent lightweight helper and authenticated bridge, without a proxy service or terminal-body scanning.

0.4.8-preview.2: src/floating_hint.rs displays full floating-bar hints in an owned native tooltips_class32 window, outside the capsule client bounds, without a browser or another GPU viewport. 0.4.8 restores the shared explicit-event reducer for ZCode and other hook Agents, without downgrading working events.

0.4.9 preview: hover hints use an owned non-activating egui native viewport sharing production text and color emoji. The root synchronizes hover changes and removes the hint viewport on leave. Local alacritty_terminal/vte patches only add private cursor-position queries; see their UPSTREAM.md for provenance and licenses.

0.4.9-preview.4: main App UI and background logic consume only the ROOT viewport; floating/hint geometry and minimized state cannot relayout the main terminal. TerminalView submits ConPTY size only for real visible layouts with a valid screen rectangle and no discard already requested; measurement passes leave the running grid unchanged.

0.4.9-preview.5: Windows egui_glow batches visible paint geometry and rebases indices before drawing, preserving texture, clipping and callback order. CPU/GPU capacity follows the largest visible frame; terminal history is not copied and production repaint behavior is unchanged. The independent render_update_probe checks transient corruption using a fixed table and a changing glyph region.


0.4.10 preview: `font_choice.rs` defines stable font IDs persisted in `UiSettings.terminal_font`. appearance applies the same definitions to the app, terminals and probes; UI emoji use the current terminal native font to avoid repeated atlas recreation. NSIS owns per-user installation/uninstallation independently of runtime and Agent integrations; ZIP remains available.

0.4.12 preview: `src/git/` owns read-only local Git snapshots for the active directory, bounded background commands and Windows directory notifications. `src/git_ui.rs` provides the header summary and collapsible right panel. Branches, line totals and history do not affect Agent status, and panel changes do not recreate terminals.

Git history inspection: `src/git/commit.rs` reads immutable commits through independent bounded request/result slots and cancellation generations. `git_ui/history.rs` draws the timeline, interactive cards and expanded files using production fonts and Codicons. App::logic alone consumes results; UI and terminal identity remain independent.

0.7.0 preview: egui_term view/scrollbar.rs uses O(1) history range and offset for local scrolling; ScrollTo never reaches the PTY. src/git/patch.rs uses independent bounded request/result slots for committed-file diffs; git_ui/diff.rs lays out visible rows only. Showing a diff disables terminal paint wakeups while retaining output draining and the original process.

0.7.0-preview.3: git/syntax.rs uses pinned Syntect 5.3.0 default grammars and project-owned color rules in the file worker. It produces bounded presentation metadata containing source-row indices, byte ranges/RGB and indentation, with independent old/new parser states and no full-file UI scan. git_ui/diff.rs virtualizes multicolor LayoutJobs and indent guides, filtering raw Git metadata from the body.

preview.6: git/worktree.rs reads HEAD-to-current-file net diffs on demand, sharing patch.rs parsing, limits and background slots with historical files. PatchSource explicitly distinguishes Commit and WorkingTree. Reads do not write the index/disk caches or invoke external drivers; Git snapshot events refresh an open working file, while UI frames do not trigger reads.

0.8.0-preview.2: git/syntax.rs uses pinned two-face bat syntaxes with Oniguruma instead of the default set and JS approximation for TS. Bounded snapshots are still prepared by the file-query worker; no scripts, LSP, plugins or user syntax settings run. chrome::paint_shell shares existing standard Codicons glyphs across all shell entry points and avatars.

0.8.0-preview.3: the bar lays out directly at its own scale without transforming rasterized layers. Icons requests textures/Codicons at the physical avatar size; badges and hit regions share those coordinates. dialogs.rs centralizes form typography, 44-point controls, spacing and surfaces; settings.rs uses fixed columns and narrow-screen stacking. Styles remain local to dialogs and do not alter terminal text.

0.9.0 preview: project_files.rs owns one background browser worker, at most 64 cached directories and coalesced selected-file requests/results. project_ui.rs shares the Files / Git right panel and virtualizes the lazy tree. Ordinary documents and Git diffs share git_ui::diff::body, syntax preparation, indent guides and emoji; file_icons.rs centralizes standard Codicons file glyphs.

0.9.1-preview.3: file_icons.rs maps filenames/extensions to embedded Seti glyphs and upstream palette colors, shared by all file rows and read-only titles. Folders/actions retain Codicons. No dependencies, file scanning, configuration reads or SVG runtime are added.

0.9.1-preview.4: file_icons.rs shares Tabler SVGs, bounded parsed/texture caches isolated by type/color, and final-pixel rasterization. Only embedded icons are used, without user SVG handling or a browser. File/diff titles align icon and name to the left; the header order is Git, project, status, zoom.

0.10.0 preview: floating background opacity updates immediately through FloatingBar shared scalar state, reusing the existing transparent native viewport, settings autosave and conditional wakeups, without additional windows or timers.

0.10.0-preview.2 keeps floating transparency composition in the owned floating_native window lifecycle, applying it only on creation and composition notifications without polling. The ignored-file icon retains Tabler file-off with a neutral gray runtime tint.

0.10.0-preview.3: background directory reads reuse Git Runner for a NUL-delimited check-ignore batch. Entry.ignored stores only positive results in the existing cache; project_ui draws muted names/icons and a tooltip. floating_native disables system non-client rendering and insets only for its own capsule; application rendering retains the corners.

0.10.0-preview.4: header order is project, Git, status, zoom. Both entry widths are reserved before allocating space from the right.

0.10.0-preview.5: dialogs.rs centralizes 18/16/14/12-point dialog typography and 36-point controls; settings.rs shares title and supporting-text constants. Styling remains scoped to dialogs.

In 0.11.0, model owns UUID group navigation metadata and sidebar provides context-menu management and labels. floating projects each group to its first session in session order, with a simplified group badge and highest-priority actual member state so pending work stays visible; it creates no team-completed event. floating_group is an interactive, non-activating native viewport owned by the bar. It shares only member metadata and bounded activation messages, with no terminal content or extra terminal instances.
