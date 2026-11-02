# Runtime, configuration, and operations

[简体中文](runtime.md) | English

## Supported environment

Verified on Windows x64 with Rust 1.99.1 MSVC, Visual Studio C++ Build Tools and Windows SDK. `Cargo.toml` declares minimum Rust 1.92; packages record the actual compiler and do not claim bit-for-bit reproducibility. Python 3.11+ supports governance and packaging. POSIX entry points do not imply native macOS/Linux GUI support.

## Processes and configuration

One manager owns several CLI/ConPTY sessions and short-lived hook helpers. Each session has a Windows Job Object; closing a session or app only reaps its own process tree. Never clean up PowerShell, Codex, or Claude by process name.

The authenticated bridge listens on a dynamic `127.0.0.1` port. It limits frames to 8,192 bytes and queues to 256 events; a full queue rejects explicitly and the helper retries finitely. Connect/read/write use short timeouts. The manager generates a token and passes it to its children.

| Name | Origin and purpose |
| --- | --- |
| `LOCALAPPDATA` | Default session metadata root, falling back to a temporary directory if absent. |
| `PATH`, `APPDATA`, `LOCALAPPDATA`, `USERPROFILE`, `SystemRoot`, `ProgramFiles` | Locate installed CLIs/Windows components; startup merges persistent registry user/system PATH. |
| `NPM_CONFIG_PREFIX`, `PNPM_HOME`, `NVM_SYMLINK`, `VOLTA_HOME`, `BUN_INSTALL` | Optional installation-directory hints; no defaults or user environment writes. |
| `AI_TERMINAL_ENDPOINT`, `AI_TERMINAL_TOKEN` | Per-manager loopback bridge endpoint/authentication; never commit values. |
| `AI_TERMINAL_SESSION`, `AI_TERMINAL_GENERATION` | Local session ID and launch generation. |
| `TERM`, `COLORTERM`, `NO_COLOR`, `FORCE_COLOR`, `CLICOLOR_FORCE` | Normalize this process's interactive terminal color capability. |
| `CLAUDECODE` | Prevent inheritance of the parent Claude nested-session marker. |
| `RC`, `CARGO_HOME`, `RUSTUP_HOME`, `AI_TERMINAL_PYTHON` | Optional resource compiler, Rust environment, and Python overrides; `.tools/env.ps1` is ignored. |

The manager does not implement a separate model connection or API-key template. It uses existing CLI authentication/settings. It does not alter user/system environment or Warp. Windows system font paths are currently fixed in source; non-default drives and other font environments are not verified.

## Startup, readiness, exit

`scripts/windows/project.ps1 run` builds and opens the interactive window. The plus menu chooses PowerShell, CMD, or Git Bash, with no directory or Agent form. The initial directory is the user home; later sessions use the current/recent valid directory. Reorder sessions by drag, edit names via pencil or context menu, collapse/expand the sidebar and resize its right edge. The context menu offers seven colors or default; selected rows are brighter and separators one physical pixel.

Ctrl+wheel or Ctrl+plus/minus scales the entire window (50%–250%); Ctrl+0 or the title-bar scale control resets it. The terminal recomputes its grid from remaining space. Opening the app starts only the last selected shell; others open when selected. Review Codex hooks in `/hooks` when necessary. Exit through the window after saving active work; no global reset deletes session data.

Readiness means an actual window/terminal and status message, not merely an existing process. Isolated smoke, measurements, and probes write JSON/PNG to unique ignored `output/` paths. The first UI marker is not a strict displayed first frame. Standard checks make no model requests; `verify-claude` requires explicit authorization and `AllowModelRequest`. `TextRendererSession` must be released in `on_exit` before TLS destruction; the helper initializes neither text nor GPU.

## Windows CLI discovery and shell sessions

Since 0.2.2, discover public exe/cmd/bat launchers using current PATH, persistent system/user PATH, then common user install directories. Preserve directory priority and prefer exe within a directory; do not scan disks, use npm private layout, execute profiles, or interpret empty/relative PATH entries as the user project. Only Windows launchers are supported; WSL-only CLIs and PowerShell functions/aliases are excluded. The merged PATH applies only to the child. Batch launchers use the console-subsystem helper `--launch-cli`, inherit ConPTY, preserve arguments and exit code, and remain in the owned Job Object.

Since 0.3.0, each session begins as a shell. Legacy Codex/Claude tabs load as PowerShell with name, directory, and history ID retained; the Agent is not started automatically. A prompt OSC `ai-terminal-cwd;<generation>;<sequence>;<directory>` updates directories without polling. Bash uses `pwd -W`. Per-session `codex`/`claude` helper launchers are generated in isolated runtime directories, preferring hard links and falling back to copies. Only the child PATH changes. Actual launch/exit notices determine Agent identity; hooks determine task status. Absolute paths, user-defined aliases, and other Agents are not auto-identified. Invocation IDs isolate stale and nested events. Claude settings are reused per terminal generation and explicit user `--settings` wins. Shell startup is controlled: PowerShell `-NoProfile`, CMD `/D`, Bash session rcfile; user profiles are not changed or loaded.

