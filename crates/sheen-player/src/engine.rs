//! Audio output. rodio on Windows; a silent clock elsewhere (tests, CI).

use std::path::Path;
use std::time::Duration;

pub(crate) trait Engine {
    /// Open the device if needed, start `path` at `start`, and play.
    fn load(&mut self, path: &Path, start: Duration) -> Result<(), String>;
    fn play(&mut self);
    fn pause(&mut self);
    fn seek(&mut self, pos: Duration) -> Result<(), String>;
    fn position(&self) -> Duration;
    /// The loaded track has played to the end.
    fn finished(&self) -> bool;
    fn set_volume(&mut self, volume: f32);
    /// Close the audio device. `load` reopens it.
    fn release(&mut self);
}

pub(crate) fn new_engine() -> Box<dyn Engine> {
    #[cfg(windows)]
    {
        Box::new(rodio_engine::RodioEngine::default())
    }
    #[cfg(not(windows))]
    {
        Box::new(silent::SilentEngine::default())
    }
}

#[cfg(windows)]
mod rodio_engine {
    use super::*;
    use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

    #[derive(Default)]
    pub struct RodioEngine {
        output: Option<(MixerDeviceSink, Player)>,
        volume: Option<f32>,
    }

    impl RodioEngine {
        fn player(&self) -> Option<&Player> {
            self.output.as_ref().map(|(_, p)| p)
        }
    }

    impl Engine for RodioEngine {
        fn load(&mut self, path: &Path, start: Duration) -> Result<(), String> {
            if self.output.is_none() {
                let mut sink = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
                sink.log_on_drop(false);
                let player = Player::connect_new(sink.mixer());
                if let Some(v) = self.volume {
                    player.set_volume(v);
                }
                self.output = Some((sink, player));
            }
            let player = self.player().expect("output just opened");
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let decoder = Decoder::try_from(file).map_err(|e| e.to_string())?;
            player.clear();
            player.append(decoder);
            if !start.is_zero() {
                let _ = player.try_seek(start);
            }
            player.play();
            Ok(())
        }

        fn play(&mut self) {
            if let Some(p) = self.player() {
                p.play();
            }
        }

        fn pause(&mut self) {
            if let Some(p) = self.player() {
                p.pause();
            }
        }

        fn seek(&mut self, pos: Duration) -> Result<(), String> {
            match self.player() {
                Some(p) => p.try_seek(pos).map_err(|e| e.to_string()),
                None => Ok(()),
            }
        }

        fn position(&self) -> Duration {
            self.player().map_or(Duration::ZERO, |p| p.get_pos())
        }

        fn finished(&self) -> bool {
            self.player().is_some_and(|p| p.empty())
        }

        fn set_volume(&mut self, volume: f32) {
            self.volume = Some(volume);
            if let Some(p) = self.player() {
                p.set_volume(volume);
            }
        }

        fn release(&mut self) {
            self.output = None;
        }
    }
}

#[cfg(not(windows))]
mod silent {
    use super::*;
    use std::time::Instant;

    /// Keeps time like a real output so the player logic behaves the same.
    #[derive(Default)]
    pub struct SilentEngine {
        base: Duration,
        started: Option<Instant>,
    }

    impl Engine for SilentEngine {
        fn load(&mut self, _path: &Path, start: Duration) -> Result<(), String> {
            self.base = start;
            self.started = Some(Instant::now());
            Ok(())
        }
        fn play(&mut self) {
            self.started.get_or_insert_with(Instant::now);
        }
        fn pause(&mut self) {
            self.base = self.position();
            self.started = None;
        }
        fn seek(&mut self, pos: Duration) -> Result<(), String> {
            self.base = pos;
            if self.started.is_some() {
                self.started = Some(Instant::now());
            }
            Ok(())
        }
        fn position(&self) -> Duration {
            self.base + self.started.map_or(Duration::ZERO, |s| s.elapsed())
        }
        fn finished(&self) -> bool {
            false
        }
        fn set_volume(&mut self, _volume: f32) {}
        fn release(&mut self) {
            self.started = None;
        }
    }
}
