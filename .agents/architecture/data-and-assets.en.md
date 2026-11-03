# Data, fonts, and artifacts

[简体中文](data-and-assets.md) | English

## Data and recovery

`%LOCALAPPDATA%/ai-terminal/sessions.json` stores session names, directories, Agent types, actual resume IDs, ordering, and UI preferences. 0.3.0 added optional `last_directory`; CMD/Bash and legacy Codex/Claude/Shell records coexist. Since 0.3.2, old Agent records load as PowerShell while preserving other metadata; only the selected shell opens automatically. Prompt metadata updates the working directory; running Agent identity and invocation IDs exist only in memory.

0.3.3 introduced optional `color` (seven fixed enum values); old entries default to no color, and resetting omits it. Color does not change UUID, directory, Agent identity, or process. `ui` stores `sidebar_collapsed`, `sidebar_width`, and `ui_scale` with serde defaults for old data. `terminal_font_size` remains a legacy compatibility field; the UI now scales as a whole from 0.5 to 2.5, with sidebar logical width 190–420. Zoom/width changes save after 400 ms, with immediate save on exit. Writes use a same-directory temporary file, flush/sync, and atomic replacement; malformed JSON must not be silently overwritten.

`runtime/` holds session-level hook configuration; bridge tokens travel only through process environments and short messages. Terminal history is in memory only, limited to 5,000 lines; prompts/transcripts are not persisted, and screen/process restoration is not promised. There is no database or migration framework. Future schemas require compatible defaults and fixture regressions; define backup/migration/rollback before incompatible changes, never clear user records as a shortcut.

## Assets and provenance

- `assets/fonts/Hack-*.ttf`: official v3.003; `SOURCE.json` records source, size, and SHA-256, with `LICENSE.md`.
- `assets/branding`: original B “stacked sessions” icon; SVG matches the selected candidate, PNG/ICO are fixed exports with identities in `SOURCE.json`.
- `assets/icons`: Lobe Icons OpenAI/Claude PNGs, MIT; source URL, date, size and hash in `SOURCE.json`, licenses included.
- Windows Chinese/UI/emoji fonts, including Segoe UI Emoji, are system fonts, not copied into Git or packages.
- `vendor/egui_term` is MIT with revision and patches in `UPSTREAM.md`. Other Cargo dependencies are in `Cargo.lock` and `THIRD_PARTY_NOTICES.md`; packaging records dependencies and collects available licenses.

Font upgrades require all four faces, provenance, hashes, licenses, DPI/baseline tests, and actual screenshots. The package needs the app/helper and project/vendor/font licenses; failed verification is not deliverable.

## Generated and excluded files

Do not commit `target/`, `output/`, `.runtime/`, `runtime/`, `.tools/`, handoff material, databases/dumps, real `.env`, credentials, or archives. Tests use tempfiles or unique `output/` directories, never real user data. Fonts are tracked source assets.

`scripts/package.py` records actual version, Git state, compiler, source and binary SHA-256, archives and verifies by rereading. Clean Git is required unless local acceptance explicitly uses `AllowDirty` and marks the manifest. Packaging does not upload or deploy. Name editing uses Unicode grapheme layout and native transparent emoji tiles, shared bounded atlas and standard egui TextEdit; no system font is packaged.

`design/icon-concepts/` contains four original SVG candidates and an offline selector. B was chosen for 0.3.4; fixed assets in `assets/branding` are embedded at build time. Source identity includes `build.rs` and branding; no external icon file is needed at runtime. The local egui-winit 0.35.0 patch records upstream commit, original archive SHA-256 and MIT/Apache-2.0 licenses; manifest/source are included in artifact identity and licenses gathered through Cargo metadata. Image paste forwards a shortcut and does not persist image data in sessions.

Packaging obtains `target_directory` from Cargo metadata and honors `CARGO_TARGET_DIR` / `build.target-dir`; use a separate ignored target directory when a running preview locks the default release EXE.

0.4.0 adds UI `language` (`system`/`zh_cn`/`en`, default `system`), `floating_enabled` (default false), old optional logical `floating_position`, and physical-pixel `floating_position_pixels` (migrated from the old field). Old data uses defaults; language/float settings use delayed atomic persistence. Floating snapshots contain only ID, name, Agent, color, status, and unread state, never task text. The capsule and gear are drawn in code without new image/font/browser dependencies.

The 0.4.2 local egui_glow patch records upstream provenance and licenses; artifact source identity includes its Cargo manifest, Rust sources, and GLSL shaders.

## Native console assets and package format

`assets/windows/conpty/SOURCE.json` pins the Microsoft NuGet version, URL, package hash, x64 hashes, minimum Windows build, signature check and license origin. Governance allows only the fixed OpenConsole.exe in this asset directory as a third-party exception; other EXEs remain rejected. The license is preserved verbatim.

New manifests use format_version 2 with native_console identity, both companions, Microsoft-ConPTY-LICENSE.txt and Microsoft-ConPTY-SOURCE.json. Missing files fail verification; older release formats remain verifiable. Source identity includes the entire native asset directory. Packaging rechecks pinned hashes before copying and never takes companions from another version's output directory.

Starting with 0.4.3, both root CHANGELOG files are covered by source identity and shipped in the ZIP. Build checks the current version, matching date, and nonempty categorized changes first. GitHub Releases reuse the same version entry. Read-only integrity verification of older ZIPs still accepts archives without the new changelog files.

Agent source icons remain 640×640. Area-filter them at the physical display size, sharing at most 12 cached sizes per avatar and aligning to integer pixels; no per-frame resampling. Native emoji tiles fit complete clusters inside cells, preserve VS15 text presentation, and keep emulator coordinates unchanged.

Provenance and the registry archive hash for the local winit 0.30.13 patch (Apache-2.0) are in vendor/winit/UPSTREAM.md; the original LICENSE is retained. Package source identity covers its Cargo.toml, build.rs, and sources/embedded assets under src. The dependency inventory records the local patch and collects that LICENSE automatically.
