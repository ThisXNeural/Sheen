//! Sheen's built-in music player.
//!
//! * [`library`]: scans a folder for audio files and reads their tags, using an
//!   on-disk cache so unchanged files are never parsed twice.
//! * [`queue`]: play order, shuffle and repeat. Pure logic, unit-tested.
//! * [`Player`]: a single thread that owns the audio device and decoder.
//!
//! The player opens the audio device only while something is playing and
//! releases it after a short idle period, so a paused Sheen costs no audio
//! thread wake-ups and does not keep the sound card awake.

pub mod library;
pub mod queue;

mod engine;
mod player;

pub use library::{Library, Track};
pub use player::{Player, PlayerCommand, PlayerState, Repeat};
