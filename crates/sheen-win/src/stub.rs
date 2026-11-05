//! Inert stand-ins used on non-Windows targets.

use crate::{MediaButton, NowPlaying, Signal};
use sheen_core::MediaCommand;
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
