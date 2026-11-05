//! Classic Win32 progress dialogs: IDM's download windows, 7-Zip and WinRAR
//! extraction, many installers. They are `#32770` dialogs with a standard
//! `msctls_progress32` control whose position can be read with
//! `PBM_GETPOS`/`PBM_GETRANGE` from any process.
//!
//! A static bar (a meter in a settings dialog, say) is not a transfer, so a
//! dialog is only reported once its bar has moved.

use super::windows_list::{exe_name, Snapshot};
use super::{strip_percent, transfer_id};
use sheen_core::{TransferKind, TransferSample, TransferUnit};
use std::collections::HashMap;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::{PBM_GETPOS, PBM_GETRANGE, PBS_MARQUEE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetClassNameW, GetWindowLongW, SendMessageTimeoutW, GWL_STYLE,
    SMTO_ABORTIFHUNG, WS_VISIBLE,
};

#[derive(Default)]
pub struct DialogProvider {
    /// hwnd → (first position seen, has moved).
    seen: HashMap<isize, (i64, bool)>,
}

impl DialogProvider {
    pub fn poll(&mut self, snap: &Snapshot) -> Vec<TransferSample> {
        let mut out = Vec::new();
        let mut alive = Vec::new();
        for w in snap.windows.iter().filter(|w| w.class == "#32770") {
            let Some(bar) = find_progress(w.hwnd) else { continue };
            let Some((pos, max)) = read_progress(bar) else { continue };
            let key = w.hwnd.0 as isize;
            alive.push(key);
            let state = self.seen.entry(key).or_insert((pos, false));
            if pos != state.0 {
                state.1 = true;
            }
            if !state.1 || max <= 0 {
                continue;
            }
            let exe = exe_name(w.pid);
            let (app, kind) = app_for(&exe);
            let name = strip_percent(&w.title);
            out.push(TransferSample {
                id: transfer_id(("dialog", key)),
                kind,
                app,
                name: if name.is_empty() { exe.trim_end_matches(".exe").to_string() } else { name },
                done: pos.clamp(0, max) as u64,
                total: Some(max as u64),
                unit: TransferUnit::Ratio,
                source_in_view: snap.in_view(w),
            });
        }
        self.seen.retain(|k, _| alive.contains(k));
        out
    }
}

fn app_for(exe: &str) -> (String, TransferKind) {
    match exe {
        "idman.exe" => ("IDM".into(), TransferKind::Download),
        "7zg.exe" | "7zfm.exe" => ("7-Zip".into(), TransferKind::Copy),
        "winrar.exe" => ("WinRAR".into(), TransferKind::Copy),
        other => {
            let name = other.trim_end_matches(".exe");
            let mut c = name.chars();
            let pretty = c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default();
            (pretty, TransferKind::Copy)
        }
    }
}

fn find_progress(dialog: HWND) -> Option<HWND> {
    let mut found: Option<HWND> = None;
    unsafe {
        let _ = EnumChildWindows(
            Some(dialog),
            Some(visit),
            LPARAM(&mut found as *mut Option<HWND> as isize),
        );
    }
    found
}

unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found = &mut *(lparam.0 as *mut Option<HWND>);
    let mut class = [0u16; 32];
    let n = GetClassNameW(hwnd, &mut class) as usize;
    if String::from_utf16_lossy(&class[..n]).eq_ignore_ascii_case("msctls_progress32") {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if style & WS_VISIBLE.0 != 0 && style & PBS_MARQUEE != 0 {
            return true.into(); // indeterminate spinner: no progress to read
        }
        if style & WS_VISIBLE.0 != 0 {
            *found = Some(hwnd);
            return false.into();
        }
    }
    true.into()
}

fn send(hwnd: HWND, msg: u32, wparam: usize) -> Option<i64> {
    let mut result = 0usize;
    let ok = unsafe {
        SendMessageTimeoutW(
            hwnd,
            msg,
            WPARAM(wparam),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            50,
            Some(&mut result),
        )
    };
    (ok.0 != 0).then_some(result as i64)
}

fn read_progress(bar: HWND) -> Option<(i64, i64)> {
    let pos = send(bar, PBM_GETPOS, 0)?;
    let low = send(bar, PBM_GETRANGE, 1)?; // wParam TRUE: low limit
    let high = send(bar, PBM_GETRANGE, 0)?; // wParam FALSE: high limit
    Some((pos - low, high - low))
}
