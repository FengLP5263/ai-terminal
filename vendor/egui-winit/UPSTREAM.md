# egui-winit local patch

[简体中文](UPSTREAM.zh.md) | English

Upstream: https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/crates/egui-winit
Version: 0.35.0; registry archive SHA-256: 9ea6bf3608db949588b95b8b341ee358d0c3f95cf4dc3f53d8d76717edee87db.
License: MIT OR Apache-2.0; both upstream license texts are included.

Preserve a paste request as Event::Paste even when the clipboard has no text. Native terminal widgets can then let the foreground CLI handle image paste. Text paste still normalizes CRLF. The patch does not write or persist clipboard content.

On Windows, position the IME candidate window at the cursor rectangle in physical pixels. Refresh the native form when the cursor geometry changes or composition starts, and translate the native composition end into an empty preedit so widgets discard unfinished text.

The normalized manifest includes local license files for redistribution. Cargo.toml.orig is the unchanged upstream manifest.
