# egui_glow local patch

[简体中文](UPSTREAM.zh.md) | English

Upstream: https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/crates/egui_glow
Version: 0.35.0; registry archive SHA-256: 52274b9bfb8d8e252cd0f00c9f6214f360d75a0962baa77a2d62ddb002fe99d4.
License: MIT OR Apache-2.0; both upstream license texts are included.

On Windows, reuse vertex and index buffer storage and grow it as required instead of replacing storage for every mesh. Repeated native glyph rendering on the observed OpenGL driver otherwise caused process-private memory to grow over time. Draw order, textures and vertex contents are unchanged; other targets keep the upstream upload path. Oversized buffers retain the upstream fallback.

The normalized manifest includes local license files for redistribution. Cargo.toml.orig is the unchanged upstream manifest.
