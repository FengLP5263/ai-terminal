//! Clipboard formats are read only on paste. Files become paths; image bytes stay owned by the CLI.
#[cfg(windows)]
pub(crate) fn has_image() -> bool {
    use windows_sys::Win32::System::DataExchange::{
        IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };
    static PNG: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let png = *PNG.get_or_init(|| unsafe { RegisterClipboardFormatW([80u16, 78, 71, 0].as_ptr()) });
    // CF_BITMAP, CF_DIB, CF_DIBV5 and the registered PNG format.
    [2, 8, 17, png]
        .into_iter()
        .any(|format| format != 0 && unsafe { IsClipboardFormatAvailable(format) != 0 })
}
#[cfg(not(windows))]
pub(crate) fn has_image() -> bool {
    false
}

/// Explorer copies CF_HDROP instead of Unicode text. Never read file contents or alter the clipboard.
#[cfg(windows)]
pub(crate) fn files() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    const CF_HDROP: u32 = 15;
    if unsafe { IsClipboardFormatAvailable(CF_HDROP) } == 0 {
        return None;
    }
    if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
        return None;
    }
    struct Close;
    impl Drop for Close {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    let _close = Close;
    let drop = unsafe { GetClipboardData(CF_HDROP) };
    if drop.is_null() {
        return None;
    }
    drop_files(drop)
}
#[cfg(not(windows))]
pub(crate) fn files() -> Option<String> {
    None
}

// Callers keep the borrowed HDROP valid for this call; ownership is never transferred.
#[cfg(windows)]
pub(crate) fn drop_files(drop: windows_sys::Win32::UI::Shell::HDROP) -> Option<String> {
    use windows_sys::Win32::UI::Shell::DragQueryFileW;
    collect_files(|index, buffer| unsafe {
        match buffer {
            Some(buffer) => DragQueryFileW(drop, index, buffer.as_mut_ptr(), buffer.len() as u32),
            None => DragQueryFileW(drop, index, std::ptr::null_mut(), 0),
        }
    })
}

#[cfg(any(windows, test))]
fn collect_files(mut query: impl FnMut(u32, Option<&mut [u16]>) -> u32) -> Option<String> {
    // Bound allocation and terminal input; reject the whole list rather than silently dropping files.
    const MAX_FILES: u32 = 1024;
    const MAX_UNITS: usize = 64 * 1024;
    let count = query(u32::MAX, None);
    if count == 0 || count > MAX_FILES {
        return None;
    }
    let mut paths = Vec::new();
    let mut units = 0;
    for index in 0..count {
        let size = query(index, None) as usize;
        units += size + 3;
        if size == 0 || size > 32767 || units > MAX_UNITS {
            return None;
        }
        let mut buffer = vec![0; size + 1];
        if query(index, Some(&mut buffer)) as usize != size {
            return None;
        }
        let path = String::from_utf16(&buffer[..size]).ok()?;
        if path.chars().any(|c| c.is_control() || c == '"') {
            return None;
        }
        paths.push(format!("\"{path}\""));
    }
    Some(paths.join(" "))
}

