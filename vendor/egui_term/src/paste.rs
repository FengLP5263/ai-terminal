//! Clipboard image detection is metadata-only and occurs only on a paste request.
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

pub(crate) fn bytes(text: &str, bracketed: bool, native_image: bool) -> Vec<u8> {
    if native_image {
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
                assert_eq!(super::bytes(text, bracketed, true), b"\x1bv");
            }
        }
    }
    #[test]
    fn text_paste_retains_newlines_and_strips_escape_only_in_bracketed_mode() {
        assert_eq!(
            super::bytes("中文\r\nnext", false, false),
            "中文\rnext".as_bytes()
        );
        assert_eq!(
            super::bytes("one\x1b\nnext", true, false),
            b"\x1b[200~one\nnext\x1b[201~"
        );
    }
    #[test]
    fn empty_clipboard_does_not_fake_an_image_paste() {
        assert!(super::bytes("", false, false).is_empty());
    }
}
