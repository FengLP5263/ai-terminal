# 0.1.1 Input and display fixes

[简体中文](input-display-0.1.1.md) | English

2026-10-30. Users reported 0.1.0 input and display were unusable. This round reproduced and fixed problems without adding product features.

## Reproduced problems

Calling `request_focus` every frame caused egui to emit `should_interrupt_composition`, interrupting IME composition; the new regression failed on the old code. The old renderer omitted combining marks; that regression also failed first. Terminal focus locking, integer-aligned character grids, actual cursor shapes, and line/block drawing were missing.

## Changes

- Use a stable terminal control ID and request focus only on activation, click, or needed recovery; repaint no longer interrupts composition.
- Route Tab, arrows, and Esc to the terminal; stop terminal input while menus/dialogs are open. Suppress raw candidate keystrokes during Chinese IME selection; handle commit, cancel, and later editing separately.
- Ctrl+C copies a selection; use Ctrl+Shift+N for a new session, retaining Ctrl+N for the terminal.
- Round grid width up and account for wide Chinese glyphs; draw background before characters and preserve combining marks, concealment, and dynamic colors.
- Use Alacritty cursor position/shape and draw common line, full-block, and quadrant glyphs on grid geometry to eliminate seams.

## Verification

fmt, all-target Clippy with warnings denied, optimized build, and **34 tests** passed. Added six real ConPTY/egui interaction tests, one application session-switch regression, and one line/block regression. They cover repaint-safe Chinese composition, single commit, editing after cancellation, Tab/arrows/Esc, backspace, combining marks, concealed glyphs, and beam cursor.

Codex 0.164.0 and Claude Code 2.1.314 actually displayed “中文输入 abc123” using controlled egui IME/Text events and real CLIs, without Enter or model requests. A new untrusted project was blocked by update/trust prompts, so verification moved to the user's already-trusted directory; no new trust was accepted. The probe disabled update checks only for its own Codex process, not user configuration. The final Claude probe timed out twice; these were not counted as passes. After adding a background schedule/exit timer independent of painting, it produced visible input, a screenshot and JSON, and exited normally. The production status logic already used a background callback.

Evidence: Codex `output/tui-final-codex-20261030-134735/result.json` and `terminal.png`; Claude `output/tui-diagnose-claude/result.json` and `terminal.png`; release smoke `output/smoke-20261030-135236/smoke.json` and `smoke.png`; preview `output/preview-v0.1.1/ai-terminal.exe` with `build.json`. The native screenshot channel was unavailable, so images came from the app's own GPU capture. Controlled IME events do not establish compatibility with every physical/system/third-party IME. `docs/verification.md` remains a 0.1.0 historical measurement, not a 0.1.1 performance result.
