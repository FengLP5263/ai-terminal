# Third-party notices

[简体中文](THIRD_PARTY_NOTICES.md) | English

| Component / asset | Declared license | Source and original license |
| --- | --- | --- |
| Local egui_term snapshot | MIT | `vendor/egui_term/LICENSE` and `UPSTREAM.md`; fixed upstream revision. |
| Alacritty terminal | Apache-2.0 | Pinned in `Cargo.lock`; upstream https://github.com/alacritty/alacritty. |
| egui / eframe | MIT OR Apache-2.0 | Pinned in `Cargo.lock`; upstream https://github.com/emilk/egui. |
| Four Hack v3.003 faces | Original files govern (MIT / Bitstream Vera terms) | `assets/fonts/LICENSE.md`, `SOURCE.json`. |
| Other Rust dependencies | Their Cargo metadata and original files govern | Package inventory records versions, declarations, and available license text. |

Windows system fonts are read at runtime and not bundled. The project's LICENSE applies to project code and does not replace third-party licenses. Local packaging produces `dependencies.json`, collects available LICENSE/COPYING/NOTICE files, and records missing texts. This notice records provenance and collection scope; it does not claim a complete legal review of all dependencies, runtimes, or distribution methods.

## Agent icons

- winit 0.30.13: local Windows native-drag DPI compatibility patch, Apache-2.0. See the [patch provenance](vendor/winit/UPSTREAM.md) and original LICENSE. The package dependency inventory collects this local dependency and its license.

OpenAI, Claude and Qoder icons come from [Lobe Icons](https://github.com/lobehub/lobe-icons), MIT. See `assets/icons/SOURCE.json` and `assets/icons/LICENSE-lobe-icons`. Brand names and marks belong to their respective owners.

- egui-winit 0.35.0: local paste-event compatibility patch, MIT OR Apache-2.0. See `vendor/egui-winit/UPSTREAM.md` and its included license files. The packaged dependency inventory includes this patched crate.

- egui_glow 0.35.0: local OpenGL buffer-reuse patch, MIT OR Apache-2.0. See `vendor/egui_glow/UPSTREAM.md` and its included licenses; the artifact dependency inventory records this patch.
- Microsoft.Windows.Console.ConPTY 1.25.260930003: MIT, Microsoft's official x64 native console runtime. See [provenance and checks](assets/windows/conpty/SOURCE.json) and the [original license](assets/windows/conpty/LICENSE.txt). Packages contain conpty.dll, OpenConsole.exe and license/provenance copies; this dependency is recorded separately in manifest.json under native_console.
