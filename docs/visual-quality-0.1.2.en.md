# 0.1.2 Fonts and colors: REA comparison and fixes

[简体中文](visual-quality-0.1.2.md) | English

2026-10-30. The user compared ai-terminal with Warp and reported typography, color and distortion issues. The reverse-engineer-anything skill and actual REA MCP inspected the local Warp artifact read-only. No injection, debugging, settings changes, or conversation reads were performed on Warp.

## REA evidence and limits

Warp installation: `C:/Program Files/Warp`; main binary 390,372,712 bytes, SHA-256 `b50810c5e6f86f6d277ab90744444cdbc3cbda2160f38c1b6063d44195bfa5d2`.

- Directory inventory `ev_a010a77e6442bf8e6364068a8537d0e482baebdc9588021d65a168c98bd9b3cf`: 231 entries, 12 headers, 3,496 bytes; no Electron candidate in that coverage, not proof no embedded code exists anywhere.
- PE imports `ev_c394b41df4c89a41ef1b3a19dd4464b663e6d2d392b56ae1d54e39567afcd29d`: complete table of 41 DLLs/670 imports including OpenGL WGL, GDI and DXGI. Static imports do not prove the runtime backend or font algorithm.
- Retained inventory view `ev_3a3d169a8382d3a4e4a86b47e1369c6b169903f79ec0276de2a080af7d220695` identified `resources/settings_schema.json`. Four Evidence records were exported to `output/rea-warp-visual-20261030.json` (SHA-256 `4a932412bf70b5d1d5c63fe8430daaa2baad2a6e1df27ce3168c3b923559f044`).

The identified schema and user `settings.toml` font/zoom fields showed schema defaults `font_name=Hack`, `font_weight=normal`, `line_height_ratio≈1.2`, `ligatures=false`; current settings had `font_size=12`, window zoom 125. No explicit font_name override was present, so the default was inferred, not directly observed as the runtime face. REA native UI/PTY capture at that version required macOS/Linux, so no unsupported Windows dynamic interface was invoked. Runtime validation used this project's Windows ConPTY and GPU screenshots.

## Fixes

1. The launcher environment had `NO_COLOR=1` and `TERM=dumb`, while Windows user/system NO_COLOR was unset; CLIs suppressed color. The app clears no-color flags in its own process before threading and declares xterm-256color/truecolor, without changing registry environment or inventing ANSI colors.
2. Integer logical widths had caused half-physical-pixel gaps at 150% and per-glyph centering destabilized baseline/position. Round the grid in physical pixels, retain fractional logical coordinates, use consistent ConPTY pixel, drawing and selection metrics, a common baseline and DPI alignment.
3. Bundle the four official Hack v3.003 faces with original license; select real bold/italic faces by terminal attributes. Chinese uses system fallback; UI uses Segoe UI plus Chinese fallback. Normalize common colors and contrast.
4. Share the appearance module between production and probes so test fonts match the app.

See [Hack v3.003](https://github.com/source-foundry/Hack/tree/v3.003/build/ttf), `assets/fonts/SOURCE.json` for identities, and [NO_COLOR](https://no-color.org/).

## Verification and gaps

fmt, all-target Clippy, 36 tests and optimized release build passed. New tests covered 100/125/150/200% physical grids, common baselines, real bold and bold-italic, retaining Chinese composition, special keys, editing and background-status regressions. Real Codex TUI `output/typography-codex-20261030-142305` showed test text and 71 non-default foreground characters; real Claude `output/typography-claude-20261030-142305` showed 384. Both used controlled IME/Text without Enter or model requests; PNGs reviewed. Optimized smoke `output/smoke-20261030-143105` used three real PowerShell sessions plus synthetic events.

Claude reported a fallback from its own fullscreen renderer; only that fallback was verified and global configuration was not altered. Full Claude fullscreen and all physical IME combinations remained unverified. Local preview `output/preview-v0.1.2/ai-terminal.exe` had checksums and font identities in `build.json`. Old memory measurements were not claimed for the new version; full Warp functionality or implementation parity was not claimed.
