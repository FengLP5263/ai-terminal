// Console subsystem: UTF-8 JSON on stdout; no GUI, fonts or GPU initialization.
fn main() {
    std::process::exit(ai_terminal::control::cli::run(
        std::env::args_os().skip(1).collect(),
    ));
}
