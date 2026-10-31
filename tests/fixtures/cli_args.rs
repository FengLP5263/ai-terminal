// A native argv / console fixture; no model, Node, or user CLI required.
use std::io::{self, Write};
fn main() {
    let mut args = std::env::args().skip(1);
    let output = args.next().expect("output path");
    let args: Vec<_> = args.collect();
    std::fs::write(&output, format!("{args:?}")).unwrap();
    println!("CLI_ENTRY_READY");
    io::stdout().flush().unwrap();
    if std::env::var_os("AI_TERMINAL_FIXTURE_INTERACTIVE").is_some() {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        std::fs::write(format!("{output}.input"), input).unwrap();
        println!("CLI_INPUT_RECEIVED");
    } else { std::process::exit(37); }
}
