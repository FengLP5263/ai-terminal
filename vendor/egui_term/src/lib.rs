mod backend;
mod bindings;
mod font;
#[cfg(windows)]
mod native_text;
mod theme;
mod types;
mod view;

pub use backend::settings::BackendSettings;
pub use backend::{BackendCommand, PtyEvent, TerminalBackend, TerminalMode};
pub use bindings::{Binding, BindingAction, InputKind, KeyboardBinding};
pub use font::{FontSettings, TerminalFont, TextRendererSession};
pub use theme::{ColorPalette, TerminalTheme};
pub use view::TerminalView;
