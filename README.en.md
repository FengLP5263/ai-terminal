# ai-terminal

[简体中文](README.md) | English

A Windows-native terminal for managing multiple AI coding sessions. Keep Codex, Claude Code, and ordinary shells in one window, switch between them, and monitor task status.

**Version: 0.4.0 · Windows x64 · MIT** · [Download releases](https://github.com/FengLP5263/ai-terminal/releases) · [Documentation](docs/README.en.md) · [Contribute](CONTRIBUTING.en.md)

## Features

- **Terminal sessions:** Create a PowerShell, CMD, or Git Bash session and run an installed CLI with `codex` or `claude`. The corresponding avatar is detected automatically.
- **Task status:** Supported lifecycle notifications and process exits indicate working, waiting, completed, or failed. Silence is not interpreted as completion. A green dot means a turn ended; it is not project acceptance.
- **Session management:** Reorder, rename, color, resize, or collapse the sidebar. Zoom the whole UI with Ctrl+wheel or Ctrl+plus/minus; Ctrl+0 resets it.
- **Settings and floating monitor:** Follow the system language or select Simplified Chinese or English. An optional always-on-top avatar bar shows up to 10 sessions at once; drag the avatar area for more. Click an avatar to return to its existing session. The bar follows UI zoom and recenters horizontally when zoom or session count changes.
- **Native rendering:** Rust, egui, Alacritty, and Windows ConPTY provide Chinese text, ANSI colors, and system color emoji without a browser runtime. Codex keeps its native TUI, including full-screen scrolling and the bottom input area.

## Install and use

Download the Windows x64 ZIP from [GitHub Releases](https://github.com/FengLP5263/ai-terminal/releases), extract it completely, and run `ai-terminal.exe`. Keep `ai-terminal-hook.exe` in the same directory. The release does not require Rust, Python, or Electron. Install and configure Codex CLI or Claude Code separately; the app uses each CLI's existing login, model, trust, and permissions.

1. Click **+** at the upper left to choose a shell, or use Ctrl+Shift+N for PowerShell. New sessions use the current or most recent valid directory; use `cd` in the shell to change it.
2. Run `codex` or `claude`. Switching an active session does not restart its terminal. Restarting the app restores the session list, not running processes or screen contents; use the CLI's own resume command to resume a conversation.
3. Use the upper-left gear to select a language and enable the floating monitor. Hover over an avatar for details, drag the bar's edge to move it, and disable it in settings.
4. Task status uses supported CLI hooks. Codex normally sends its first notification after the first task begins. If status is still absent after submission, review `/hooks` in the CLI. The app never trusts hooks on your behalf or alters user-level CLI configuration.
5. The wheel scrolls terminal history; a full-screen application handles it when it enables the mouse protocol. Shift+wheel uses local history. In recognized Codex or Claude sessions, Ctrl+V or Shift+Insert passes clipboard images to the CLI's native attachment handling.

If an installed CLI is not found, ensure its public `.exe`, `.cmd`, or `.bat` launcher is on the current or Windows user/system PATH. Commands available only through WSL or PowerShell aliases are not supported Windows launchers.

Session data defaults to `%LOCALAPPDATA%/ai-terminal` and contains only session metadata, UI preferences, and temporary hook configuration, not prompts, transcripts, or API keys. Closing the app reaps its own session processes; save active work first.

## Develop from source

Requires Windows x64, Rust stable MSVC, Visual Studio C++ Build Tools, the Windows SDK (including `rc.exe`), and Python 3.11+. See [development scripts](scripts/README.en.md) for verified toolchains and platform limits.

```powershell
git clone https://github.com/FengLP5263/ai-terminal.git
cd ai-terminal
rustup component add rustfmt clippy
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`ci` runs governance, formatting, Clippy, Rust, and Python checks without model API calls. Native macOS/Linux GUI builds are not supported yet. See the [documentation index](docs/README.en.md) for commands, packaging, and verification.

## Contributing and license

Issues and contributions covering input, rendering, status, performance, and documentation are welcome. Read the [contribution guide](CONTRIBUTING.en.md) and [collaboration rules](AGENTS.en.md); omit credentials and private conversations from reports.

Project code is under the [MIT License](LICENSE). Third-party components retain their own licenses; see [third-party notices](THIRD_PARTY_NOTICES.en.md).
