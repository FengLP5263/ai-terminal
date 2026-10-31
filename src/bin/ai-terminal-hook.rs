// A console subsystem preserves ConPTY handles for the optional CLI launcher.
// Lifecycle hooks inherit the agent's existing console; no GUI / GPU is initialized.
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() == Some(std::ffi::OsStr::new("--launch-cli")) {
        std::process::exit(ai_terminal::launch::run_cli(arguments));
    }
    ai_terminal::bridge::report_hook();
}
