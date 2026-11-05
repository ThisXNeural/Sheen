//! Dynamic hit-testing for a transparent, oversized WebView2 window.
//!
//! The classic Win32 answer (`WM_NCHITTEST` → `HTTRANSPARENT`) does not work
//! here: WebView2 renders into child windows that receive input directly, and
//! `HTTRANSPARENT` only forwards to windows on the same thread anyway. Instead
//! we toggle `WS_EX_TRANSPARENT` (Tauri's `set_ignore_cursor_events`) based on
//! where the cursor is: the window is click-through everywhere except over the
//! capsule, which `sheen_core::HitRegion` describes.
//!
//! The cursor is polled at display rate only while it is near the island and
//! at a lazy rate otherwise, so the idle cost is negligible.

use sheen_core::{HitRegion, SheenEvent};
use sheen_win::Signal;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::WebviewWindow;

const FAST: Duration = Duration::from_millis(16);
const SLOW: Duration = Duration::from_millis(120);
/// Distance (logical px) from the capsule within which we poll fast.
const NEAR: f64 = 120.0;
const FRAME_REFRESH: Duration = Duration::from_secs(2);

pub fn spawn(window: WebviewWindow, region: Arc<Mutex<HitRegion>>, signals: Sender<Signal>) {
    std::thread::Builder::new()
        .name("sheen-hit-test".into())
        .spawn(move || {
            let mut inside = false;
            let mut frame = None;
            let mut refreshed = Instant::now();
            loop {
                let mut delay = SLOW;
                // The window does not move, but monitor/DPI changes can still
                // shift it; re-read its frame now and then instead of per tick.
                if frame.is_none() || refreshed.elapsed() > FRAME_REFRESH {
                    frame = Frame::read(&window);
                    refreshed = Instant::now();
                }
                let cursor = window.cursor_position().ok();
                if let (Some(f), Some(c)) = (frame, cursor) {
                    let (x, y) = f.to_canvas(c.x, c.y);
                    let rect = region.lock().unwrap().active(Instant::now());
                    let now_inside = rect.contains(x, y);
                    if now_inside != inside {
                        inside = now_inside;
                        let _ = window.set_ignore_cursor_events(!inside);
                        if signals
                            .send(Signal::Event(SheenEvent::Hover(inside)))
                            .is_err()
                        {
                            return;
                        }
                    }
                    let near = x > rect.x - NEAR
                        && x < rect.x + rect.width + NEAR
                        && y < rect.y + rect.height + NEAR;
                    if inside || near {
                        delay = FAST;
                    }
                }
                std::thread::sleep(delay);
            }
        })
        .expect("spawn hit-tester");
}

/// The window's physical origin and DPI scale.
#[derive(Clone, Copy)]
struct Frame {
    x: f64,
    y: f64,
    scale: f64,
}

impl Frame {
    fn read(window: &WebviewWindow) -> Option<Self> {
        let origin = window.outer_position().ok()?;
        let scale = window.scale_factor().ok()?;
        Some(Self {
            x: origin.x as f64,
            y: origin.y as f64,
            scale,
        })
    }

    /// Physical screen coordinates → the canvas' logical coordinates.
    fn to_canvas(self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.x) / self.scale, (y - self.y) / self.scale)
    }
}
