mod audio;
mod clipboard;
mod drag;
mod fullscreen;
mod hotkey;
mod media;
mod now_playing;
mod shell;
mod transfers;
mod window;

pub use audio::AudioHandle;
pub use clipboard::spawn_clipboard_listener;
pub use drag::{drag_files_out, file_drag_active};
pub use fullscreen::spawn_fullscreen_watcher;
pub use hotkey::HotkeyHandle;
pub use media::MediaHandle;
pub use now_playing::NowPlayingHandle;
pub use shell::{open, reveal};
pub use transfers::spawn_transfer_watcher;
pub use window::apply_island_window_style;

use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

/// Join the process-wide MTA on the current (service) thread.
fn init_com() {
    // S_FALSE ("already initialised") is fine; a mode clash is logged only.
    if let Err(e) = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok() {
        log::warn!("CoInitializeEx failed: {e}");
    }
}
