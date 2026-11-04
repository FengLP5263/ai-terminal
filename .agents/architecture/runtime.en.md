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
| `AI_TERMINAL_SHELL_INVOCATION`, `AI_TERMINAL_REAL_*` | Current nested-shell invocation and resolved executable paths, scoped to this child environment. |
| `TERM`, `COLORTERM`, `NO_COLOR`, `FORCE_COLOR`, `CLICOLOR_FORCE` | Normalize this process's interactive terminal color capability. |
| `CLAUDECODE` | Prevent inheritance of the parent Claude nested-session marker. |
| `RC`, `CARGO_HOME`, `RUSTUP_HOME`, `AI_TERMINAL_PYTHON` | Optional resource compiler, Rust environment, and Python overrides; `.tools/env.ps1` is ignored. |

The manager does not implement a separate model connection or API-key template. It uses existing CLI authentication/settings. It does not alter user/system environment or Warp. Windows system font paths are currently fixed in source; non-default drives and other font environments are not verified.

## Startup, readiness, exit

`scripts/windows/project.ps1 run` builds and opens the interactive window. The plus menu chooses PowerShell, CMD, or Git Bash, with no directory or Agent form. The initial directory is the user home; later sessions use the current/recent valid directory. Reorder sessions by drag, edit names via pencil or context menu, collapse/expand the sidebar and resize its right edge. The context menu offers seven colors or default; selected rows are brighter and separators one physical pixel.

Expanded and collapsed sidebars share a fixed 3-point scrollbar, about one logical point inside the divider. Hovering/scrolling changes opacity without widening it. A dedicated gutter keeps it clear of avatars and status dots; wheel and thumb dragging remain available. This style is scoped to the sidebar. When expanded, a separate drag target outside the divider resizes the panel (190–420 logical points) without stealing scrollbar input.

Ctrl+wheel or Ctrl+plus/minus scales the entire window (50%–250%); Ctrl+0 or the title-bar scale control resets it. The terminal recomputes its grid from remaining space. Opening the app starts only the last selected shell; others open when selected. Review Codex hooks in `/hooks` when necessary. Exit through the window after saving active work; no global reset deletes session data.

Readiness means an actual window/terminal and status message, not merely an existing process. Isolated smoke, measurements, and probes write JSON/PNG to unique ignored `output/` paths. The first UI marker is not a strict displayed first frame. Standard checks make no model requests; `verify-claude` requires explicit authorization and `AllowModelRequest`. `TextRendererSession` must be released in `on_exit` before TLS destruction; the helper initializes neither text nor GPU.

## Windows CLI discovery and shell sessions

Since 0.2.2, discover public exe/cmd/bat launchers using current PATH, persistent system/user PATH, then common user install directories. Preserve directory priority and prefer exe within a directory; do not scan disks, use npm private layout, execute profiles, or interpret empty/relative PATH entries as the user project. Only Windows launchers are supported; WSL-only CLIs and PowerShell functions/aliases are excluded. The merged PATH applies only to the child. Batch launchers use the console-subsystem helper `--launch-cli`, inherit ConPTY, preserve arguments and exit code, and remain in the owned Job Object.

Since 0.3.0, each session begins as a shell. Legacy Codex/Claude tabs load as PowerShell with name, directory, and history ID retained; the Agent is not started automatically. A prompt OSC `ai-terminal-cwd;<generation>;<sequence>;<directory>` updates directories without polling. Bash uses `pwd -W`. Per-session `codex`/`claude` helper launchers are generated in isolated runtime directories, preferring hard links and falling back to copies. Only the child PATH changes. Actual launch/exit notices determine Agent identity; hooks determine task status. Absolute paths, user-defined aliases, and other Agents are not auto-identified. Invocation IDs isolate stale and nested events. Claude settings are reused per terminal generation and explicit user `--settings` wins.

