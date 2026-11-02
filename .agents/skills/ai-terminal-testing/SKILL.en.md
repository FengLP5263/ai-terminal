# Testing and acceptance

[简体中文](SKILL.md) | English

```powershell
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 measure-idle -Release
```

See [scripts](../../../scripts/README.en.md) for the POSIX counterpart and platform limits. Preserve the module and `tests/` layout. Standard tests include the Rust workspace and Python package-verification tests, without model API calls. Codex configuration tests may skip when the CLI is absent; record installed-CLI and live-turn checks separately.

Native smoke runs three real PowerShell PTYs with synthetic lifecycle events in a unique isolated `output/` directory. Verify JSON, PNG, exit, and cleanup of the PTYs it owns. On failure, stop only processes started by that test. Distinguish manager from Agent subprocesses, debug from release, GPU, measurement window, and version when reporting performance.

Run TUI probes with unsubmitted text only in a previously trusted directory; never automatically accept CLI trust or authorization. Real model turns use the user's account and can incur charges: they require explicit authorization and `verify-claude -AllowModelRequest`, with a $0.15 limit and tools disabled. They are not default CI.

Run screenshots and idle measurements after optimized builds complete. `terminal_probe` and `workspace_probe` must wait for real CLI or sample content; blank screenshots do not pass. Check native probe exit codes and release `TextRendererSession` in `on_exit`. Text changes cover color emoji, modifiers/ZWJ, caret, and terminal boundaries at multiple zoom levels.

`scroll_probe` uses a local raw-console pager to test application scrolling, normal/SGR mouse, MOUSE_MOTION, Shift local history, and Ctrl zoom; do not synthesize history-arrow keys. A real Claude startup check is not a completed model-task test.

For window/image paste changes also run `cargo test --offline --locked -p egui-winit --lib paste_tests`; this patch is outside the workspace and must be included explicitly. Image probes read the current clipboard only for explicit paste checks and do not press Enter. `lifecycle_probe offline` sends a fixed example only to a temporary local fake Responses service and records fake model turns separately from real hook events; it does not call an actual inference service or accept trust.

For 0.4.0 settings/floating-window work, run `examples/island_probe.rs` with an isolated output directory: Chinese/English screenshots, minimized root, floating visibility/topmost, click-to-restore, and close/reopen. States are synthetic; no Agent/model is started. `tests/settings_floating.rs` covers old settings, persistence, control selection, and clicking after horizontal scrolling. App tests check actual owned PTY PIDs. Probes read only their own window properties and do not inject system input or touch user windows. `island_zoom_probe` covers viewport size, centering, and DPI as described in [scripts](../../../scripts/README.en.md).
