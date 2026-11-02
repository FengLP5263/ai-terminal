# AI Terminal application icon

[简体中文](README.md) | English

The selected original B “stacked sessions” SVG is byte-identical to design candidate `b.svg` and uses the project's MIT license.

- `ai-terminal.svg`: vector source.
- `ai-terminal.png`: 256×256 RGBA native window icon.
- `ai-terminal.ico`: 16, 20, 24, 32, 40, 48, 64, 96, 128, and 256 pixels, each independently rendered from the vector, for Windows EXE resources.
- `SOURCE.json`: provenance, export method, sizes, and SHA-256 hashes.

The build uses fixed exports without a browser or image tool. `build.rs` invokes Windows SDK `rc.exe`, linking the icon into the app and debug examples, not the hook helper. Root, floating, and visible debug windows share it. The icon is embedded; no external file or installed font is required.

## 0.3.4 verification record

- Governance, fmt, all-target Clippy, 15 relevant Rust checks, and 3 Python checks passed.
- Optimized native smoke in `output/smoke-20261101-155248-137/` exited normally and reaped all three owned PTYs.
- `output/brand-b-20261101-154347/verified.json` records that the Windows resource API returned ten EXE icon frames byte-identical to the fixed ICO; the helper had no GUI icon.
- `WM_GETICON` returned large and small B icons from the new window; screenshots were reviewed. At 150% DPI these were 48×48 and 24×24.
- The `output/preview-v0.3.4-20261101-155258/ai-terminal.exe` preview used separate metadata copied from the prior version without taking over its processes.

Evidence directories are ignored and not part of source or the public package. This historical verification did not itself create a commit, tag, or remote release.
