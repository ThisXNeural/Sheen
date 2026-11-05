//! Inert stand-ins used on non-Windows targets.

use crate::Signal;
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
