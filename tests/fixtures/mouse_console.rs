//! Local raw-console mouse fixture; never invokes an Agent or a model.
use std::io::{Read, Write};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(kind: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
    fn GetConsoleScreenBufferInfo(handle: *mut std::ffi::c_void, info: *mut ConsoleInfo) -> i32;
}
#[repr(C)]
#[derive(Default)]
struct Coord { x: i16, y: i16 }
#[repr(C)]
#[derive(Default)]
struct SmallRect { left: i16, top: i16, right: i16, bottom: i16 }
#[repr(C)]
#[derive(Default)]
struct ConsoleInfo { size: Coord, cursor: Coord, attributes: u16, window: SmallRect, maximum: Coord }
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "sgr".into());
    let trace = std::env::var_os("AI_TERMINAL_MOUSE_TRACE").map(std::path::PathBuf::from);
    let mut traced_bytes = 0;
    unsafe {
        let input = GetStdHandle(-10_i32 as u32);
        let mut flags = 0;
        assert_ne!(GetConsoleMode(input, &mut flags), 0);
        assert_ne!(SetConsoleMode(input, (flags & !7) | 0x200), 0);
        if let Some(path) = &trace {
            std::fs::write(path, format!("input mode: {:X}\n", (flags & !7) | 0x200)).unwrap();
        }
        let output = GetStdHandle(-11_i32 as u32);
        let mut output_flags = 0;
        assert_ne!(GetConsoleMode(output, &mut output_flags), 0);
        assert_ne!(SetConsoleMode(output, output_flags | 4), 0);
        if let Some(path) = &trace {
            let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
            writeln!(file, "output mode before VT: {output_flags:X}").unwrap();
        }
    }
    let sgr = mode != "legacy";
    print!(
        "\x1b[?1049h{}{}APP_WHEEL_READY",
        if mode == "pager" || mode == "select-all" {
            "\x1b[?1003h"
        } else if mode == "select-motion" {
            "\x1b[?1002h"
        } else if mode == "local" || mode == "composer" {
            ""
        } else {
            "\x1b[?1000h"
        },
        if sgr { "\x1b[?1006h" } else { "" }
    );
    if mode != "pager" && mode != "composer" {
        print!("\r\nSELECT_ME local selection fixture\r\n");
    }
    std::io::stdout().flush().unwrap();
    let mut input = std::io::stdin().lock();
    let mut report = 0;
    let mut offset = 100;
    if mode == "pager" {
        render(offset);
    } else if mode == "composer" {
        render_composer();
    }
    loop {
        let mut bytes = Vec::new();
        loop {
            let mut b = [0];
            if input.read_exact(&mut b).is_err() {
                return;
            }
            if traced_bytes < 256 {
                if let Some(path) = &trace {
                    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
                    writeln!(file, "{:02X}", b[0]).unwrap();
                }
                traced_bytes += 1;
            }
            bytes.push(b[0]);
            if (sgr && (b[0] == b'M' || b[0] == b'm'))
                || (!sgr && bytes.len() == 6)
                || (bytes.len() == 1 && b[0] != 0x1b)
            {
                break;
            }
            if bytes.len() > 128 {
                return;
            }
        }
        if mode == "composer" {
            render_composer();
        } else if mode == "pager" {
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
fn render_composer() {
    let mut info = ConsoleInfo::default();
    unsafe { assert_ne!(GetConsoleScreenBufferInfo(GetStdHandle(-11_i32 as u32), &mut info), 0); }
    let rows = info.window.bottom - info.window.top + 1;
    let cols = info.window.right - info.window.left + 1;
    assert!((4..=1000).contains(&rows));
    print!("\x1b[2J\x1b[H\x1b[1;{}r", rows - 2);
    // A completed response overflows the transcript region while the final
    // two rows remain reserved for an editable composer and status line.
    for line in 0..rows * 3 { print!("Response line {line:03}\r\n"); }
    print!("\x1b[r\x1b[HGRID rows={rows} cols={cols}\x1b[{};1H\x1b[48;2;35;35;35mCOMPOSER> type here\x1b[K\x1b[{};1HSTATUS: READY\x1b[K\x1b[0m\x1b[{};11H", rows-1, rows, rows-1);
    std::io::stdout().flush().unwrap();
}
fn render(offset: i32) {
    print!("\x1b[H\x1b[2JAPP_WHEEL_READY | history offset {offset}\r\n");
    for i in offset..offset + 24 {
        println!("TOP_MESSAGE_{i:03} | local conversation fixture\r");
    }
    std::io::stdout().flush().unwrap();
}
