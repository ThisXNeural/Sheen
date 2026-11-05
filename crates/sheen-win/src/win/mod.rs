mod audio;
mod fullscreen;
mod media;
mod window;

pub use audio::AudioHandle;
pub use fullscreen::spawn_fullscreen_watcher;
pub use media::MediaHandle;
pub use window::apply_island_window_style;

use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

/// Join the process-wide MTA on the current (service) thread.
fn init_com() {
    // S_FALSE ("already initialised") is fine; a mode clash is logged only.
    if let Err(e) = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok() {
        log::warn!("CoInitializeEx failed: {e}");
    }
}
