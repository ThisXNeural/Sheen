//! Clipboard peek: a message-only window registered with
//! `AddClipboardFormatListener`, so it costs nothing until something is
//! copied. Content that password managers mark as private
//! (`ExcludeClipboardContentFromMonitorProcessing`) is never read.

use crate::Signal;
use base64::Engine;
use sheen_core::{OsdKind, SheenEvent};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, GetClipboardData, GetClipboardOwner,
    IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowThreadProcessId,
    KillTimer, RegisterClassW, SetTimer, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_CLIPBOARDUPDATE, WM_TIMER, WNDCLASSW,
};

/// Apps often write several formats in quick succession; read once they settle.
const SETTLE_MS: u32 = 120;
const TIMER_ID: usize = 1;
const TEXT_PREVIEW: usize = 140;
const THUMB: u32 = 96;

static OUT: OnceLock<Sender<Signal>> = OnceLock::new();

pub fn spawn_clipboard_listener(out: Sender<Signal>) {
    if OUT.set(out).is_err() {
        return;
    }
    std::thread::Builder::new()
        .name("sheen-clipboard".into())
        .spawn(|| unsafe {
            let instance = GetModuleHandleW(None).unwrap_or_default();
            let class = w!("SheenClipboardListener");
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            });
            let Ok(hwnd) = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            ) else {
                return log::warn!("clipboard listener window failed");
            };
            if AddClipboardFormatListener(hwnd).is_err() {
                return log::warn!("AddClipboardFormatListener failed");
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                DispatchMessageW(&msg);
            }
        })
        .expect("spawn clipboard listener");
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            SetTimer(Some(hwnd), TIMER_ID, SETTLE_MS, None);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_ID => {
            let _ = KillTimer(Some(hwnd), TIMER_ID);
            if let Some(osd) = read(hwnd) {
                if let Some(out) = OUT.get() {
                    let _ = out.send(Signal::Event(SheenEvent::Osd(osd)));
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn read(hwnd: HWND) -> Option<OsdKind> {
    // Our own copies (e.g. from the note) need no confirmation.
    if let Ok(owner) = GetClipboardOwner() {
        let mut pid = 0;
        GetWindowThreadProcessId(owner, Some(&mut pid));
        if pid == std::process::id() {
            return None;
        }
    }
    for private in
        [w!("ExcludeClipboardContentFromMonitorProcessing"), w!("Clipboard Viewer Ignore")]
    {
        let fmt = RegisterClipboardFormatW(private);
        if fmt != 0 && IsClipboardFormatAvailable(fmt).is_ok() {
            return None;
        }
    }

    // Another app may hold the clipboard briefly.
    let mut opened = false;
    for _ in 0..4 {
        if OpenClipboard(Some(hwnd)).is_ok() {
            opened = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    if !opened {
        return None;
    }
    let result = read_open();
    let _ = CloseClipboard();
    result
}

unsafe fn read_open() -> Option<OsdKind> {
    if IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok() {
        let h = GetClipboardData(CF_HDROP.0 as u32).ok()?;
        let drop = HDROP(h.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        let mut buf = [0u16; 260];
        let n = DragQueryFileW(drop, 0, Some(&mut buf)) as usize;
        let first = String::from_utf16_lossy(&buf[..n]);
        let name = first.rsplit('\\').next().unwrap_or(&first).to_string();
        return Some(OsdKind::Clipboard { text: Some(name), image: None, files: count });
    }
    if IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok() {
        let h = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
        let text = with_global(h, |bytes| {
            let wide: &[u16] = std::slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len() / 2);
            let end = wide.iter().position(|c| *c == 0).unwrap_or(wide.len());
            String::from_utf16_lossy(&wide[..end.min(4096)])
        })?;
        return Some(OsdKind::Clipboard { text: Some(preview(&text)), image: None, files: 0 });
    }
    if IsClipboardFormatAvailable(CF_DIB.0 as u32).is_ok() {
        let h = GetClipboardData(CF_DIB.0 as u32).ok()?;
        let thumb = with_global(h, dib_thumbnail).flatten();
        return Some(OsdKind::Clipboard { text: None, image: thumb, files: 0 });
    }
    None
}

unsafe fn with_global<T>(h: HANDLE, f: impl FnOnce(&[u8]) -> T) -> Option<T> {
    let global = HGLOBAL(h.0);
    let size = GlobalSize(global);
    let ptr = GlobalLock(global) as *const u8;
    if ptr.is_null() {
        return None;
    }
    let result = f(std::slice::from_raw_parts(ptr, size));
    let _ = GlobalUnlock(global);
    Some(result)
}

/// Collapse whitespace and cut to a short single-line preview.
pub(crate) fn preview(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > TEXT_PREVIEW {
        let cut: String = collapsed.chars().take(TEXT_PREVIEW).collect();
        format!("{}…", cut.trim_end())
    } else {
        collapsed
    }
}

/// A packed DIB from the clipboard → small JPEG `data:` URL.
fn dib_thumbnail(dib: &[u8]) -> Option<String> {
    if dib.len() < 40 {
        return None;
    }
    // Prepend a BITMAPFILEHEADER so the BMP decoder can read it.
    let header_size = u32::from_le_bytes(dib[0..4].try_into().ok()?) as usize;
    let bit_count = u16::from_le_bytes(dib[14..16].try_into().ok()?) as usize;
    let compression = u32::from_le_bytes(dib[16..20].try_into().ok()?);
    let colors_used = u32::from_le_bytes(dib[32..36].try_into().ok()?) as usize;
    let palette = if bit_count <= 8 {
        (if colors_used == 0 { 1 << bit_count } else { colors_used }) * 4
    } else {
        0
    };
    let masks = if compression == 3 && header_size == 40 { 12 } else { 0 }; // BI_BITFIELDS
    let offset = 14 + header_size + masks + palette;
    let mut bmp = Vec::with_capacity(14 + dib.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((14 + dib.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&(offset as u32).to_le_bytes());
    bmp.extend_from_slice(dib);

    let img = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).ok()?;
    let thumb = img.thumbnail(THUMB, THUMB).to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80).encode_image(&thumb).ok()?;
    Some(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(jpeg)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_are_single_line_and_short() {
        assert_eq!(preview("  hello\n\n  world\t!"), "hello world !");
        let long = "word ".repeat(100);
        let p = preview(&long);
        assert!(p.ends_with('…'));
        assert!(p.chars().count() <= TEXT_PREVIEW + 1);
    }

    #[test]
    fn decodes_a_24bit_dib() {
        // 2×2 BI_RGB, 24-bit, rows padded to 4 bytes.
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&24u16.to_le_bytes());
        dib.extend_from_slice(&[0; 24]);
        dib.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0]);
        dib.extend_from_slice(&[0, 0, 255, 255, 255, 255, 0, 0]);
        let url = dib_thumbnail(&dib).unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }
}
