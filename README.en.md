<div align="center">

<img src="assets/branding/ai-terminal.png" width="96" alt="AI Terminal icon">

# AI Terminal

**A calmer workspace for your AI sessions.**

Use Codex and Claude Code in a familiar terminal, keep an eye on their status, and return to any session in a click.

[![GitHub Release](https://img.shields.io/github/v/release/FengLP5263/ai-terminal?label=release)](https://github.com/FengLP5263/ai-terminal/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D4)](#platform-support)
[![License](https://img.shields.io/badge/license-MIT-5c8a66)](LICENSE)

[Get started](#quick-start) · [Features](#built-for-multiple-sessions) · [FAQ](#frequently-asked-questions) · [Contribute](CONTRIBUTING.en.md)

[简体中文](README.md) | English

</div>

## Why AI Terminal?

AI tasks can run for a long time. When several windows are open, repeatedly checking which session is working or needs attention can interrupt your focus. AI Terminal brings terminal sessions and their status together so you can keep using your familiar Shell and Agent, and return when needed.

It is a native Windows terminal workspace. It does not bundle a model service or require you to change your CLI login, permissions, or workflow.

## A look inside

![AI Terminal workspace with several sessions in the sidebar and a terminal on the right](assets/readme/workspace.png)

Enable the floating bar to see session avatars and status above other windows. Click an avatar to return to that session:

![AI Terminal floating session bar](assets/readme/floating.png)

> Screenshots show a development build with synthetic session content and simulated status; they contain no real conversations. See the release notes for the features and appearance of a particular release.

## Built for multiple sessions

| Feature | What you can do |
| --- | --- |
| **A real terminal** | Open PowerShell, CMD, or Git Bash and run your installed command-line tools directly. |
| **Agent recognition** | Starting Codex or Claude Code displays the corresponding avatar; other commands still work as ordinary terminal sessions. |
| **Task status** | See working, needs attention, completed, or failed states for supported Agents in the sidebar and floating bar. Status comes from supported lifecycle notifications and process exits; it is not a universal inference for every CLI. |
| **Always-on-top bar** | Show only session avatars and status, up to 10 at a time. Drag the avatar area to see more, and click an avatar to return to its session. |
| **Your workspace** | Drag to reorder, rename or color sessions, collapse the sidebar, and scale the entire interface with Ctrl + mouse wheel. |
| **Familiar terminal behavior** | Use Chinese text, ANSI colors, and system color emoji; pass clipboard images to Codex or Claude Code for the CLI to handle. |
| **Chinese and English** | Follow the system language or choose Simplified Chinese or English in settings. |

## Platform support

| Platform | Status |
| --- | --- |
| Windows x64 | Supported |
| macOS / Linux | Native GUI not currently supported |

## Quick start

### 1. Download and run

Download the Windows x64 ZIP from [GitHub Releases](https://github.com/FengLP5263/ai-terminal/releases/latest), **extract the entire archive**, and run `ai-terminal.exe`. Keep `ai-terminal-hook.exe` in the same directory. Rust, Python, and Electron are not required to run the release package.

### 2. Open a terminal and start an Agent

Select a Shell with the **+** button at the top left, or press <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>N</kbd> for a new PowerShell session. Change to your project directory and run an installed CLI (replace the example path with your own):

```powershell
cd 'C:\path\to\your-project'
codex
# or run claude
```

Install and configure [Codex CLI](https://github.com/openai/codex) or [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) separately. AI Terminal respects the CLI's own login, model, trust, and permission settings.

### 3. Make it yours

Use the gear at the top left to set the language and floating bar. Hover over a session avatar for details and click it to switch sessions. Drag the floating bar to reposition it, and turn it off in settings. Hold <kbd>Ctrl</kbd> and scroll to scale the entire interface.

## Frequently asked questions

### Why is an installed CLI not found?

Make sure its Windows `.exe`, `.cmd`, or `.bat` launcher is available on the current, user, or system `PATH`. A program installed only in WSL or exposed only as a PowerShell alias is not a supported Windows entry point.

### Why is there no Agent status?

Status depends on lifecycle notifications supported by the CLI. Codex usually sends its first notification after a task is submitted; if status is still missing, inspect `/hooks` in the CLI. No status does not mean a task has finished. AI Terminal does not trust hooks on your behalf or change user-level CLI configuration.

### Can I continue a conversation after restarting?

The app restores the session list and interface settings, not running processes or terminal screen contents. To continue an AI conversation, use the CLI's own session recovery feature. Exiting the app ends the session processes it manages, so save active work first.

### How do I scroll terminal history or paste an image?

Use the mouse wheel for history. When a full-screen app takes over the wheel, use <kbd>Shift</kbd> + wheel for local history. When Codex or Claude Code is recognized, <kbd>Ctrl</kbd> + <kbd>V</kbd> or <kbd>Shift</kbd> + <kbd>Insert</kbd> passes a clipboard image to the CLI.

### Are conversations or API keys saved?

The default data directory is `%LOCALAPPDATA%/ai-terminal`. It stores session metadata, interface preferences, and temporary hook configuration, not prompts, conversation text, or API keys. Each CLI manages its own data and configuration.

## Develop from source

You will need Windows x64, Rust stable MSVC, Visual Studio C++ Build Tools, the Windows SDK with `rc.exe`, and Python 3.11+.

```powershell
git clone https://github.com/FengLP5263/ai-terminal.git
cd ai-terminal
rustup component add rustfmt clippy
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`ci` runs governance, formatting, Clippy, Rust, and Python checks without calling model APIs. See the [documentation](docs/README.en.md) and [development scripts](scripts/README.en.md) for development, testing, and packaging details.

## Contribute

Issues, ideas, terminal improvements, and documentation contributions are welcome! Please read the [contributing guide](CONTRIBUTING.en.md) and [collaboration guidelines](AGENTS.en.md) first. Do not include credentials or private conversations in reports.

[Open an issue](https://github.com/FengLP5263/ai-terminal/issues) · [Contributing guide](CONTRIBUTING.en.md) · [Documentation](docs/README.en.md)

## License

Project code is available under the [MIT License](LICENSE). Third-party components retain their own licenses; see [third-party notices](THIRD_PARTY_NOTICES.en.md).
