# Third-party notices

[简体中文](THIRD_PARTY_NOTICES.md) | English

| Component / asset | Declared license | Source and original license |
| --- | --- | --- |
| Local egui_term snapshot | MIT | `vendor/egui_term/LICENSE` and `UPSTREAM.md`; fixed upstream revision. |
| Alacritty terminal | Apache-2.0 | Pinned in `Cargo.lock`; upstream https://github.com/alacritty/alacritty. |
| egui / eframe | MIT OR Apache-2.0 | Pinned in `Cargo.lock`; upstream https://github.com/emilk/egui. |
| Four Cascadia Mono v2407.24 faces (optional) | SIL Open Font License 1.1 | Microsoft official release archive; `assets/fonts/LICENSE-Cascadia.txt`, `SOURCE.json`. |
| Four Hack v3.003 faces (optional) | Original files govern (MIT / Bitstream Vera terms) | `assets/fonts/LICENSE.md`, `SOURCE.json`. |
| Other Rust dependencies | Their Cargo metadata and original files govern | Package inventory records versions, declarations, and available license text. |

Windows system fonts are read at runtime and not bundled. The project's LICENSE applies to project code and does not replace third-party licenses. Local packaging produces `dependencies.json`, collects available LICENSE/COPYING/NOTICE files, and records missing texts. This notice records provenance and collection scope; it does not claim a complete legal review of all dependencies, runtimes, or distribution methods.

## Agent icons

- winit 0.30.13: local Windows native-drag DPI compatibility patch, Apache-2.0. See the [patch provenance](vendor/winit/UPSTREAM.md) and original LICENSE. The package dependency inventory collects this local dependency and its license.

- OpenAI/Claude/Qoder icons come from [Lobe Icons](https://github.com/lobehub/lobe-icons), MIT; the original text is `assets/icons/LICENSE-lobe-icons`.
- The Kimi Code icon is the PNG used by the official [MoonshotAI/kimi-code](https://github.com/MoonshotAI/kimi-code) documentation, pinned to a commit with original bytes retained. The repository declares MIT; its text is `assets/icons/LICENSE-kimi-code`.
- The ZCode icon is the application PNG shown in the official [zai-org/ZCode](https://github.com/zai-org/ZCode) README, pinned to a commit with original bytes retained. The repository declares Apache-2.0; its text is `assets/icons/LICENSE-zcode`.
- The TRAE icon is the PNG referenced by the [official CLI documentation site](https://docs.trae.cn/cli_about-trae-code-cli-2), with only outer padding cropped and the result reduced offline. That official source states no standalone open-source license for the brand asset; the Lobe Icons MIT declaration does not apply.

Brand icons identify integrations only. Names and marks belong to their respective owners; repository licenses do not grant trademark rights. `assets/icons/SOURCE.json` records per-file URLs, pinned revisions, dimensions, transforms and SHA-256. Packages include the same provenance and the available original license texts above in `agent-icons/`.

- egui-winit 0.35.0: local paste-event compatibility patch, MIT OR Apache-2.0. See `vendor/egui-winit/UPSTREAM.md` and its included license files. The packaged dependency inventory includes this patched crate.

- egui_glow 0.35.0: local OpenGL buffer-reuse patch, MIT OR Apache-2.0. See `vendor/egui_glow/UPSTREAM.md` and its included licenses; the artifact dependency inventory records this patch.
- Microsoft.Windows.Console.ConPTY 1.25.260930003: MIT, Microsoft's official x64 native console runtime. See [provenance and checks](assets/windows/conpty/SOURCE.json) and the [original license](assets/windows/conpty/LICENSE.txt). Packages contain conpty.dll, OpenConsole.exe and license/provenance copies; this dependency is recorded separately in manifest.json under native_console.

## JetBrains Mono

The optional terminal font comes from [JetBrains official v2.304](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304) under SIL OFL 1.1. Four static styles and the license retain their original bytes; `assets/fonts/SOURCE.json` records provenance/SHA-256. Source license: [LICENSE-JetBrainsMono.txt](assets/fonts/LICENSE-JetBrainsMono.txt). The ZIP includes `JetBrainsMono-LICENSE.txt`; system fonts are not copied or redistributed.

## Codicons

The Git branch, commit, copy and file icons use the original font and `git-branch` (U+EC6F), `copy` (U+EBCC), `git-commit` (U+EAFC) and `file` (U+EA7B), plus file-type glyphs from Microsoft's official [VS Code Codicons](https://github.com/microsoft/vscode-codicons) `@vscode/codicons` 0.0.45 package, licensed CC BY 4.0. Font, CSS mapping and license bytes are unchanged. See `assets/fonts/SOURCE.json` for provenance/hashes and `assets/fonts/LICENSE-Codicons.txt` for the original license; the ZIP includes `Codicons-LICENSE.txt`. It is only an in-app symbol, with no system font installation or terminal-font option.

## File syntax highlighting

[Syntect 5.3.0](https://github.com/trishume/syntect/tree/v5.3.0) is MIT licensed. Default grammars are embedded from the locked crates.io archive; default theme resources are disabled and colors are project-defined. The original LICENSE.txt is included under licenses/syntect-5.3.0/; Cargo.lock/dependencies.json record dependency identity and checksums. fancy-regex, bincode, bit-set/bit-vec, fnv and other dependencies retain their own declarations and original licenses.

0.8.0-preview.2 uses the [two-face 0.5.2+bat-0.26.1](https://docs.rs/two-face/0.5.2+bat-0.26.1/two_face/) (MIT OR Apache-2.0) bat syntax set with Oniguruma. Original onig/onig_sys licenses are archived with dependencies; no themes are loaded. Individual syntax licenses are in [LICENSES.json](assets/syntaxes/LICENSES.json), with pinned provenance/hashes in [SOURCE.json](assets/syntaxes/SOURCE.json). ZIP copies are Syntax-LICENSES.json/SYNTAX-SOURCE.json. Shell avatars reuse terminal-powershell, terminal-bash and terminal-cmd from existing Codicons 0.0.45; original font bytes and CC BY 4.0 license are unchanged.

## Outline file-type icons

File types use original outline SVGs from [Tabler Icons](https://github.com/tabler/tabler-icons), licensed MIT, with only runtime UI tinting. See [SOURCE.json](assets/file-icons/SOURCE.json) and the original [LICENSE.txt](assets/file-icons/LICENSE.txt) for the pinned commit and per-file SHA-256. ZIPs include `Tabler-Icons-LICENSE.txt` and `FILE-ICONS-SOURCE.json`.

Local SVG rasterization uses resvg/usvg 0.45.1 (Apache-2.0 OR MIT) and tiny-skia 0.11.4 (BSD-3-Clause), pinned in Cargo.lock with original licenses archived in the dependency inventory. Text, system fonts and raster-image features are disabled. Only embedded icons are rendered; project SVGs and browser runtimes are not loaded.
