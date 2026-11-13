pub mod appearance;
pub mod bridge;
pub mod chrome;
pub mod connection_ui;
pub mod dialogs;
pub mod discovery;
pub mod font_choice;
pub mod git;
pub mod git_ui;
pub mod launch;
pub mod model;
pub mod native_console;
pub mod shell;
pub mod shortcut;
pub mod sidebar;
pub mod status;
pub mod ui_text;
pub mod window_frame;
pub mod zoom;

pub mod floating;
#[cfg(windows)]
mod floating_group;
#[cfg(windows)]
mod floating_hint;
mod floating_native;
pub mod i18n;
pub mod settings;

pub mod integration;

pub mod file_icons;
pub mod project_files;

pub mod project_ui;
