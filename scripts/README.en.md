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

`project.ps1 ci` is vendor-independent; a local run is not a GitHub Actions result. Standard CI does not require a GPU, model account, or proxy. Native GPU smoke and visual acceptance are separate interactive Windows checks. Historical Windows CI, native smoke, idle measurement, package, and Git Bash governance/format results are documented in the [Chinese record](README.md); native macOS/Linux remains unverified.

Since 0.3.7, test/CI also explicitly run `egui-winit`'s `paste_tests`. The `codex-image` and `claude-image` terminal probes insert but do not send an attachment. `lifecycle_probe offline` uses a local fake Responses service, not a real billed model turn.

Packaging respects Cargo's `target_directory` and `CARGO_TARGET_DIR`. If the default binary is in use, build with a separate ignored target directory; do not overwrite a running preview.

For 0.4.0 multi-window acceptance, build `island_probe` offline with `cargo build --offline --locked --example island_probe`, then run `target/debug/examples/island_probe.exe output/<unique-island-probe>`. It uses synthetic states and its own windows, writes PNG/JSON, and exits. Record whether the floating monitor and active status are enabled during performance checks.

For zoom, build `island_zoom_probe` with `cargo build --offline --locked --example island_zoom_probe`, then run `target/debug/examples/island_zoom_probe.exe output/<unique-zoom-probe>`. Verify physical viewport size, screen centering, vertical position, and screenshots at 50%, 71%, 100%, and 150%. The optional physical display X coordinate tests different-DPI monitors; the probe only reads its own windows and uses synthetic sessions.

Artifact source identity also includes the local egui_glow Cargo manifest, Rust sources, and GLSL shaders so the graphics patch is covered.
