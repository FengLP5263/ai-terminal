# 0.3.1 UI polish

[简体中文](ui-polish-0.3.1.md) | English

This local build followed consistent dimensions, low visual decoration, and accessible controls while retaining a compact desktop terminal layout.

## Changes

- `src/chrome.rs` centralizes icon/control dimensions: operation icon 16, target 32; avatar container 32 and mark 20 logical pixels, all scaled globally. New, collapse/expand, and edit use 1.5-width vector icons with accessible names, tooltips, hover/press and keyboard-focus feedback.
- Center Codex, Claude, and three shell avatars; shell icons use a shared vector canvas. Brand PNGs, fonts, and provenance hashes stay unchanged. Align list titles, secondary text (12), status markers, selected/hover/status backgrounds, and reserve edit space so long names do not cover controls.
- Align toolbar height and zoom button. Terminal content retains pure black, Hack, and native Windows text. New-session menus, directory inheritance, Agent identification/lifecycle, persistence, and zoom retain existing behavior.

## Verification

Governance, formatting, all-target Clippy, and 11 relevant regressions passed (6 main app, 3 sidebar, 2 typography). All six actual GPU screenshots were reviewed: `output/ui-polish-v031-1160-20261101-133031/` at 1160×760 logical and `output/ui-polish-v031-760-20261101-133031/` at 760×760 logical, each at 100%, about 146% via Ctrl+wheel, and collapsed. Windows DPI was 150%, giving 1740×1140 and 1140×1140 physical pixels. Codex, Claude, PowerShell, CMD, and Bash avatars were checked together; icons remained centered and the terminal filled remaining space. `output/smoke-20261101-133049-798/` passed optimized main app, three real PowerShell PTYs and synthetic lifecycle checks; `smoke.png` was reviewed. Probes and smoke exited and reaped owned PTYs. Samples used local ANSI and synthetic status, without model requests; no new real Agent turn or physical IME acceptance was included.
