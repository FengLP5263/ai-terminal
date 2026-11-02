# 0.3.5 Session status fix

[简体中文](status-reporting-0.3.5.md) | English

## Problem and behavior

- `/resume` changed the main conversation but the old reducer accepted ID changes only for clear, discarding later working/completed events. Resume/clear/fork now explicitly change the main session while filtering old-session and old-turn events.
- Stop enters Completed independently of unread status. A read active session keeps its green dot and “completed” until the next submission; tab selection does not alter execution state.
- The old absolute Codex hook path required fresh review after relocation. The stable command reads per-process `AI_TERMINAL_HELPER`, still respecting CLI user review; upgrades from the old version require one review. A disconnected command is shown in the title bar. Windows hooks use a finite five-second timeout for shell startup.

## Verification types

The `/resume` regression first failed old code, then verified Working/Completed and rejected old Stop. A native helper reported through an actually generated PowerShell hook across installations and paths with Chinese, spaces, quotes, `&` and `%`; unread and completed were tested separately. Main app tests covered in-shell session recovery and background completion without painting; sidebar tests retained a green dot for the read active session when expanded/collapsed. Actual Claude initialization and TUI startup in a trusted directory emitted SessionStart; actual Codex `/hooks` showed eight old definitions requiring review. Probes did not accept trust, submit prompts, or call models.

Real turns and first approval require user action; offline regressions, native synthetic smoke, and actual CLI startup are not interchangeable evidence of full integration.

## Read-only startup probe

`cargo run --example lifecycle_probe -- codex output/startup-check` (or `claude`) launches a real CLI through a real shell for up to 30 s. Codex only opens `/hooks`, without selecting trust. It writes isolated evidence and sends no model request. `AI_TERMINAL_PROBE_CWD` and `AI_TERMINAL_PROBE_HELPER` can select a previously trusted directory and the current helper; no trust setting is changed.

## Final local results

78 Rust and 3 Python regressions, governance, fmt and all-target Clippy passed. After isolated optimized release build, native smoke produced Completed/Waiting/Working in three sessions, GPU screenshot review passed, and zoom/collapsed-sidebar probes exited and cleaned up owned PTYs. No real model turn ran; the new Codex status command still required the user's first review.
