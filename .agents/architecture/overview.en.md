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
| `assets/fonts`, `assets/icons`, `assets/branding` | Pinned Hack fonts, Agent icons, and the selected original B application icon. |
| `examples/terminal_probe.rs`, `workspace_probe.rs` | Real TUI or local ANSI probes, controlled input, GPU screenshots and exit. |
| `examples/bash_probe.rs` | Real Git Bash and isolated HOME: startup configuration, directory notifications, Agent entries and native screenshots; no model request. |
| `examples/shell_switch_probe.rs` | Production App screenshots of nested-shell avatars and per-level return; isolated sessions, input only into its own window, no Agent/model startup. |

Data flow: ConPTY I/O thread → Alacritty grid → active viewport → GPU. A separate path carries session CLI events and Agent hooks → independent helper → authenticated TCP → bounded queue → `App::logic` → sidebar status. Shell prompts report directories through an OSC title with generation and sequence; there are no disk, process-list, or terminal-history scans. Event reduction must not depend on drawing or selection.

Preserve `src/`, `vendor/`, `tests/`, and `examples/` layout. Module tests stay with modules; cross-component Rust tests are in `tests/`. Python governance and package tests are also in `tests/`.

The 0.4.0 `i18n.rs` manages system/Chinese/English preferences; `settings.rs` provides settings; `floating.rs` creates a transparent always-on-top deferred viewport. Windows share only a session metadata snapshot and a bounded 32-item message channel, not terminal contents. Root logic continues when minimized; PTY and bridge notifications explicitly wake ROOT so events continue to be reduced.

Known limits: all physical IME combinations, all image clipboard sources, sustained high session concurrency, and future CLI protocol versions need further acceptance. Claude fullscreen startup and MOUSE_MOTION/SGR wheel protocol were checked, not all long-term interactions. The [GitHub repository](https://github.com/FengLP5263/ai-terminal) uses MIT; see the [contribution guide](../../CONTRIBUTING.en.md). Windows releases are on GitHub Releases; no cloud CI, automatic publishing, or product server is established. Earlier measurement records are not evidence for the current build.
