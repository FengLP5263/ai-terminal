# Testing and acceptance

[简体中文](SKILL.md) | English

```powershell
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 measure-idle -Release
```

See [scripts](../../../scripts/README.en.md) for the POSIX counterpart and platform limits. Preserve the module and `tests/` layout. Standard tests include the Rust workspace and Python package-verification tests, without model API calls. Codex configuration tests may skip when the CLI is absent; record installed-CLI and live-turn checks separately.

Native smoke runs six real PowerShell PTYs with synthetic lifecycle events (covering all six Agent avatars) in a unique isolated `output/` directory. Verify JSON, PNG, exit, and cleanup of the PTYs it owns. On failure, stop only processes started by that test. Distinguish manager from Agent subprocesses, debug from release, GPU, measurement window, and version when reporting performance.

Native Git Bash regressions use a temporary HOME and synthetic startup files, never the developer's real profiles. Cover login-file precedence, PS1, aliases, completion, scalar/array PROMPT_COMMAND, directory tracking and Agent entries. Discovery/fallback checks cover portable paths, rejection of unrelated Bash, missing Git/PowerShell, actual fallback shell identity and retained session data. If Git is absent, explicitly record that native Bash checks did not run; do not claim all-device validation.

Nested-shell checks use real CMD→PowerShell→CMD and a local Agent fixture, verifying parent correlation, root console membership, per-level return and retention of the original PTY. PowerShell tests use -NoProfile and Git Bash uses a temporary HOME. Identity reduction tests separately cover detached consoles, stale parents and Agent priority; synthetic events are not real model turns.

Run TUI probes with unsubmitted text only in a previously trusted directory; never automatically accept CLI trust or authorization. Real model turns use the user's account and can incur charges: they require explicit authorization and `verify-claude -AllowModelRequest`, with a $0.15 limit and tools disabled. They are not default CI.

Run screenshots and idle measurements after optimized builds complete. `terminal_probe` and `workspace_probe` must wait for real CLI or sample content; blank screenshots do not pass. Check native probe exit codes and release `TextRendererSession` in `on_exit`. Text changes cover color emoji, modifiers/ZWJ, caret, and terminal boundaries at multiple zoom levels.

`scroll_probe` uses a local raw-console pager to test application scrolling, normal/SGR mouse, MOUSE_MOTION, Shift local history, and Ctrl zoom; do not synthesize history-arrow keys. A real Claude startup check is not a completed model-task test.

For drag-selection changes, terminal_interaction uses real ConPTY tests for normal/SGR modes, 1002/1003, ordinary clicks, Shift selection, copying a selection and interrupting without one. scroll_probe also captures actual left-drag highlighting without overwriting the system clipboard. Tests enabling the preference do not replace checking its integration with recognized Codex/Claude in the production App.

For window/image paste changes also run `cargo test --offline --locked -p egui-winit --lib paste_tests`; this patch is outside the workspace and must be included explicitly. Image probes read the current clipboard only for explicit paste checks and do not press Enter. `lifecycle_probe offline` sends a fixed example only to a temporary local fake Responses service and records fake model turns separately from real hook events; it does not call an actual inference service or accept trust.

For 0.4.0 settings/floating-window work, run `examples/island_probe.rs` with an isolated output directory: Chinese/English screenshots, minimized root, floating visibility/topmost, click-to-restore, and close/reopen. States are synthetic; no Agent/model is started. `tests/settings_floating.rs` covers old settings, persistence, control selection, and clicking after horizontal scrolling. App tests check actual owned PTY PIDs. Probes read only their own window properties and do not inject system input or touch user windows. `island_zoom_probe` covers viewport size, centering, and DPI as described in [scripts](../../../scripts/README.en.md).

