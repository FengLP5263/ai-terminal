# 0.3.0 Terminal-first sessions

[简体中文](shell-sessions-0.3.0.md) | English

New sessions show only PowerShell, CMD, and Bash. The first directory is the user home; later sessions use the active or most recent valid directory. A changed directory is reported at the shell prompt boundary. Existing sessions are retained without automatic startup.

Only entering `codex`/`claude` launches an Agent; the avatar changes on that session's real launch/exit notification and returns to the shell on exit. Merely printing those words does not change the icon. Working/waiting/completed still relies on lifecycle events, never process existence or output silence. Configuration is session-scoped; existing user configuration and trust are unchanged and Claude hook files are reused. Bash means Git Bash; other Agents, WSL, absolute paths, and user aliases were deferred.

## Verification

Real-shell tests passed for directory changes, paths with Chinese and spaces, directory inheritance, and launch/exit notifications for codex/claude in all three shells using a native test CLI without model requests. Regressions covered old data, background status, stale events, icon recovery, and user names. Full checks, native UI, and preview were recorded after completion below.

## Local delivery (2026-11-01)

- Governance, fmt, Clippy, 62 Rust and 3 Python checks passed (65 total), including rendered/clickable new-session menus in expanded/collapsed sidebars and no false detection of `echo codex`.
- `output/shell-codex-v030-20261101-025559`: real PowerShell submitted the launch command, received `ShellAgentStarted:codex`, showed Chinese in real Codex TUI, screenshot reviewed, exited and reaped its root PTY. No Agent Enter or model request.
- `output/smoke-20261101-025827-855`: three real PowerShell sessions and synthetic lifecycle events checked icons, unread/waiting/working, GPU screenshot, exit and own PTY cleanup; synthetic status is not a real Agent turn.
- `output/idle-shell-v030-20261101-025839-442`: empty workspace, 8 frames/8.025 s, UI CPU 12.673 ms; five-second manager CPU 0.296875 s, working set about 105.1 MiB. Not a long-term multi-session performance claim.
- Local ZIP `output/packages/ai-terminal-v0.3.0-windows-x64-20261031T190728Z-b35506.zip`, 316 members, read-back verified, `dirty=true`, unpublished; SHA-256 `572de634f0d5a60c5356c3ef1c3a324607f94df20ddb2323be4bdb1fbd5a0dd4`.
- Both EXEs in `output/preview-v0.3.0` matched the package hashes; isolated metadata was used. Saved old Agents were not auto-started and existing terminals were unchanged. At that time the work was uncommitted/unpublished on `feature/shell-first-sessions`, based on CLI fix `45dc789`.
