# 0.3.2 Direct shell entry and emoji names

[简体中文](shell-entry-emoji-0.3.2.md) | English

- A new workspace opens PowerShell directly; restoring opens the previously selected shell, with other tabs opening on click rather than in bulk. Remove “start/continue session” from the body and context menu. Switching running terminals preserves PTY, process and input; errors show actual failure rather than retrying each frame.
- Legacy Codex/Claude tabs become PowerShell while preserving names, directories and history IDs, without automatically running an Agent. Icon identity comes from launch notifications, separate from lifecycle activity.
- Sidebar names, titles and rename fields use Unicode grapheme layout for system color emoji, skin tone and ZWJ sequences. Standard TextEdit retains selection, caret, copy and paste; stored text remains original Unicode. UI tiles share the bounded native atlas with terminal glyphs and preserve transparency. System emoji fonts are not packaged; unknown emoji depend on installed Windows fonts.

## Verification

Governance, formatting, all-target Clippy, 67 Rust and 3 Python tests passed. Coverage included direct opening in real PowerShell, legacy records, click/open and PTY preservation, selecting/copying/pasting names, and transparent color emoji rasterization. `output/emoji-v032-debug-1160-20261101-142956/` and `output/emoji-v032-debug-760-20261101-142956/` contain reviewed wide/narrow screenshots, name fields, and select-all at about 146%; both probes exited and reaped their PTYs. Native tiles had transparent backgrounds and color pixels; input tests kept skin-tone and ZWJ clusters together without restoring monochrome outlines on selection.

The existing 12,000-line CMD output test once exceeded its 15 s budget. Output kept progressing and drained in about 17.6 s, so the functional wait became 30 s; it finished in about 4.7 s in the final full regression. Production I/O was unchanged and this is not a throughput promise.

Optimized smoke `output/smoke-20261101-143533-862/` passed; `smoke.png` showed color emoji in sidebar and title, with normal exit and all three owned PTYs reaped. Local optimized preview `output/preview-v0.3.2-20261101-143738/ai-terminal.exe` used isolated `output/preview-state-v032-20261101-143738/` metadata copied from the previous preview without taking over processes. It opened a PowerShell root and did not auto-run saved Codex/Claude; the window title was `ai-terminal 0.3.2`. No model request was made. All physical IME combinations and emoji absent from system fonts remain outside acceptance.
