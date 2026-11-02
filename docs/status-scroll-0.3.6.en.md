# 0.3.6 Status connection and Claude scrolling

[简体中文](status-scroll-0.3.6.md) | English

## Behavior

- Clicking top status explains the Agent connection and can copy `/hooks` for user review in the CLI; there is no automatic input, trust, global configuration write, or model proxy. The stable 0.3.5 Codex hook command did not change.
- Exclude inherited old session launcher paths to avoid identifying an old helper as the real CLI. Child Agent events no longer affect the root; root `--agent` still reports normally.
- The old wheel always scrolled local history, which did not work in Claude's application-managed alternate screen. Route by the application's mouse protocol, supporting SGR and traditional encodings; never synthesize history-arrow input. Ordinary terminals scroll local history, Shift forces it, and Ctrl/Command remains host zoom. High-resolution point scrolling accumulates remainders, capped at 32 steps per event.

## Verification

Regressions covered real ConPTY up/down wheel bytes, alternate-screen application paging, Shift local history, Ctrl without application input, normal history, and no arrow injection without mouse mode. Status guidance was tested with real click/copy events; the helper filters child sessions, and a separate process tested discovery with an inherited old launcher.

Native `scroll_probe` used a local raw-console pager and captured older messages, return to bottom, and the status explanation. A real Claude was started only in an already-trusted directory to record lifecycle and terminal modes, without a model request; this is distinct from a real model turn. References: [Claude terminal configuration](https://code.claude.com/docs/en/terminal-config), [Claude hook fields](https://code.claude.com/docs/en/hooks).

## Final local results

84 Rust and 3 Python regressions, governance, fmt, and all-target Clippy passed. Optimized native normal/MOUSE_MOTION paging, connection guidance, and three-state smoke passed with reviewed screenshots and owned PTY/process cleanup. Real Claude Code 2.1.316 fullscreen emitted SessionStart with ALT_SCREEN/MOUSE_MOTION/SGR_MOUSE; no model request or renderer preference change. Long conversations and real model tasks still required user acceptance.
