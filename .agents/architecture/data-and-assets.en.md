# Data, fonts, and artifacts

[简体中文](data-and-assets.md) | English

## Data and recovery

`%LOCALAPPDATA%/ai-terminal/sessions.json` stores session names, directories, Agent types, actual resume IDs, ordering, and UI preferences. 0.3.0 added optional `last_directory`; CMD/Bash and legacy Codex/Claude/Shell records coexist. Since 0.3.2, old Agent records load as PowerShell while preserving other metadata; only the selected shell opens automatically. Prompt metadata updates the working directory; running Agent identity and invocation IDs exist only in memory.

0.3.3 introduced optional `color` (seven fixed enum values); old entries default to no color, and resetting omits it. Color does not change UUID, directory, Agent identity, or process. `ui` stores `sidebar_collapsed`, `sidebar_width`, and `ui_scale` with serde defaults for old data. `terminal_font_size` remains a legacy compatibility field; the UI now scales as a whole from 0.5 to 2.5, with sidebar logical width 190–420. Zoom/width changes save after 400 ms, with immediate save on exit. Writes use a same-directory temporary file, flush/sync, and atomic replacement; malformed JSON must not be silently overwritten.

`runtime/` holds session-level hook configuration; bridge tokens travel only through process environments and short messages. Terminal history is in memory only, limited to 5,000 lines; prompts/transcripts are not persisted, and screen/process restoration is not promised. There is no database or migration framework. Future schemas require compatible defaults and fixture regressions; define backup/migration/rollback before incompatible changes, never clear user records as a shortcut.

## Assets and provenance

- `assets/fonts/CascadiaMono-*.ttf`: optional Latin/digit family from official Microsoft v2407.24, four original static styles under SIL OFL 1.1. Archive/revision/file hashes and the original license are in `SOURCE.json` and `LICENSE-Cascadia.txt`.
- `assets/fonts/Hack-*.ttf`: official v3.003, the optional 0.4.8 family; `SOURCE.json` records source, size, and SHA-256, with `LICENSE.md`.
- `assets/fonts/JetBrainsMono-*.ttf`: four original static styles from official v2.304, the default Latin family. `SOURCE.json` pins the commit, archive and file SHA-256; `LICENSE-JetBrainsMono.txt` preserves SIL OFL 1.1.
- `assets/branding`: original B “stacked sessions” icon; SVG matches the selected candidate, PNG/ICO are fixed exports with identities in `SOURCE.json`.
- `assets/icons`: OpenAI/Claude/Qoder come from Lobe Icons (MIT); Kimi Code/ZCode use their official repositories' product icons, and TRAE uses its official CLI documentation site icon. `SOURCE.json` records provenance, license declarations, original dimensions, transforms, size and SHA-256 per file; licenses do not apply across sources.
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

Agent icons retain their upstream PNG dimensions; only TRAE has outer padding cropped and is reduced offline to 640×640. Area-filter them at the physical display size, sharing at most 12 cached sizes per avatar and aligning to integer pixels; no per-frame resampling. Native emoji tiles fit complete clusters inside cells, preserve VS15 text presentation, and keep emulator coordinates unchanged.

Provenance and the registry archive hash for the local winit 0.30.13 patch (Apache-2.0) are in vendor/winit/UPSTREAM.md; the original LICENSE is retained. Package source identity covers its Cargo.toml, build.rs, and sources/embedded assets under src. The dependency inventory records the local patch and collects that LICENSE automatically.

Qoder CN uses the 640-pixel Qoder asset from Lobe Icons, with provenance, MIT license and SHA-256 in assets/icons. It shares the bounded physical-size area-filtered cache. Agent::QoderCn adds an identity only, preserving persisted session fields, authentication boundaries and the terminal history limit.

