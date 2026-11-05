//! One `EnumWindows` pass per poll, shared by all providers.

use std::collections::HashMap;
use std::sync::Mutex;
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible,
};

#[derive(Debug, Clone)]
pub struct Win {
    pub hwnd: HWND,
    pub class: String,
    pub title: String,
    pub pid: u32,
    pub visible: bool,
    pub minimized: bool,
}

pub struct Snapshot {
    pub windows: Vec<Win>,
    pub foreground: HWND,
}

impl Snapshot {
    /// The user is looking at this window right now.
    pub fn in_view(&self, w: &Win) -> bool {
        w.hwnd == self.foreground && w.visible && !w.minimized
    }
}

/// Process image names by pid (lower-case file name), cached because pids
/// are long-lived and the lookup costs a handle open.
static EXE_CACHE: Mutex<Option<HashMap<u32, String>>> = Mutex::new(None);

pub fn exe_name(pid: u32) -> String {
    let mut cache = EXE_CACHE.lock().unwrap();
    let cache = cache.get_or_insert_with(HashMap::new);
    if cache.len() > 512 {
        cache.clear();
    }
    cache.entry(pid).or_insert_with(|| query_exe(pid).unwrap_or_default()).clone()
}

fn query_exe(pid: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        ok.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        Some(full.rsplit('\\').next().unwrap_or(&full).to_ascii_lowercase())
    }
}

pub fn snapshot() -> Snapshot {
    let mut windows: Vec<Win> = Vec::with_capacity(256);
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut windows as *mut Vec<Win> as isize));
        Snapshot { windows, foreground: GetForegroundWindow() }
    }
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<Win>);
    let visible = IsWindowVisible(hwnd).as_bool();
    let minimized = IsIconic(hwnd).as_bool();

    let mut class = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut class) as usize;
    let class = String::from_utf16_lossy(&class[..n]);
    // Only classes a provider cares about; skip the rest cheaply.
    if class != "OperationStatusWindow" && class != "#32770" {
        return true.into();
    }

    let mut title = [0u16; 256];
    let n = GetWindowTextW(hwnd, &mut title) as usize;
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    list.push(Win {
        hwnd,
        class,
        title: String::from_utf16_lossy(&title[..n]),
        pid,
        visible,
        minimized,
    });
    true.into()
}
