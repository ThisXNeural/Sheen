//! "Game mode": detect a full-screen app on the island's (primary) monitor.

use crate::Signal;
use sheen_core::SheenEvent;
use std::sync::mpsc::Sender;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindowRect, GetWindowThreadProcessId,
};

const POLL: Duration = Duration::from_millis(750);

/// Polls rather than hooking `EVENT_SYSTEM_FOREGROUND`: the check is a handful
/// of cheap syscalls, and polling also catches windows that go full-screen
/// without changing focus (F11 in a browser, a video player toggling).
pub fn spawn_fullscreen_watcher(tx: Sender<Signal>) {
    std::thread::Builder::new()
        .name("sheen-fullscreen".into())
        .spawn(move || {
            let mut last = false;
            loop {
                let now = is_fullscreen_active();
                if now != last {
                    last = now;
                    if tx.send(Signal::Event(SheenEvent::Fullscreen(now))).is_err() {
                        return;
                    }
                }
                std::thread::sleep(POLL);
            }
        })
        .expect("spawn fullscreen watcher");
}

fn is_fullscreen_active() -> bool {
    unsafe {
        // The shell already tracks exclusive D3D and presentation mode. Other
        // full-screen apps (borderless games, F11 browsers, video players) are
        // caught by checking whether the foreground window covers the monitor.
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE {
                return true;
            }
        }
        foreground_covers_primary_monitor()
    }
}

unsafe fn foreground_covers_primary_monitor() -> bool {
    let fg = GetForegroundWindow();
    if fg.is_invalid() || fg == GetShellWindow() || is_desktop_or_shell(fg) {
        return false;
    }
    // Ignore our own windows.
    let mut pid = 0u32;
    GetWindowThreadProcessId(fg, Some(&mut pid));
    if pid == std::process::id() {
        return false;
    }

    let primary = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
    if MonitorFromWindow(fg, MONITOR_DEFAULTTONULL) != primary {
        return false;
    }
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let mut rect = RECT::default();
    if !GetMonitorInfoW(primary, &mut info).as_bool() || GetWindowRect(fg, &mut rect).is_err() {
        return false;
    }
    let m = info.rcMonitor;
    rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
}

unsafe fn is_desktop_or_shell(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let len = GetClassNameW(hwnd, &mut buf) as usize;
    let class = String::from_utf16_lossy(&buf[..len]);
    matches!(
        class.as_str(),
        "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
    )
}
