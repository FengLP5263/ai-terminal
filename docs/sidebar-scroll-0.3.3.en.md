# 0.3.3 Session colors, sidebar details, and wheel scrolling

[简体中文](sidebar-scroll-0.3.3.md) | English

## Behavior

- The sidebar title became AI终端; title, plus, and expand/collapse shared a 32-logical-pixel height and horizontal center. Neutral separators are one physical pixel even after zoom.
- Right-click → session color offers seven colors or default. Color persists by UUID; the active row has a brighter background/text and collapsed avatars retain the preference. Changing color does not select the session or restart its PTY.
- Normal wheel scrolls local display, no longer sending history-arrow keys on the alternate screen. At this version the managed Codex entry used public `--no-alt-screen` to retain history without changing global configuration; existing Codex processes required a restart. This behavior was superseded in 0.3.7.
- `design/icon-concepts/index.html` provided four original SVG candidates for selection/feedback; the application icon had not yet been replaced.

## Verification

Governance, fmt, all-target Clippy, 72 Rust and 3 Python tests passed. Regressions covered missing old color defaults, save/reset, PTY preservation, real context menus in wide/narrow sidebars, and header/control alignment at multiple zoom levels. A real ConPTY verified up/down output viewing and pixel/line scrolling without injected arrows on the alternate screen. Installed Codex accepted `--no-alt-screen` and session hook arguments in isolated `CODEX_HOME` via `features list`, with no model turn.

Reviewed wide/narrow debug screenshots at `output/sidebar-v033-1160-20261101-150849/` and `output/sidebar-v033-760-20261101-150849/` for colors, separators and alignment at 100%, about 146%, and collapsed. Both probes exited and reaped PTYs. `output/icon-check-20261101-151632/` recorded dark/light/narrow screenshots; an isolated headless browser checked selection, SVG links, eight size previews, and theme changes, with three screenshots reviewed. Optimized main smoke `output/smoke-20261101-152102-865/` passed; main UI was reviewed and three owned PTYs were reaped.

The preview `output/preview-v0.3.3-20261101-152215/ai-terminal.exe` used separate `output/preview-state-v033-20261101-152215/` metadata copied from the previous preview and did not take over its processes. The icon selector was also opened in the browser; it had no network dependency and delivered candidates only.