PowerShell uses `-NoProfile` and CMD uses `/D`. The Git Bash session rcfile loads `/etc/profile`, then the first readable `~/.bash_profile`, `~/.bash_login`, or `~/.profile`, following login-file precedence; that user file decides whether to source `~/.bashrc`. Preserve user PS1, aliases, completion and scalar/array PROMPT_COMMAND before appending directory notifications. After profiles, restore only this session's Agent entry directory to the front of the child PATH. CHERE_INVOKING preserves the selected directory and MSYSTEM follows the Git installation architecture. No user configuration is edited; startup scripts still execute their own behavior. Shutdown reaps only owned Jobs. Running CLI/console handles and prompt/result text are not persisted.

The Git Bash menu requires Git for Windows. Discovery supports current/persistent PATH (including Git's cmd directory), both HKCU/HKLM registry views of GitForWindows/InstallPath, environment-based system/user locations and the user's Scoop directory. Portable installations must expose Git's cmd or bin on PATH. Do not scan disks or treat WSL bash.exe aliases or unrelated Bash distributions as Git Bash. If missing, show an explicit error and offer PowerShell, or CMD if PowerShell is unavailable. Clicking changes the actual shell type, default name and icon, preserving UUID, directory, color, custom name and conversation ID. With no available fallback, retain the error; never install software automatically.

Session-local entries also recognize interactive nested powershell, cmd and bash calls; shell entries are created only when the corresponding installation is found. Forward the original arguments, sending ShellStarted on entry and ShellEnded on return. Avatar priority is the foreground Agent, then the current nested shell, then the original root shell; exit restores each level, shared by sidebar and floating bar. Only managed entries and explicit interactive flags are supported. One-off `-Command`, `-File`, `/c` or `-c` calls, redirected streams and shells inside Agent tools do not switch the avatar. At launch, read only the helper's own console process IDs to confirm root PTY membership, without scanning system processes or reading input/output. Keep at most 32 transient identity frames; reject stale generations, mismatched parents and detached-console notices. Nested identity never changes the persisted root shell or implies AI task start/completion. Directory notices still depend on actual prompt integration because original shell arguments remain unchanged; absolute paths and custom aliases are not presented as supported entries. AI_TERMINAL_SHELL_INVOCATION and AI_TERMINAL_REAL_* carry the parent shell invocation and resolved executable paths only in this child environment.

## Status and input evolution

0.3.3 stopped converting alternate-screen wheel events into history-arrow keys. Its temporary Codex `--no-alt-screen` behavior was removed in 0.3.7. Since 0.3.4, root app, visible debug windows, and examples share the embedded B icon (10 ICO sizes, 16–256); the helper has no GUI icon.

Since 0.3.5, Stop is Completed independently of unread state. A background completion shows a green unread dot; viewing the session clears that dot while retaining the completed status. The next UserPromptSubmit restores Working. Resume/clear/fork may change the foreground session ID; old events and child startups are filtered. The stable Codex command `& $env:AI_TERMINAL_HELPER --hook` reads the per-session helper location. No user hook trust file is rewritten; users upgrading from old definitions must review them in `/hooks`.

Since 0.3.6, clicking status explains the Agent connection and can copy `/hooks`, without entering or trusting it. Detection of an Agent is separate from receiving task notifications. A new manager excludes an inherited `AI_TERMINAL_SHIM_DIR` from discovery and child PATH so it does not mistake an older helper for a real CLI. Child `agent_id` events are ignored; a root `agent_type` is retained. The wheel is routed to a full-screen application that enables MOUSE_MODE; otherwise local history scrolls. Shift forces local history; Ctrl/Command remains host zoom.

Since 0.3.7, Codex keeps the default full-screen TUI and native scrolling. `--no-daemon` retains its engine/session environment within its Job; existing shared daemons are not controlled. Codex CLI 0.164.0 was checked; explicit user arguments still belong to the CLI. SessionStart may not appear until the first task, so the welcome page says it is waiting for task notifications; only suggest `/hooks` if events still do not arrive after work begins. The local egui-winit patch retains paste requests with no text. When an identified Codex/Claude terminal receives paste, the manager checks clipboard image-format metadata and sends native Alt+V; the CLI reads the image. Ctrl+V/Shift+Insert remain host entry points; normal shell/text/UI paste remains unchanged. No pixels are read, files created, or clipboard polling performed.

Recognized Codex/Claude sessions prefer ordinary left-drag text selection; Ctrl+C copies the selection and still interrupts the CLI when nothing is selected. With application mouse tracking enabled, movement of at least 3 logical points starts local selection without leaking click/drag reports. A click without dragging forwards a press/release pair on release. Wheel events still go to the application. Other terminal programs retain their mouse protocol, with Shift forcing local selection. This uses the existing terminal grid and selection, without adding conversation logs.

## 0.4.0 settings and floating monitor

The upper-left gear opens settings whether the sidebar is expanded or collapsed. Language follows Windows `GetUserDefaultUILanguage` or manually selected Simplified Chinese/English; unsupported system languages fall back to English. Selection applies immediately to manager controls/status, not CLI output, user names, or directories.

The optional floating monitor defaults off. It uses a transparent, rounded, always-on-top native deferred viewport without a taskbar button. Initially it is at the top of the root window's display work area. Drag its unmarked edge to move and save it; when reopened, correct it into an available work area. Zoom or session-count changes resize the native viewport and center it horizontally on its own display, using physical screen coordinates and target-display DPI while preserving physical vertical position. Avatars/status reuse the sidebar. Hover shows name/status; up to 10 avatars are visible at once; additional sessions are reached by dragging the avatar region. Clicking an avatar activates the existing PTY and restores/focuses the root window; settings disables the monitor.

Without active animation or dragging there is no continuous repaint. Working animation schedules the child around every 80 ms; background notifications wake root explicitly. A minimized root still reduces bridge events, updates snapshots, and saves preferences without unnecessary root animation repaint. The monitor does not poll clipboard, process trees, or terminal content.

The Windows main title bar explicitly uses the dark theme. After a normal window is dragged across displays and released, its bounds are fitted to the work area of the display under the pointer; DPI changes settle before repositioning or resizing. Windows manages maximized windows, and drags within the same display are left alone. Only left-button state and the pointer position at the end of a drag are checked; no input is recorded.

## Attention-state signals

Codex PermissionRequest also fires before automatic review, so it does not alone prove that a user prompt appeared. Managed Codex processes use per-invocation `tui.notifications`, `tui.notification_method`, and `tui.notification_condition` arguments to receive actual user-facing OSC 9 notifications. Only known attention prefixes are recognized; message content is not stored and notices are coalesced by invocation. Explicit later CLI arguments can override these settings. Some Codex prompt types or future versions may not emit a recognizable notification; already-running processes must be restarted for the new arguments.

Claude PermissionRequest/Elicitation can likewise be resolved automatically by other hooks. Notification confirms actual permission or elicitation prompts, typically after about six seconds for permission prompts. PermissionDenied and PostToolBatch restore state after a denial or completed tool batch. Claude launched with explicit `--settings` still uses the user's settings without injecting project hooks. After a prompt is resolved, a CLI without a close event may not return to Working until the associated tool or batch ends; this is not a complete realtime status protocol.

## Multi-session shutdown

Closing first asks for confirmation and does not stop sessions early. The confirmation paints the active terminal's latest snapshot without waiting for its grid lock while output continues draining. Once confirmed, session metadata is saved in the background and up to four owned jobs and PTYs are closed concurrently; the responsive window displays progress. A save or cleanup failure keeps the window open for a retry. Unrelated processes are never scanned or terminated. Forced termination does not guarantee persistence.
## Windows OpenGL buffers

On some Windows OpenGL drivers, native glyph drawing can cause sustained growth in process-private memory. The local `egui_glow` patch reuses and grows vertex and index buffers while preserving textures, draw order, and glyph content; other targets retain the upstream path. Memory checks distinguish the manager from Agent children and use isolated sessions with native windows.

## Windows compatibility fixes (unreleased)

Packages use pinned x64 conpty.dll and OpenConsole.exe from Microsoft.Windows.Console.ConPTY 1.25.260930003. The inbox Windows 10 console drops fullscreen/mouse modes in native fixtures, whereas the same program passes with these companions. They remain beside the app without changing system consoles, default terminal, PATH or user configuration; missing companions produce an explicit error before session creation. build.rs stages them in the actual Cargo profile, examples and deps directories, skipping identical content.

Dark captions use DwmSetWindowAttribute on the current window: attribute 20 first, then legacy Windows 10 attribute 19. A window subclass reapplies this after default handling of WM_THEMECHANGED/WM_SETTINGCHANGE and removes itself on WM_NCDESTROY. There is no idle polling or global personalization change. Unsupported attributes retain the backend theme.

CLI discovery skips standard npm .cmd/.bat launchers whose literal targets are confirmed missing, preserving directory precedence. It executes no health probe and guesses no private package layout, inspecting only literal targets in recognized public templates. Custom, unknown or unreadable scripts retain precedence; nonzero CLI exit codes must never trigger execution of another entry. If all recognized launchers are stale, the error lists launcher/target paths. Cached session entries are rechecked at invocation.

UI repair: collapsed selection tiles are square; the scrollbar disappears outside the sidebar without changing width or gutter. Chrome owns shared status colors. Floating position commands use the existing child window DPI; reapply logical dimensions after its DPI changes. The missing Codex agents entry is an independent-mode limitation, not bottom clipping; see [UI rendering](../../docs/ui-rendering.en.md).
Collapsed sidebar tiles are fixed at 44×44 logical pixels; never derive row height from the current scroll-area width, which creates a continuous scrollbar/layout feedback loop near overflow. Geometry regressions cover threshold counts, consecutive idle frames, zoom, and hover; idle layout must not continuously request immediate repaint.

In preview.3, observed floating-window positions are no longer fed back as per-frame placement commands. Only creation, zoom or session-count changes request placement. `floating_native.rs` caches/checks our own window on its UI thread, reads the native physical rectangle, and handles native movement enter/exit, DPI and size events. Client-size reconciliation is coalesced after backend handling; placement is deferred during movement and subclass state is released on destruction, without polling other windows or processes. The three collapsed-sidebar controls share session geometry: 44×44 tiles, 32-point circular surfaces, 20-point glyphs and 4-point gaps.

Preview.4: the local winit patch directly uses the WM_DPICHANGED OS-suggested rectangle during native movement, preserving the window's mouse anchor and avoiding recursive rescaling from old-monitor nudges. All additional floating size repairs wait until after WM_EXITSIZEMOVE; no extra mid-drag correction is added to the main window. Expanded-sidebar cards retain approximately 8 logical pixels on both sides while the scrollbar stays inside the divider.

## 0.4.5 native Codex lifecycle and agents overview

Managed `codex` no longer injects `--no-daemon`, status hooks or notification `-c` overrides by default, retaining the CLI's shared-server policy and `← for agents`. The current PTY reports explicit OSC 7501 lifecycle: idle, working, blocked (permission/question/auth), done and error; clear only clears terminal state and is not completion. After native lifecycle arrives, delayed legacy hooks/OSC 9 cannot overwrite it. Invocation epochs isolate notices; at most 32 states and coalesced ROOT wakes are retained, consumed by logic even when hidden. No transcript heuristics or polling are introduced.

Older CLIs without this protocol can explicitly run `codex --no-daemon` to retain session-scoped hooks and `/hooks` review without the shared overview. No automatic CLI health queries, model-command retries, user-config changes or trust changes occur. User profiles/overrides remain subject to Codex's shared-server policy. Native lifecycle carries no conversation UUID and does not invent or overwrite known recovery IDs; resume/agents in Codex handles conversation recovery and switching. Codex owns its shared server: application shutdown reaps only owned terminals/helpers, leaving the user's shared service and potentially running tasks intact.

## Session unread badges and drag ordering

Failure red dots and completion green dots both indicate unread results. Viewing a session, or receiving a result in the focused session, clears the badge while retaining the failed/completed state. Waiting yellow dots and working animations still represent the current state. Agent failures, terminal startup failures and abnormal process exits share this unread behavior; the sidebar and floating rail use the same painter.

Sidebar dragging projects metadata order only. A foreground card preserves the pointer's original grab offset, a full-size destination gap makes other cards move aside, and fixed row slots determine the destination to prevent feedback from moving hitboxes. Release commits the real UUID order; Escape or release outside cancels. Expanded and collapsed views behave alike without changing PTYs, the active session or status. Only a held card near an overflowing list's edge scrolls further; cancellation leaves no idle repaint loop.

## 0.4.6 direct Codex startup and Qoder CN entries

Observe strictly matched native Codex status as soon as a shell starts; only an `app=codex` state establishes runtime identity. Absolute paths, aliases and `--remote … resume …` can therefore connect without rewriting commands or injecting hooks. An app-less clear, ordinary output and OSC 9 do not establish identity or replace another recognized Agent. Clear may be a temporary terminal handoff, so it is not exit evidence. Codex without a helper invocation restores shell identity when this generation's shell prompt notification arrives, resets the notification epoch and keeps observing subsequent calls. Saved conversation UUIDs remain unchanged.

Enter `qodercn` or `qoderclicn` in a normal shell. The per-session helper forwards the real CN CLI and reports invocation-scoped start/exit. Sidebar and floating rail share its avatar/status metadata without renaming sessions. Explicit user `--settings` determines task hooks; without events, keep the pending state rather than assuming work or completion. A missing CLI produces a discovery error without automatic installation, login or permission changes.

Local 0.4.6-preview.1 checks passed 166 Rust and 13 Python regressions, governance, formatting and Clippy; Release smoke and seven synthetic state captures were inspected. Real Qoder CN 1.1.82 startup completed authentication and displayed directory trust confirmation. The probe did not approve it or receive SessionStart, so real hook/model-turn integration remains unverified. The user reviews trust and subsequent tasks inside the CLI.

0.4.6: The user accepted the Qoder preview. Final checks passed 168 Rust and 13 Python regressions, governance/formatting/Clippy, Release smoke and native captures. The production App verified real Codex 0.166.0 absolute-path/--remote unix:// startup, avatar, idle and agents entry. A fake CLI in a real ConPTY separately covers resume arguments, waiting/completion, clear, shell return and repeated invocation. No model prompt was sent; neither evidence category is a real model-turn pass.

Before release, the user also tested recovery of an existing Codex conversation, confirmed that it works correctly and authorized commit/release. This acceptance is separate from developer startup captures and synthetic events; it does not extend verification to long-duration load or other platforms.

## Unreleased: additional CLI identity and first-run shortcut

Managed shells gain `kimi.exe`, `zcode.exe` and `traecli.exe` entries backed by `AI_TERMINAL_REAL_KIMI`/`AI_TERMINAL_REAL_ZCODE`/`AI_TERMINAL_REAL_TRAE`. The shim only forwards the real program and reports ShellAgentStarted/ShellAgentEnded per invocation; it injects no arguments, copies no user configuration and edits none. Task status remains "Awaiting task events", with each CLI's hook limitation explained in the status menu.

On first launch with the default data directory, the app tries to create a desktop shortcut off the main thread: `IShellLinkW`/`IPersistFile` target the current `ai-terminal.exe` through the Windows Desktop known folder (honoring OneDrive redirection) with no arguments or credentials. `UiSettings.desktop_shortcut` is recorded before the attempt, so failure neither retries nor blocks startup. `--data-dir`, isolated smoke/idle measurements and failed workspace loads skip the attempt, and a user-deleted shortcut is not recreated. Failure is reported only to stderr and touches nothing outside the Desktop.
