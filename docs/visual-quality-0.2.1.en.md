# 0.2.1 Whole-window zoom and native Windows text

[简体中文](visual-quality-0.2.1.md) | English

User acceptance found 0.2.0 scaled only terminal text, not the sidebar, while fonts, color and emoji remained unsatisfactory. The earlier result was not treated as this round's visual acceptance; actual GPU screenshots used the same content.

## Behavior and implementation

Ctrl+wheel is consumed in `raw_input_hook` anywhere in the window and changes egui's whole-window scale; Ctrl+plus/minus also work, and Ctrl+0/title percentage restores 100%. Sidebar, controls, title and content scale together from 50%–250%, persisted as `ui.ui_scale`. Legacy `terminal_font_size` remains for compatibility but no longer separately changes content. The terminal fills the area right of the sidebar with a 12-logical-pixel margin and recalculates ConPTY rows/columns from available space and actual glyph metrics.

Windows text uses DirectWrite layout and Direct2D/WIC rasterization with system color fonts. The four Hack faces load in a process-local font set without installation; Chinese uses Microsoft YaHei UI and emoji system fallback. Grayscale anti-aliasing avoids cross-monitor RGB fringes; size, position and cells align in physical pixels. Original ANSI/truecolor values are retained, transparency is composited against the actual background, and color emoji are not tinted as foreground text.

Unicode grapheme composition supports skin tones and ZWJ while retaining original Alacritty grid coordinates; it does not claim to change emulator width algorithms. A VS16 heart followed by a space gets a full two-cell image. Missing emoji depend on installed fonts; fallback is not a full Unicode-conformance guarantee. Native tiles match actual cursor background; insertion/hollow cursors are drawn last so opaque glyph tiles cannot cover them.

At most eight 1024² atlases and 64 format/metric entries are cached per context, with at most two contexts per thread. Glyphs rasterize only on cache miss; no new background polling. The independent hook helper initializes no native text/GPU. `TextRendererSession` explicitly releases in `on_exit` before window/COM teardown, with Drop as backup.

## Verification

- Governance, fmt, Clippy `-D warnings`, 46 Rust and 3 Python tests passed.
- At 75/100/150/200% whole-window zoom, sidebar physical size followed scaling, terminal met sidebar/right/bottom edges, and PID stayed unchanged.
- Real ConPTY regressions covered skin tones, VS16, ZWJ/regional sequences, color pixels, bold, cursor occlusion, IME, and persisted preferences.
- `output/ui-v021-clean-20261030-164113` debug PNGs were reviewed: scale 100% → 146.41%, all controls followed, exit code 0, test PTY reaped. An earlier native probe crashed with `0xc000000d` after capture and was not counted as passing; `on_exit` resource cleanup was added rather than relying on Windows TLS destructor order.

See [Microsoft Color font support](https://learn.microsoft.com/en-us/windows/win32/directwrite/color-fonts). This does not reuse Warp private code or claim parity with all its fonts/protocols.

## Final optimized delivery (2026-10-30)

- `output/ui-v021-settled-20261030-170142`: expanded, 146.41% zoomed and collapsed GPU views reviewed, exit 0 and PTY reaped. An earlier `output/ui-v021-final-20261030-165218/zoom.png` was a ConPTY reflow transition frame and not accepted. Probe now waits at least three frames and one second after layout changes and for ready content; production paint semantics were not changed for screenshots.
- Real Claude `output/claude-v021-final-20261030-165218` had visible Chinese and 402 non-default foreground characters; Codex `output/codex-v021-final-20261030-165218` had 48. PNGs reviewed; no Enter/model request; exit 0 and PTYs reaped.
- `output/smoke-20261030-165245-513` passed three real PowerShell, synthetic lifecycle, screenshot and normal exit. `output/idle-v021-20261030-165305-784`: 11 UI callbacks/8.031 s, UI CPU 6.326 ms, five-second manager CPU 0.125 s, working set about 105.1 MiB; empty-session measurement, not heavy-load evidence.
- ZIP `output/packages/ai-terminal-v0.2.1-windows-x64-20261030T090002Z-bcf389.zip`: 4,969,021 bytes, 317 members, read-back verified; SHA-256 `56307381ee9eef0bbd6fe485ce54f7e89c674a45455d5ff44d345aa43d52e1c4`, source SHA-256 `b99244863f01849370cf04ebadadddf3f7b41f25fd969cb97ef29a4925abf8fb`, `dirty=true`, unpublished.
- `output/preview-v0.2.1/ai-terminal.exe` (PID 51748, title `ai-terminal 0.2.1`) used separate `output/preview-state-v021-20261030-170417` metadata and retained old windows/tasks. Both EXE hashes matched the package; helper was 197,120 bytes and initialized neither fonts nor GPU. These are pre-commit local acceptance records from `feature/terminal-workspace-ui`; `dirty=true` describes package build time, not later tag/commit state.
