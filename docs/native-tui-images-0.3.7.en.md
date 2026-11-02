# 0.3.7 Native TUI, status, and images

[简体中文](native-tui-images-0.3.7.md) | English

## Changes

- Remove forced Codex `--no-alt-screen`, retaining its full-screen welcome logo, fixed bottom input, and native scrolling. The managed entry uses `--no-daemon` to retain the session environment and Job ownership; existing daemons are not stopped.
- Codex may emit SessionStart only after the first task. Before then show “waiting for task notifications”; do not infer state from output or process existence, and do not describe an already Active hook as needing approval.
- The window layer previously emitted Paste only for nonempty text. The pinned local egui-winit 0.35.0 patch retains image/empty-text paste events and records upstream commit, archive hash, and license.
- Only when the terminal is recognized as Codex/Claude, inspect Windows image formats and forward Alt+V, supported natively by both CLIs; users still press Ctrl+V or Shift+Insert. The CLI owns pixels, encoding, and upload. Normal text, shells, and session-name editing retain their paths.

## Verified

- Optimized native smoke passed with three real PowerShell processes, working/waiting/completed indicators, GPU screenshot, and owned PTY cleanup; screenshot reviewed. An isolated ai-terminal 0.3.7 preview preserved existing windows and processes.
- CI passed governance, fmt, all-target Clippy, 89 Rust tests (including the extra window patch test), and 3 Python tests.
- Codex CLI 0.164.0 showed the large logo with ALT_SCREEN, SGR_MOUSE and MOUSE_MOTION. After full startup, paste produced `[Image #1]` without Enter. Claude Code 2.1.316 also showed `[Image #1]` without Enter; both fullscreen and CLI fallback to classic were observed. User renderer configuration was not changed.
- Codex `/hooks` showed eight status hooks Active; the welcome page alone did not emit SessionStart. The real CLI against a loopback fake Responses service produced SessionStart, UserPromptSubmit and Stop using the manager's launcher and helper; normal SessionEnd was separately checked.
- Images were not submitted and the fake service did not call real inference. CLI login checks or startup warmup network activity is separate from a billed model turn.

Evidence is in ignored `output/codex-ready-image-v037`, `output/claude-image-wait`, `output/codex-offline-complete`, and other isolated directories; terminal content is not committed.

## Limits

Checks covered the installed Windows x64 CLIs and current image clipboard. Other screenshot tools, image file lists, remote/WSL clipboards, custom bindings, and future CLI versions require separate acceptance. The user must review hook trust in the CLI; no user-level hook/trust files were rewritten. References: [Codex CLI options](https://learn.chatgpt.com/docs/cli/reference), [Codex lifecycle hooks](https://learn.chatgpt.com/docs/hooks), and [Claude image input shortcut](https://code.claude.com/docs/en/interactive-mode).
