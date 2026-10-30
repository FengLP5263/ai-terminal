Source: https://github.com/kemokempo/egui_term
Revision: 31bbc7ab8503c9518fcee5717cfa29011e59f451
License: MIT (LICENSE preserved).

Local changes: viewport-only snapshots; 5000-line scrollback; no background redraw;
coalesced output wakeups; receiver-disconnect termination; per-child env and Windows
argument escaping; IME commit support; keyboard focus independent of pointer;
bracketed paste; selection copy from the live grid; respect hidden cursor.

Color palettes and default key bindings are cached and shared. Rendering does not
rebuild mappings, clone color strings, or parse hex colors per cell. Theme OSC color
queries are answered for CLI compatibility. IME candidate positioning is supplied.

Terminal row height accounts for the CJK fallback face, preventing mixed Chinese
and Latin output from overlapping adjacent rows.

Input/display repair: stable widget identity; focus is requested only when needed
(repeated requests cancel native IME); terminal owns Tab/arrows/Escape; composition
suppresses raw candidate keys; literal Text events are never parsed as key names.
Cell widths round up rather than truncate. Paint backgrounds before glyphs; retain
combining marks and concealed cells; use native cursor shape, wide cursor position,
and dynamic terminal colors; clip all drawing to the terminal pane.

Draw common box rules and block/quadrant glyphs as cell geometry to prevent seams.

0.1.2 typography: cell geometry is in logical points quantized on the physical
DPI grid, ConPTY receives physical cell pixels, selection uses the same float
metrics. Glyphs share a baseline instead of centering each character vertically;
real bold/italic faces are selected by terminal flags, with placeholder-color
galleys reused across ANSI colors. Application embeds licensed Hack faces.
