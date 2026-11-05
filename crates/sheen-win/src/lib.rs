//! Windows platform services for Sheen.
//!
//! Every service runs on its own thread, owns its (non-`Send`) COM objects, and
//! reports what it sees as [`Signal`]s over a plain `std::sync::mpsc` channel.
//! Nothing here decides *what* the island shows — that is `sheen-core`'s job.
//!
//! On non-Windows targets the services are inert stubs so the workspace (and
//! `sheen-core`'s tests) still build everywhere.

use sheen_core::{Accelerator, MediaCommand, SheenEvent};
use std::path::{Path, PathBuf};
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

/// A transport button pressed in Windows' media controls (media keys, flyout,
/// or the island acting on the built-in player's session).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaButton {
    Play,
    Pause,
    Next,
    Previous,
    Seek(u64),
}

/// What the built-in player is doing, as published to Windows.
#[derive(Debug, Clone, Default)]
pub struct NowPlaying {
    pub track: Option<NowPlayingTrack>,
    pub playing: bool,
    pub position_ms: u64,
}

#[derive(Debug, Clone)]
pub struct NowPlayingTrack {
    /// Changes whenever title/artist/cover change; metadata is only re-sent then.
    pub key: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    /// Encoded image bytes (JPEG/PNG) of the cover art.
    pub cover: Option<Vec<u8>>,
}

/// Registers the built-in player with Windows' media controls.
pub struct NowPlayingPublisher(imp::NowPlayingHandle);

impl NowPlayingPublisher {
    /// `hwnd` is any top-level window of this process (raw handle as integer).
    pub fn start(hwnd: isize, on_button: impl Fn(MediaButton) + Send + Sync + 'static) -> Self {
        Self(imp::NowPlayingHandle::spawn(hwnd, Box::new(on_button)))
    }

    pub fn update(&self, now_playing: NowPlaying) {
        self.0.update(now_playing);
    }
}

/// Handles to the running services.
pub struct Services {
    media: imp::MediaHandle,
    audio: imp::AudioHandle,
}

impl Services {
    /// Start media (SMTC), audio (Core Audio), full-screen, transfer and
    /// clipboard watchers.
    pub fn start(tx: Sender<Signal>) -> Self {
        imp::spawn_fullscreen_watcher(tx.clone());
        imp::spawn_transfer_watcher(tx.clone());
        imp::spawn_clipboard_listener(tx.clone());
        Self { media: imp::MediaHandle::spawn(tx.clone()), audio: imp::AudioHandle::spawn(tx) }
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

/// What a drop target did with files dragged out of the shelf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropOutcome {
    Cancelled,
    Copied,
    Moved,
    Linked,
}

/// A file drag is in progress anywhere on the desktop.
pub fn file_drag_active() -> bool {
    imp::file_drag_active()
}

/// A mouse button is held somewhere on the desktop.
pub fn mouse_button_down() -> bool {
    imp::mouse_button_down()
}

/// Drag files out to another app. Call on the UI thread while the mouse
/// button is held; returns when the drop is done.
pub fn drag_files_out(paths: &[PathBuf]) -> DropOutcome {
    imp::drag_files_out(paths).unwrap_or_else(|e| {
        log::warn!("drag out failed: {e:?}");
        DropOutcome::Cancelled
    })
}

/// Show a file selected in Explorer.
pub fn reveal_in_explorer(path: &Path) {
    imp::reveal(path)
}

/// Open a file or folder with its default app.
pub fn open_path(path: &Path) {
    imp::open(path)
}

/// System-wide keyboard shortcuts.
pub struct Hotkeys(imp::HotkeyHandle);

impl Hotkeys {
    /// `on_press` receives the id the shortcut was registered with.
    pub fn start(on_press: impl Fn(i32) + Send + 'static) -> Self {
        Self(imp::HotkeyHandle::spawn(Box::new(on_press)))
    }

    /// Register (or with `None`, remove) the shortcut for `id`. Fails if
    /// another app already owns the combination.
    pub fn set(&self, id: i32, accelerator: Option<Accelerator>) -> Result<(), String> {
        self.0.set(id, accelerator)
    }
}
