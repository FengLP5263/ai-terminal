#![cfg(windows)]

// Exercise the same capacity policy used by the patched OpenGL painter.
#[path = "../vendor/egui_glow/src/buffer_capacity.rs"]
mod buffer_capacity;