## Status and input evolution

0.3.3 stopped converting alternate-screen wheel events into history-arrow keys. Its temporary Codex `--no-alt-screen` behavior was removed in 0.3.7. Since 0.3.4, root app, visible debug windows, and examples share the embedded B icon (10 ICO sizes, 16–256); the helper has no GUI icon.

Since 0.3.5, Stop is Completed independently of unread state. A background completion shows a green unread dot; viewing the session clears that dot while retaining the completed status. The next UserPromptSubmit restores Working. Resume/clear/fork may change the foreground session ID; old events and child startups are filtered. The stable Codex command `& $env:AI_TERMINAL_HELPER --hook` reads the per-session helper location. No user hook trust file is rewritten; users upgrading from old definitions must review them in `/hooks`.

Since 0.3.6, clicking status explains the Agent connection and can copy `/hooks`, without entering or trusting it. Detection of an Agent is separate from receiving task notifications. A new manager excludes an inherited `AI_TERMINAL_SHIM_DIR` from discovery and child PATH so it does not mistake an older helper for a real CLI. Child `agent_id` events are ignored; a root `agent_type` is retained. The wheel is routed to a full-screen application that enables MOUSE_MODE; otherwise local history scrolls. Shift forces local history; Ctrl/Command remains host zoom.

Since 0.3.7, Codex keeps the default full-screen TUI and native scrolling. `--no-daemon` retains its engine/session environment within its Job; existing shared daemons are not controlled. Codex CLI 0.164.0 was checked; explicit user arguments still belong to the CLI. SessionStart may not appear until the first task, so the welcome page says it is waiting for task notifications; only suggest `/hooks` if events still do not arrive after work begins. The local egui-winit patch retains paste requests with no text. When an identified Codex/Claude terminal receives paste, the manager checks clipboard image-format metadata and sends native Alt+V; the CLI reads the image. Ctrl+V/Shift+Insert remain host entry points; normal shell/text/UI paste remains unchanged. No pixels are read, files created, or clipboard polling performed.

## 0.4.0 settings and floating monitor

The upper-left gear opens settings whether the sidebar is expanded or collapsed. Language follows Windows `GetUserDefaultUILanguage` or manually selected Simplified Chinese/English; unsupported system languages fall back to English. Selection applies immediately to manager controls/status, not CLI output, user names, or directories.

The optional floating monitor defaults off. It uses a transparent, rounded, always-on-top native deferred viewport without a taskbar button. Initially it is at the top of the root window's display work area. Drag its unmarked edge to move and save it; when reopened, correct it into an available work area. Zoom or session-count changes resize the native viewport and center it horizontally on its own display, using physical screen coordinates and target-display DPI while preserving physical vertical position. Avatars/status reuse the sidebar. Hover shows name/status; up to 10 avatars are visible at once; additional sessions are reached by dragging the avatar region. Clicking an avatar activates the existing PTY and restores/focuses the root window; settings disables the monitor.

Without active animation or dragging there is no continuous repaint. Working animation schedules the child around every 80 ms; background notifications wake root explicitly. A minimized root still reduces bridge events, updates snapshots, and saves preferences without unnecessary root animation repaint. The monitor does not poll clipboard, process trees, or terminal content.

The Windows main title bar explicitly uses the dark theme. After a normal window is dragged across displays and released, its bounds are fitted to the work area of the display under the pointer; DPI changes settle before repositioning or resizing. Windows manages maximized windows, and drags within the same display are left alone. Only left-button state and the pointer position at the end of a drag are checked; no input is recorded.

## Attention-state signals

Codex PermissionRequest also fires before automatic review, so it does not alone prove that a user prompt appeared. Managed Codex processes use per-invocation `tui.notifications`, `tui.notification_method`, and `tui.notification_condition` arguments to receive actual user-facing OSC 9 notifications. Only known attention prefixes are recognized; message content is not stored and notices are coalesced by invocation. Explicit later CLI arguments can override these settings. Some Codex prompt types or future versions may not emit a recognizable notification; already-running processes must be restarted for the new arguments.

Claude PermissionRequest/Elicitation can likewise be resolved automatically by other hooks. Notification confirms actual permission or elicitation prompts, typically after about six seconds for permission prompts. PermissionDenied and PostToolBatch restore state after a denial or completed tool batch. Claude launched with explicit `--settings` still uses the user's settings without injecting project hooks. After a prompt is resolved, a CLI without a close event may not return to Working until the associated tool or batch ends; this is not a complete realtime status protocol.