Kimi Code uses the 316×316 PNG shown by its official documentation, and ZCode uses the 1024×1024 PNG shown in its official repository README. Both are pinned to commits and retain their original bytes. The repositories' MIT/Apache-2.0 texts are preserved as LICENSE-kimi-code/LICENSE-zcode; repository licenses do not grant trademark rights. TRAE comes from the official CLI documentation site's 2400×2400 PNG, with only outer padding cropped before offline reduction to 640×640; SOURCE.json records the original hash and transform. That official source states no standalone open-source license for its brand asset, so Lobe/MIT must not be assigned to it. Brand icons identify integrations only, with no runtime network requests. ZIPs include provenance and available original licenses in agent-icons/, plus bilingual third-party notices.

The 0.4.7 README support table uses assets/readme/agent-*.png. examples/agent_icons_capture.rs natively invokes the production avatar painter on a dark surface for legibility in light/dark documentation. PNGs are copied unchanged; assets/readme/SOURCE.json records per-file versions, dimensions, SHA-256 and provenance. They contain no user data or real model sessions.

The 0.4.8 preview README main images use Chinese workspace.png/floating.png and English workspace.en.png/floating.en.png. All four are unmodified Release native-component captures with synthetic content and example directories; per-file metadata records version, language, dimensions and SHA-256. English names and terminal examples are also English. ZCode basic settings snapshots exist only in the invocation-owned ignored runtime directory and contain no providers/credentials, custom commands or conversation bodies. The persisted session format is unchanged.

0.4.9 preview: vendor published alacritty_terminal 0.26.0 (Apache-2.0) and vte 0.15.0 (Apache-2.0 OR MIT) with minimal protocol patches. Preserve original licenses and bilingual provenance; include manifests and src Rust files in package identity and license inventories. Font files/provenance stay unchanged. Preview.2 reverts preview.1 GDI classic fitting after user visual review and restores 0.4.8 native default rendering parameters, including color emoji. Cell backgrounds precede block cursors and inverted glyphs; wide characters span two cells.

0.4.9-preview.3 adopts the user-selected Cascadia Mono + system Microsoft YaHei UI with original native rendering parameters and unchanged size. Embed four original static Cascadia Mono v2407.24 faces from Microsoft, pinned by archive/revision/file SHA-256 under SIL OFL 1.1. Hack remains a comparison fixture. Packages contain Cascadia-LICENSE.txt and FONT-SOURCE.json; declared font provenance/license omissions are rejected while old archives remain verifiable. No system Chinese/UI/emoji font is copied or installed.

0.4.9-preview.5: egui_glow CPU staging holds only visible paint vertices, indices and draw ranges, reusing peak-frame capacity without persisting terminal content. The new mesh_batch.rs and vendor provenance remain part of packaged source identity.

Published upstream terminal recordings use binary diffs, and Git EOL conversion is disabled for the two new vendor snapshots. Preserve all original Cascadia-license bytes and whitespace. Project-authored source retains normal formatting and whitespace checks.


0.4.10 preview: `ui.terminal_font` uses stable IDs `jetbrains_mono` (default), `cascadia_mono`, `hack`, `consolas` and `courier_new`. All three embedded families reuse the pinned four styles; system font bytes are not redistributed, and SOURCE.json records catalog/provenance boundaries. scripts/installer pins the NSIS compiler source and original license. Installed programs carry an ownership marker; uninstall deletes only generated listed files and empty directories, never recursively removes the application directory or touches session data. EXE/checksum/JSON outputs and isolated installation evidence stay under ignored output/.

0.4.12 preview: the menu offers six colors plus No color, retaining blue/green/amber/pink/purple IDs and adding red. Legacy teal/orange IDs round-trip unchanged while rendering and menu selection map to blue/red. Session UUIDs, directories and order are untouched. Colors and the No color icon are drawn in code without new bitmap/font assets.

0.4.12 Git adds only the boolean `ui.git_panel_open` preference, defaulting to collapsed for older records. Branch, path, line totals and commit metadata remain in the current in-memory snapshot. Untracked text is read temporarily to count lines; no contents, diffs, history cache or Git credentials are saved. Session data roots and user repositories are unchanged.

