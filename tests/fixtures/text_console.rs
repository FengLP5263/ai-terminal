//! Fixed native text output for ConPTY renderer tests; no Shell or Agent.
use std::io::Write;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(kind: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
}
fn main() {
    unsafe {
        let output = GetStdHandle(-11_i32 as u32);
        let mut flags = 0;
        assert_ne!(GetConsoleMode(output, &mut flags), 0);
        assert_ne!(SetConsoleMode(output, flags | 4), 0);
    }
    let mode = std::env::args().nth(1).expect("text fixture mode");
    match mode.as_str() {
        "typography" => print!("\x1b[2J\x1b[HABCxyz 0123 你好 \x1b[1mBOLD \x1b[0;3mITALIC \x1b[1;3mBOTH\x1b[0m DONE"),
        "emoji" => print!("\x1b[2J\x1b[H👋 👋🏽 ❤️ 👩‍💻 🇨🇳 DONE"),
        "caret-2" => print!("\x1b[2J\x1b[HA\x1b[H\x1b[2 q"),
        "caret-6" => print!("\x1b[2J\x1b[HA\x1b[H\x1b[6 q"),
        _ => panic!("unknown text fixture mode"),
    }
    std::io::stdout().flush().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(20));
}
