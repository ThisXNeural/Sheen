//! Inert stand-ins used on non-Windows targets.

use crate::{DropOutcome, MediaButton, NowPlaying, Signal};
use sheen_core::Accelerator;
use sheen_core::MediaCommand;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

pub struct MediaHandle;
impl MediaHandle {
    pub fn spawn(_tx: Sender<Signal>) -> Self {
        log::warn!("media service is only available on Windows");
        Self
    }
    pub fn send(&self, _command: MediaCommand) {}
}

pub struct AudioHandle;
impl AudioHandle {
    pub fn spawn(_tx: Sender<Signal>) -> Self {
        Self
    }
    pub fn adjust(&self, _delta: f32) {}
}

pub fn spawn_fullscreen_watcher(_tx: Sender<Signal>) {}

pub fn apply_island_window_style(_hwnd: isize) -> anyhow::Result<()> {
    Ok(())
}

pub struct NowPlayingHandle;
impl NowPlayingHandle {
    pub fn spawn(_hwnd: isize, _on_button: Box<dyn Fn(MediaButton) + Send + Sync>) -> Self {
        Self
    }
    pub fn update(&self, _now_playing: NowPlaying) {}
}

pub fn spawn_transfer_watcher(_tx: Sender<Signal>) {}
pub fn spawn_clipboard_listener(_tx: Sender<Signal>) {}

pub fn file_drag_active() -> bool {
    false
}

pub fn mouse_button_down() -> bool {
    false
}

pub fn drag_files_out(_paths: &[PathBuf]) -> Result<DropOutcome, String> {
    Ok(DropOutcome::Cancelled)
}

pub fn reveal(_path: &Path) {}
pub fn open(_path: &Path) {}

pub struct HotkeyHandle;
impl HotkeyHandle {
    pub fn spawn(_on_press: Box<dyn Fn(i32) + Send>) -> Self {
        Self
    }
    pub fn set(&self, _id: i32, _accelerator: Option<Accelerator>) -> Result<(), String> {
        Ok(())
    }
}