0.4.12-preview.3: `assets/fonts/Codicons.ttf`, original CSS glyph mapping and license come from Microsoft's official Codicons 0.0.45 npm package. SOURCE.json records original bytes, archive digest and the U+EC6F branch symbol. This is a UI icon font, neither a terminal-font option nor a system installation. Production appearance and probes share its definition; the helper never loads it. ZIP packaging copies Codicons-LICENSE.txt and validates it as a declared font license.

Commit inspection is not persisted: retain only current and expanded details in memory, display at most 500 files and keep the six-second/2 MiB per-command bounds. Only an explicit copy click writes the full commit ID; the clipboard is never read automatically. Copy/file icons reuse the original Codicons U+EBCC/U+EA7B glyphs without new assets.

0.6.0-preview.2: Changes and commit files share git_ui/files.rs. File-type icons all use the existing original Codicons font; SOURCE.json lists Markdown, code, JSON, configuration, script, media, PDF, archive and binary glyphs in use. No font or bitmap assets were added.

0.7.0 preview: history size is O(1) viewport metadata without full-history copying. Diffs retain one bounded memory result (20,000 lines/4,000 characters per line, 2-MiB command output cap), with no workspace/disk persistence. No fonts/images are added; colors reuse chrome::COMPLETED/FAILED.

Syntect 5.3.0 embeds grammars from the locked crate; default theme resources are disabled and colors are defined by project code. Cargo.lock records the archive checksum; the original MIT LICENSE.txt is packaged through dependencies.json and licenses/syntect-5.3.0/. Current-file presentation retains at most 100,000 tokens, with 256 spans per line, containing byte boundaries, RGB, source-row indices and indentation without workspace persistence. The artifact inventory records fancy-regex and other added dependencies and their original licenses.

preview.6 working-file bodies likewise exist only in the current bounded memory result, with no file copies, index writes or disk diff cache. PatchSource is a presentation-memory type and does not change sessions.json. No fonts, images or third-party dependencies are added.

0.7.0 README images come from optimized readme_capture (default/history/diff) and island_probe. Eight localized workspace/Git-history/read-only-diff/floating PNGs retain native bytes; SOURCE.json records each version, language, dimensions and SHA-256. Code, metadata and history are fixed synthetic content, with no reads of real user configuration, repositories or sessions.

0.8.0 preview adds UiSettings.floating_scale with missing-field compatibility, preserving data roots, session IDs/directories/order, fonts and processes. Both scales save independently, with no new font or icon assets.

0.8.0-preview.2 syntax data comes from Cargo.lock-pinned two-face 0.5.2+bat-0.26.1 using the full Oniguruma variant, without third-party themes. assets/syntaxes/LICENSES.json is exported from the crate acknowledgement::listing().for_syntaxes(), preserving all 66 license texts. SOURCE.json records version, Cargo archive checksum, export command and SHA-256. Governance/packaging check provenance; ZIP includes Syntax-LICENSES.json/SYNTAX-SOURCE.json in its source identity. Licenses for two-face and onig/onig_sys are separately archived with dependencies. Font bytes are unchanged; provenance adds terminal-powershell (U+EBC7), terminal-bash (U+EBCA) and terminal-cmd (U+EBC4).

0.8.0-preview.3 adds no font/icon assets and changes no font bytes or stroke parameters. Avatar caches use final pixel sizes, retaining at most 12 sizes per icon. Form styling/responsive layout changes no workspace fields, directories or data root.

0.9.0 preview adds only a file-panel boolean preference, persisting no file bodies, directory snapshots or recent files. Limits are 1,000 entries per directory, 64 expanded directories, and 2 MiB / 20,000 lines / 4,000 characters per line. UTF-8/BOM and BOM-marked UTF-16 are supported with explicit errors; symlinks/junctions are not followed. Codicons bytes stay unchanged; SOURCE.json only records additional used glyphs. See docs/files.en.md.

The 0.9.0 public README images use readme_capture default/history/diff/files modes and island_probe: ten native Chinese/English PNGs. File-tree fixtures are written only in the capture output directory, with a fixed example path in the header and no user-project reads. Copy unchanged and record each source, version, dimensions and hash; retain the original provenance version of historical Agent avatars.
