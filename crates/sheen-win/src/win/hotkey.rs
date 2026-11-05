//! Global shortcuts via `RegisterHotKey`. One thread with a message queue owns
//! the registrations; requests to change them are posted to it.

use sheen_core::Accelerator;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, WM_APP, WM_HOTKEY,
};

struct Request {
    id: i32,
    accelerator: Option<Accelerator>,
    reply: Sender<Result<(), String>>,
}

pub struct HotkeyHandle {
    thread: u32,
    requests: Sender<Request>,
}

impl HotkeyHandle {
    pub fn spawn(on_press: Box<dyn Fn(i32) + Send>) -> Self {
        let (req_tx, req_rx) = channel::<Request>();
        let (ready_tx, ready_rx) = channel();
        std::thread::Builder::new()
            .name("sheen-hotkeys".into())
            .spawn(move || unsafe {
                let mut msg = MSG::default();
                // Create this thread's message queue before announcing it.
                let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
                let _ = ready_tx.send(GetCurrentThreadId());
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    match msg.message {
                        WM_HOTKEY => on_press(msg.wParam.0 as i32),
                        WM_APP => {
                            while let Ok(req) = req_rx.try_recv() {
                                let _ = UnregisterHotKey(None, req.id);
                                let result = match req.accelerator {
                                    None => Ok(()),
                                    Some(acc) => RegisterHotKey(
                                        None,
                                        req.id,
                                        HOT_KEY_MODIFIERS(acc.modifiers()) | MOD_NOREPEAT,
                                        acc.vk as u32,
                                    )
                                    .map_err(|_| format!("{acc} is already used by another app")),
                                };
                                let _ = req.reply.send(result);
                            }
                        }
                        _ => {}
                    }
                }
            })
            .expect("spawn hotkey thread");
        let thread = ready_rx.recv_timeout(Duration::from_secs(2)).unwrap_or(0);
        Self { thread, requests: req_tx }
    }

    pub fn set(&self, id: i32, accelerator: Option<Accelerator>) -> Result<(), String> {
        let (reply, result) = channel();
        self.requests.send(Request { id, accelerator, reply }).map_err(|e| e.to_string())?;
        unsafe { PostThreadMessageW(self.thread, WM_APP, WPARAM(0), LPARAM(0)) }
            .map_err(|e| e.to_string())?;
        result.recv_timeout(Duration::from_secs(2)).map_err(|e| e.to_string())?
    }
}
