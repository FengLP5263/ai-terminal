//! Local raw-console mouse fixture; never invokes an Agent or a model.
use std::io::{Read, Write};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(kind: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
}
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "sgr".into());
    unsafe {
        let input = GetStdHandle(-10_i32 as u32);
        let mut flags = 0;
        assert_ne!(GetConsoleMode(input, &mut flags), 0);
        assert_ne!(SetConsoleMode(input, (flags & !7) | 0x200), 0);
    }
    let sgr = mode != "legacy";
    print!(
        "\x1b[?1049h{}{}APP_WHEEL_READY",
        if mode == "pager" {
            "\x1b[?1003h"
        } else {
            "\x1b[?1000h"
        },
        if sgr { "\x1b[?1006h" } else { "" }
    );
    std::io::stdout().flush().unwrap();
    let mut input = std::io::stdin().lock();
    let mut report = 0;
    let mut offset = 100;
    if mode == "pager" {
        render(offset);
    }
    loop {
        let mut bytes = Vec::new();
        loop {
            let mut b = [0];
            if input.read_exact(&mut b).is_err() {
                return;
            }
            bytes.push(b[0]);
            if (sgr && b[0] == b'M') || (!sgr && bytes.len() == 6) {
                break;
            }
            if bytes.len() > 128 {
                return;
            }
        }
        if mode == "pager" {
            if bytes.starts_with(b"\x1b[<64;") {
                offset = (offset - 5).max(0);
            } else if bytes.starts_with(b"\x1b[<65;") {
                offset = (offset + 5).min(100);
            }
            render(offset);
        } else {
            let hex = bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join("-");
            println!("REPORT_{report}_{hex}");
            report += 1;
            std::io::stdout().flush().unwrap();
        }
    }
}
fn render(offset: i32) {
    print!("\x1b[H\x1b[2JAPP_WHEEL_READY | history offset {offset}\r\n");
    for i in offset..offset + 24 {
        println!("TOP_MESSAGE_{i:03} | local conversation fixture\r");
    }
    std::io::stdout().flush().unwrap();
}
