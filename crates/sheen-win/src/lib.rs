//! Windows platform services for Sheen.
//!
//! Every service runs on its own thread, owns its (non-`Send`) COM objects, and
//! reports what it sees as [`Signal`]s over a plain `std::sync::mpsc` channel.
//! Nothing here decides *what* the island shows — that is `sheen-core`'s job.
//!
//! On non-Windows targets the services are inert stubs so the workspace (and
//! `sheen-core`'s tests) still build everywhere.

use sheen_core::{MediaCommand, SheenEvent};
use std::sync::mpsc::Sender;

/// Output of the platform layer.
#[derive(Debug, Clone)]
pub enum Signal {
    /// Something the island reducer must know about.
    Event(SheenEvent),
    /// New album art as a `data:` URL (or `None` when there is none). Kept out
    /// of [`SheenEvent`] because the reducer has no use for the bytes.
    Artwork(Option<String>),
}

#[cfg(windows)]
mod win;
#[cfg(windows)]
use win as imp;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use stub as imp;

/// Handles to the running services.
pub struct Services {
    media: imp::MediaHandle,
    audio: imp::AudioHandle,
}

impl Services {
    /// Start media (SMTC), audio (Core Audio) and full-screen watchers.
    pub fn start(tx: Sender<Signal>) -> Self {
        imp::spawn_fullscreen_watcher(tx.clone());
        Self {
            media: imp::MediaHandle::spawn(tx.clone()),
            audio: imp::AudioHandle::spawn(tx),
        }
    }

    pub fn media_command(&self, command: MediaCommand) {
        self.media.send(command);
    }

    /// Nudge the default output device's master volume by `delta` (-1.0..=1.0).
    pub fn adjust_volume(&self, delta: f32) {
        self.audio.adjust(delta);
    }
}

/// Give the island window its system-utility behaviour:
/// never steal focus, stay out of Alt-Tab/taskbar, stay on top.
///
/// `hwnd` is the raw window handle (`HWND.0 as isize`), passed as an integer so
/// this crate does not have to share a `windows` crate version with Tauri.
pub fn apply_island_window_style(hwnd: isize) -> anyhow::Result<()> {
    imp::apply_island_window_style(hwnd)
}