Run composer_probe, titlebar_probe and discovery_probe separately on Windows 10/11; commands are in scripts/README.md. composer_probe checks the restored 151.13608% composer, transitions through 125%/150%/151.13608%, final rows and viewport bounds. titlebar_probe captures only its own window and checks caption/background buttons before and after theme notifications. discovery_probe uses --version only, not a model turn. AI_TERMINAL_MOUSE_TRACE may record modes and at most 256 synthetic input bytes in the independent mouse_console fixture, never user CLIs or real sessions. After console-runtime upgrades, verify normal exit and owned PTY reclamation.
Cross-monitor floating checks: build and run examples/island_move_probe.rs (commands in scripts/README.en.md), requiring at least two physical monitors. Move only the probe native window, checking client dimensions and avatar screenshots at zoom levels including 71%, 142%, and 200%. No user input injection or Agent launch; missing a second monitor fails explicitly.
Collapsed-sidebar repairs must cover the overflow boundary: run sidebar_interaction multi-frame/idle-repaint regressions and workspace_probe threshold mode (scripts/README.en.md). Long-list many screenshots cannot establish that scrollbar/row-height feedback is absent at the threshold.

Record programmatic floating-window movement separately from physical mouse dragging. island_move_probe tests native enter/exit movement notifications, programmatic monitor transitions, multi-frame position retention and oversized-client recovery; do not describe it as simulating the Windows mouse modal loop. If the UI-control channel is unavailable, preserve that manual-acceptance boundary instead of claiming every reported symptom is resolved because probes pass.

The preview.4 island_move_probe keeps native movement state active across actual DPI boundaries for both main and floating windows. It records suggested/actual rectangle differences, nested DPI depth, absence of extra size repairs during movement, post-release recovery and stable dimensions on repeated crossings. It still injects no system mouse input and does not establish physical mouse-modal dragging; the older preview.3 enter/exit-before-crossing evidence is not a substitute.

Generate public README captures separately with Release readme_capture and island_probe; commands are in scripts/README.en.md. Reuse production sidebar/terminal/floating components with fixed synthetic text, example directories and simulated status, without reading user profiles or launching Agents/models. Verify clean exit, native PNGs and privacy before copying unchanged and updating assets/readme/SOURCE.json. Public display fixtures do not replace actual Agent acceptance.

Use agents_status_probe for native Codex lifecycle/agents acceptance; commands are in scripts/README.en.md. codex mode sends only Left and Esc, checking the real entry, overview, return and native idle; no prompts, hook/project trust acceptance or shared-server stop. badges mode captures synthetic adjacent mode icons at 100%/146%/200%. Offline regressions separately cover queue bounds, old epochs, hidden ConPTY, unread state and legacy-hook precedence; startup/navigation captures are not real model turns.

Use `sidebar_drag_probe` for session dragging and unread-failure captures (commands in scripts/README.en.md). It uses synthetic metadata and its own egui input to capture expanded/collapsed unread, read, drag and drop states; it starts no PTY, Agent or model and injects no system input. `sidebar_interaction` covers off-center grab points, fixed-slot gaps, multiple zoom levels, upward/downward ordering, Escape/outside cancellation, edge scrolling and idle behavior after cancellation. `settings_floating` verifies that viewed failure and completion badges both clear. Record App startup/process-exit failure regressions separately from real model tasks.

Qoder CN: tests/qoder_cn.rs covers discovery, configuration, resume and synthetic metadata through the real helper. shell_sessions uses a fake CLI to verify both entries and explicit-setting preservation in PowerShell/CMD/Git Bash. qoder_cn_probe fixture mode paints synthetic state only; default mode starts an already-installed CLI without prompts, trust approval or authorization. Record capture completion separately from hook_connected; login/trust screens do not establish a connection or a successful real model task.

Direct Codex startup regression uses the App's real shell/ConPTY and an absolute fake codex.cmd, preserving --remote/-C/resume arguments. Check the unidentified phase, native state, hidden background reduction, temporary clear handoff, prompt return and repeated calls. Append `codex-direct` to `shell_switch_probe` for real absolute-path/--remote unix:// startup captures in an already trusted directory. Do not submit model prompts, approve trust or stop the shared server. Inspect the captures manually; they are not real model-turn evidence.

For the three new entries in 0.4.7, use shell_switch_probe modes kimi/zcode/traecli; commands are in scripts/README.en.md. Only launch commands, an empty-terminal focus click and Esc/Ctrl+C enter owned windows; inspect identity and return to shell without prompts or trust approval. Record whether ZCode is an official or community distribution; local zcode-app-cli is not the official standalone CLI. agent_icons_capture natively renders six dark-surface README badges using production components, without PTYs or Agents; inspect, copy unchanged, and record provenance.
