# winit local patch

[简体中文](UPSTREAM.zh.md) | English

Upstream: https://github.com/rust-windowing/winit/tree/e9809ef54b18499bb4f2cac945719ecc2a61061b

Version: 0.30.13. Registry archive SHA-256: a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d.

License: Apache-2.0; the original LICENSE is retained. The local snapshot omits the registry cache marker and the dependency's standalone Cargo.lock; the repository root lockfile is authoritative.

The README's contribution link points to the pinned upstream revision because the registry archive omits CONTRIBUTING.md. Other upstream documentation is preserved.

During Windows native movement, apply the WM_DPICHANGED suggested rectangle directly. Recomputing its mouse anchor and nudging it back toward the previous monitor can recursively change DPI, stretch the window and flicker. ScaleFactorChanged delivery and the existing non-drag/maximized/fullscreen paths remain intact. Reference: [upstream issue 4600](https://github.com/rust-windowing/winit/issues/4600) and [Microsoft guidance](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged). This is a local compatibility patch, not an upstream merged fix.

Validation uses the application's two-window native probe with move state held across real DPI boundaries, suggested/actual rectangle comparisons, recursion counts and post-release dimensions. Programmatic window movement does not establish physical mouse-drag acceptance. Upstream tests/examples remain source references; the project does not claim to have run the full upstream test suite.