pub(crate) fn bytes(
    text: &str,
    bracketed: bool,
    native_image: bool,
    files: Option<&str>,
) -> Vec<u8> {
    let text = files.unwrap_or(text);
    if native_image && files.is_none() {
        // Both Codex and Claude Code accept Alt+V for native Windows images.
        // Keep the clipboard owned by the CLI: no encoding, temp file or upload here.
        return b"\x1bv".to_vec();
    }
    if bracketed {
        format!("\x1b[200~{}\x1b[201~", text.replace('\x1b', "")).into_bytes()
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn image_paste_is_one_native_key_even_with_text_or_bracketed_mode() {
        for text in ["", "clipboard text beside the bitmap"] {
            for bracketed in [false, true] {
                assert_eq!(super::bytes(text, bracketed, true, None), b"\x1bv");
            }
        }
    }
    #[test]
    fn text_paste_retains_newlines_and_strips_escape_only_in_bracketed_mode() {
        assert_eq!(
            super::bytes("中文\r\nnext", false, false, None),
            "中文\rnext".as_bytes()
        );
        assert_eq!(
            super::bytes("one\x1b\nnext", true, false, None),
            b"\x1b[200~one\nnext\x1b[201~"
        );
    }
    #[test]
    fn empty_clipboard_does_not_fake_an_image_paste() {
        assert!(super::bytes("", false, false, None).is_empty());
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;
    fn fixture(paths: &[&str]) -> Option<String> {
        collect_files(|index, buffer| {
            if index == u32::MAX {
                return paths.len() as u32;
            }
            let path = paths[index as usize].encode_utf16().collect::<Vec<_>>();
            if let Some(buffer) = buffer {
                buffer[..path.len()].copy_from_slice(&path);
                buffer[path.len()] = 0;
            }
            path.len() as u32
        })
    }
    #[test]
    fn files_preserve_unicode_spaces_unc_and_order_without_enter_or_image_shortcut() {
        let paths = fixture(&[
            r"C:\项目 文件\图😀.png",
            r"\\server\share\a.txt",
            r"C:\folder",
        ])
        .unwrap();
        assert_eq!(
            paths,
            "\"C:\\项目 文件\\图😀.png\" \"\\\\server\\share\\a.txt\" \"C:\\folder\""
        );
        for bracketed in [false, true] {
            let actual = bytes("alternate text", bracketed, true, Some(&paths));
            let expected = if bracketed {
                format!("\x1b[200~{paths}\x1b[201~").into_bytes()
            } else {
                paths.as_bytes().to_vec()
            };
            assert_eq!(actual, expected);
            assert!(!actual.contains(&b'\r') && !actual.contains(&b'\n'));
        }
    }
    #[test]
    fn malformed_or_oversized_file_lists_never_produce_partial_or_control_input() {
        assert!(fixture(&[]).is_none());
        assert!(fixture(&["good", "bad\ncommand"]).is_none());
        assert!(fixture(&["bad\"name"]).is_none());
        assert!(collect_files(|_, _| 1025).is_none());
        assert!(fixture(&[&"a".repeat(32768)]).is_none());
        assert!(fixture(&[&"a".repeat(32767), &"b".repeat(32767)]).is_none());
        assert!(collect_files(|index, buffer| if index == u32::MAX {
            1
        } else if buffer.is_some() {
            0
        } else {
            4
        })
        .is_none());
        assert!(collect_files(|index, buffer| {
            if index == u32::MAX {
                return 1;
            };
            if let Some(buffer) = buffer {
                buffer[0] = 0xd800;
            }
            1
        })
        .is_none());
    }
    #[cfg(windows)]
    #[test]
    fn native_hdrop_decoding_uses_owned_memory_without_changing_the_system_clipboard() {
        use windows_sys::Win32::{
            Foundation::GlobalFree,
            System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT},
        };
        let path = "C:\\项目 文件\\图😀.png";
        let mut units = path.encode_utf16().collect::<Vec<_>>();
        units.extend([0, 0]);
        unsafe {
            let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, 20 + units.len() * 2);
            assert!(!memory.is_null());
            let ptr = GlobalLock(memory).cast::<u8>();
            assert!(!ptr.is_null());
            // DROPFILES: pFiles=20, pt=(0,0), fNC=0, fWide=1.
            ptr.cast::<u32>().write(20);
            ptr.add(16).cast::<i32>().write(1);
            std::ptr::copy_nonoverlapping(units.as_ptr(), ptr.add(20).cast::<u16>(), units.len());
            GlobalUnlock(memory);
            let result = drop_files(memory);
            GlobalFree(memory);
            assert_eq!(result, Some(format!("\"{path}\"")));
        }
    }
}
