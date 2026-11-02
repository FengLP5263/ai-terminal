# 0.2.0 Terminal display and session panel

[简体中文](visual-quality-0.2.0.md) | English

This work addressed the user's comparison of the same Claude session in ai-terminal and Warp and the Ctrl+wheel zoom request.

## Behavior

- Terminal background became RGB(0,0,0), sidebar and selection neutral gray, and default font size 14 instead of 15. Four real Hack faces remained. Font size, origin, grid and baseline aligned to physical pixels with light hinting and linear metrics. Line height used mixed-glyph ink bounds and 1.2× font size to avoid stretching every line for CJK.
- Fixed title-bar height and represented disconnected status as a dot tooltip to avoid changing terminal height on hook connection. ConPTY resize occurs only when rows, columns or cell dimensions change.
- Ctrl+wheel over terminal content zoomed terminal font between 8 and 32; normal wheel scrolled history. The title percentage restored 14; zoom preserved terminal process and IME focus. This version's terminal-only zoom was superseded in 0.2.1.
- Session cards supported drag ordering, hover pencil/right-click rename, title-bar collapse/expand, avatar selection when collapsed, and resizing the sidebar's right edge. Avatars and lifecycle state were separate. Moving Session ownership preserved terminal, PID, Job Object, resume ID and active UUID.
- Optional `ui` in `sessions.json` used defaults for old data and kept malformed-file protection. Frequent font/width changes saved after a delay; exit saved immediately.

## Verification

Mouse regressions distinguished select/edit/drag, cross-card ordering, and collapsed selection. Real ConPTY/IME tests confirmed unchanged PID, uninterrupted composition and persisted preferences across zoom. Typography tested 100/125/150/200% DPI and 8/14/15.5/24/32 font sizes for physical grids and baseline.

Governance, fmt, Clippy `-D warnings`, 42 Rust and 3 Python tests passed. A hidden 12,000-line PTY once timed out at 15 s during parallel optimized compilation; serial rerun passed in 12.84 s without relaxing the deadline or dropping assertions. Native UI acceptance should run separately from heavy compilation.

`examples/workspace_probe.rs` captured expanded, actual Ctrl+wheel zoom, and collapsed GPU views using local PowerShell ANSI; content had to be ready, not a blank startup frame. `examples/terminal_probe.rs` waited for actual Claude/Codex in a trusted directory and injected only unsent IME/Text, without Enter or a model request. Claude fullscreen fallback and all physical IME combinations remained outside acceptance; no global CLI settings were changed. Visual quality still required user acceptance.

## Final native and artifact record (2026-10-30)

- `output/ui-v020-final-20261030-154910`: three layouts and real Ctrl+wheel at 150% DPI, size 14 → 18; expanded/zoomed/collapsed PNGs reviewed with terminal sample visible.
- Claude 2.1.314 `output/claude-v020-final-20261030-154931`: unsent Chinese visible, 402 non-default foreground characters. Codex 0.164.0 `output/codex-v020-final-20261030-155042`: unsent Chinese visible, 2 non-default foreground characters. PNGs reviewed, no Enter/model requests, test PTYs exited.
- Main smoke `output/smoke-20261030-154955-043`: three real PowerShell, synthetic events, PNG, exit and owned PTY cleanup passed.
- Idle recheck `output/idle-v020-check-20261030-155307-827`: 8 UI callbacks/8.026 s, UI CPU 6.654 ms, five-second manager CPU 0.140625 s, working set about 104.6 MiB. Earlier `output/idle-v020-20261030-155137-824` recorded 227 callbacks/8 s; the independent rerun did not reproduce persistent activity. Both samples were kept; neither short window is a long-term guarantee.
- ZIP `output/packages/ai-terminal-v0.2.0-windows-x64-20261030T075115Z-af1bea.zip`: 4,897,133 bytes, 297 members, verified by rereading; SHA-256 `255e75313cfa3428ac068dedccf79253e430c79fb07c6d9b6aad4711a24bece3`. Manifest recorded `dirty=true`, unpublished, icon licenses and source identity.
- Preview `output/preview-v0.2.0/ai-terminal.exe`, title `ai-terminal 0.2.0`, used isolated `output/preview-state-v020-20261030-155427/` copied from prior metadata without launching saved Agents, ending the old window, or competing for its configuration. Both EXE hashes matched the verified package. At that time `feature/terminal-workspace-ui` had local changes only: no commit, remote, tag, or release.
