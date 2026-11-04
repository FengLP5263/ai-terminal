# egui_term upstream and local patches

[简体中文](UPSTREAM.zh.md) | English · [Original mixed-language record](UPSTREAM.md)

Source: https://github.com/kemokempo/egui_term

Revision: `31bbc7ab8503c9518fcee5717cfa29011e59f451`

License: MIT; original LICENSE preserved.

Local changes: viewport-only snapshots; 5,000-line scrollback; no background redraw; coalesced output wakeups; receiver-disconnect termination; per-child environment and Windows argument escaping; IME commits; keyboard focus independent of pointer; bracketed paste; live-grid selection copy; hidden-cursor support.

Color palettes and default key bindings are cached/shared. Rendering avoids per-cell mapping rebuilds, color-string clones and hex parsing. Theme OSC color queries are answered for CLI compatibility; IME candidate positioning is supplied. Terminal row height accounts for the CJK fallback face to avoid overlapping Chinese and Latin lines.

Input/display repair: stable widget identity; request focus only when needed (repeated focus cancels native IME); terminal owns Tab/arrows/Escape; suppress raw candidate keys during composition; never interpret literal Text events as key names. Round cell widths up; paint backgrounds before glyphs; retain combining marks and concealed cells; use native cursor shape, wide cursor positions and dynamic colors; clip drawing to the pane. Draw common box/block/quadrant glyphs as cell geometry to avoid seams.

0.1.2 typography: logical cell geometry is quantized to physical DPI pixels; ConPTY receives physical cell pixels; selection uses the same floating-point metrics. Glyphs share a baseline; terminal flags select real bold/italic, and placeholder-color galleys are reused across ANSI colors. The application embeds licensed Hack faces.

## 0.2.0: Clarity and size stability

- Pure black background and neutral gray text; font size, grid and baseline align to physical pixels. Compute line height using mixed glyph ink bounds and 1.2× font size so CJK fallback does not enlarge every line.
- Resize ConPTY only when rows, columns or physical cell size change; minor layout changes update only local layout. Ctrl+wheel was consumed locally by the host to update TerminalFont; normal scroll, IME and PTY input stayed available.

## 0.2.1: Native Windows text and whole-window zoom

- Host consumes Ctrl+wheel/plus/minus/0 in `raw_input_hook` to scale the whole egui window. TerminalFont may use a process-local font set. Windows uses DirectWrite layout and Direct2D/WIC atlas rasterization with system color fonts, without installing fonts.
- Chinese uses Microsoft YaHei UI; Chinese/Latin share baseline and physical alignment, with grayscale anti-aliasing to avoid RGB fringes.
- NativeFont caches up to 64 formats/metrics and eight 1024² atlases per context; the thread keeps at most two contexts. Rasterize on demand without background polling.
- Compose skin tones, ZWJ, and flags as Unicode graphemes while retaining original emulator coordinates. A VS16 heart followed by a space occupies two cells. Native tiles use actual background/cursor colors; insertion cursor draws after text.
- Release `TextRendererSession` in `on_exit` before window/COM teardown; Drop is backup. If Windows native rendering fails or on other platforms, retain epaint fallback, which does not imply color emoji support.

0.3.0: Forward `ai-terminal-cwd;` OSC title events to the host for generation-validated working-directory updates; other titles are not used to infer Agent identity or state.

0.3.2: Expose `TerminalFont::emoji_shape` to host name/edit fields, reusing the engine and bounded atlas with system Segoe UI Emoji for composed sequences. UI tiles retain PBGRA transparency and upload premultiplied colors; terminal opaque backgrounds retain their semantics. No new font-engine cache or background polling.

0.3.3: Normal wheel used `Term::scroll_display`, without generating ESC OA/OB for ALTERNATE_SCROLL+ALT_SCREEN. Real keyboard/pointer and Ctrl zoom remained; the Codex entry also used public `--no-alt-screen` for normal history. This was superseded by 0.3.7.

0.3.6: Route a separate MouseWheel command according to backend mode: standard SGR/traditional mouse wheel when MOUSE_MODE is enabled; local history otherwise. Shift forces local history; Ctrl/Command belongs to host zoom. Pointer position uses screen grid, at most 32 events per input; no synthetic arrows. Preserve smooth-point remainder and bounded history, selecting mouse protocol from current mode.

0.3.7: The host can enable `native_image_paste` for recognized coding CLIs. On Paste query only Windows image formats; forward Alt+V for CLI-owned image attachment. Text keeps bracketed paste/newline behavior. No pixel copy, temporary image, clipboard writes, or background query. Codex restored default fullscreen TUI without forced `--no-alt-screen`.

0.4.0: PTY metadata, exit and redraw events explicitly request ROOT viewport repaint. With a deferred floating status window, waking root logic cannot depend on the viewport currently being painted. Terminal I/O, hidden-output draining, mouse protocol and bounded history remain unchanged.

Managed Codex invocations: a bounded, session-scoped OSC 9 observer forwards only recognized attention-notification metadata; content is discarded. The read/write wrapper preserves the PTY stream and coalesces repeated notices. No ordinary output or bell is used to infer completion.

Input responsiveness: avoid duplicate grid snapshots on frames without input and skip unchanged resize locks. Queue input before waiting for the grid, and coalesce adjacent keyboard bytes within a bounded 64 KiB batch while preserving action order.

During the application exit confirmation, TerminalView can paint its last snapshot without acquiring the active terminal grid lock. Output continues to drain and the normal live view resumes if the user cancels.

IME focus recovery: clear unfinished composition when the native window or terminal loses focus, or focus is explicitly restored. Cancellation received while unfocused must not leave later keyboard input suppressed. Ordinary repaints preserve active composition. A real ConPTY regression covers cancellation during focus loss and typing/submission after returning.

Coding CLI drag selection: the host can enable set_drag_selection to prefer local selection for ordinary left drags under application mouse tracking. A 3-point threshold distinguishes a drag from a click; clicks forward a press/release pair on release. Shift forces local selection and other applications retain 1002/1003 drag reports; wheel routing is independent. Copying a selection sends no Ctrl+C to the PTY, while copying without one preserves interruption. Real ConPTY regressions cover normal/SGR mouse modes, clicks, drags, copying and interruption. Native selection screenshots do not overwrite the system clipboard.

Windows compatibility acceptance: publicly re-export Alacritty WindowSize for host probes to inspect final ConPTY rows/columns and physical cell dimensions. The host bundles pinned Microsoft conpty.dll/OpenConsole.exe beside its executable, using Alacritty's existing dynamic loading path; this patch does not alter its ConPTY protocol implementation.

Unreleased: fit native emoji clusters into cell bounds instead of clipping their fallback line boxes; narrow pause/play badges may use following padding. VS15 keeps text presentation. Emulator coordinates, selection and bounded atlas ownership remain unchanged.

Native Codex lifecycle: recognized OSC 7501 state/app/kind metadata only, with a bounded 32-state ordered queue, coalesced wakeups and invocation-epoch rejection. PTY bytes remain intact; no prompt/activity text is stored. Adjacent narrow mode badges keep equal visual widths; only isolated badges extend into following padding.
