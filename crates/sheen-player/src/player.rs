use crate::engine::{new_engine, Engine};
use crate::queue::Queue;
use crate::Track;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often to check for end-of-track while playing.
const POLL: Duration = Duration::from_millis(250);
/// Release the audio device after being paused this long.
const IDLE_RELEASE: Duration = Duration::from_secs(30);
/// "Previous" restarts the track instead when we are past this point.
const RESTART_THRESHOLD: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PlayerCommand {
    /// Replace the queue and start playing `tracks[start]`.
    PlayQueue {
        tracks: Vec<Track>,
        start: usize,
    },
    TogglePlay,
    Play,
    Pause,
    Next,
    Previous,
    Seek {
        position_ms: u64,
    },
    SetVolume {
        volume: f32,
    },
    SetShuffle {
        on: bool,
    },
    SetRepeat {
        repeat: Repeat,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub track: Option<Track>,
    pub playing: bool,
    pub position_ms: u64,
    /// Unix ms when `position_ms` was sampled (the UI interpolates).
    pub sampled_at_ms: u64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub queue_len: usize,
}

/// Handle to the player thread.
#[derive(Clone)]
pub struct Player(Sender<PlayerCommand>);

impl Player {
    /// Start the player thread. `on_change` runs on that thread whenever the
    /// state changes (track, play/pause, seek, settings). It is not called
    /// for the playhead simply moving forward.
    pub fn spawn(volume: f32, on_change: impl Fn(&PlayerState) + Send + 'static) -> Self {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("sheen-player".into())
            .spawn(move || {
                let mut core = Core::new(new_engine(), volume);
                loop {
                    let timeout = core.timeout();
                    let changed = match rx.recv_timeout(timeout) {
                        Ok(cmd) => {
                            core.handle(cmd);
                            true
                        }
                        Err(RecvTimeoutError::Timeout) => core.tick(),
                        Err(RecvTimeoutError::Disconnected) => return,
                    };
                    if changed {
                        on_change(&core.state());
                    }
                }
            })
            .expect("spawn player");
        Player(tx)
    }

    pub fn send(&self, command: PlayerCommand) {
        let _ = self.0.send(command);
    }
}

struct Core {
    engine: Box<dyn Engine>,
    queue: Queue,
    repeat: Repeat,
    volume: f32,
    playing: bool,
    /// The current track is loaded into the engine.
    loaded: bool,
    /// Position to resume from once the engine has been released.
    resume_at: Duration,
    paused_since: Option<Instant>,
}

impl Core {
    fn new(mut engine: Box<dyn Engine>, volume: f32) -> Self {
        engine.set_volume(volume);
        Core {
            engine,
            queue: Queue::default(),
            repeat: Repeat::Off,
            volume,
            playing: false,
            loaded: false,
            resume_at: Duration::ZERO,
            paused_since: None,
        }
    }

    fn timeout(&self) -> Duration {
        if self.playing {
            POLL
        } else if let Some(since) = self.paused_since.filter(|_| self.loaded) {
            IDLE_RELEASE.saturating_sub(since.elapsed())
        } else {
            Duration::from_secs(3600)
        }
    }

    fn handle(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::PlayQueue { tracks, start } => {
                self.queue = Queue::new(tracks, start, self.queue.shuffle());
                self.start_current(Duration::ZERO);
            }
            PlayerCommand::TogglePlay if self.playing => self.pause(),
            PlayerCommand::TogglePlay | PlayerCommand::Play => self.play(),
            PlayerCommand::Pause => self.pause(),
            PlayerCommand::Next => {
                if self.queue.advance(self.repeat, false) {
                    self.start_current(Duration::ZERO);
                }
            }
            PlayerCommand::Previous => {
                if self.position() > RESTART_THRESHOLD {
                    self.seek(Duration::ZERO);
                } else {
                    self.queue.back();
                    self.start_current(Duration::ZERO);
                }
            }
            PlayerCommand::Seek { position_ms } => self.seek(Duration::from_millis(position_ms)),
            PlayerCommand::SetVolume { volume } => {
                self.volume = volume.clamp(0.0, 1.0);
                self.engine.set_volume(self.volume);
            }
            PlayerCommand::SetShuffle { on } => self.queue.set_shuffle(on),
            PlayerCommand::SetRepeat { repeat } => self.repeat = repeat,
        }
    }

    /// Returns `true` if the state changed.
    fn tick(&mut self) -> bool {
        if self.playing && self.engine.finished() {
            if self.queue.advance(self.repeat, true) {
                self.start_current(Duration::ZERO);
            } else {
                self.stop();
            }
            return true;
        }
        if !self.playing
            && self.loaded
            && self
                .paused_since
                .is_some_and(|t| t.elapsed() >= IDLE_RELEASE)
        {
            self.resume_at = self.engine.position();
            self.engine.release();
            self.loaded = false;
            log::debug!("audio device released after idle");
        }
        false
    }

    /// Load the current track, skipping unplayable files (at most one lap).
    fn start_current(&mut self, start: Duration) {
        for _ in 0..self.queue.len().max(1) {
            let Some(track) = self.queue.current() else {
                break;
            };
            match self.engine.load(&track.path, start) {
                Ok(()) => {
                    self.loaded = true;
                    self.playing = true;
                    self.paused_since = None;
                    return;
                }
                Err(e) => {
                    log::warn!("cannot play {}: {e}", track.path.display());
                    if !self.queue.advance(Repeat::All, false) {
                        break;
                    }
                }
            }
        }
        self.stop();
    }

    fn play(&mut self) {
        if self.playing || self.queue.is_empty() {
            return;
        }
        if self.loaded {
            self.engine.play();
            self.playing = true;
            self.paused_since = None;
        } else {
            self.start_current(self.resume_at);
        }
    }

    fn pause(&mut self) {
        if self.playing {
            self.engine.pause();
            self.playing = false;
            self.paused_since = Some(Instant::now());
        }
    }

    fn stop(&mut self) {
        self.engine.release();
        self.playing = false;
        self.loaded = false;
        self.resume_at = Duration::ZERO;
        self.paused_since = None;
    }

    fn seek(&mut self, pos: Duration) {
        if self.loaded {
            if let Err(e) = self.engine.seek(pos) {
                log::warn!("seek failed: {e}");
            }
        } else {
            self.resume_at = pos;
        }
    }

    fn position(&self) -> Duration {
        if self.loaded {
            self.engine.position()
        } else {
            self.resume_at
        }
    }

    fn state(&self) -> PlayerState {
        PlayerState {
            track: self.queue.current().cloned(),
            playing: self.playing,
            position_ms: self.position().as_millis() as u64,
            sampled_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
            volume: self.volume,
            shuffle: self.queue.shuffle(),
            repeat: self.repeat,
            queue_len: self.queue.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracks(n: u32) -> Vec<Track> {
        (0..n)
            .map(|id| Track {
                id,
                path: format!("{id}.mp3").into(),
                title: id.to_string(),
                artist: String::new(),
                album: String::new(),
                duration_ms: 10_000,
                mtime: 0,
            })
            .collect()
    }

    fn core() -> Core {
        Core::new(new_engine(), 0.8)
    }

    #[test]
    fn play_queue_starts_requested_track() {
        let mut c = core();
        c.handle(PlayerCommand::PlayQueue {
            tracks: tracks(3),
            start: 2,
        });
        let s = c.state();
        assert!(s.playing);
        assert_eq!(s.track.unwrap().id, 2);
        assert_eq!(s.queue_len, 3);
    }

    #[test]
    fn toggle_pauses_and_resumes() {
        let mut c = core();
        c.handle(PlayerCommand::PlayQueue {
            tracks: tracks(1),
            start: 0,
        });
        c.handle(PlayerCommand::TogglePlay);
        assert!(!c.state().playing);
        assert!(c.timeout() <= IDLE_RELEASE);
        c.handle(PlayerCommand::TogglePlay);
        assert!(c.state().playing);
    }

    #[test]
    fn previous_restarts_when_past_threshold() {
        let mut c = core();
        c.handle(PlayerCommand::PlayQueue {
            tracks: tracks(3),
            start: 1,
        });
        c.handle(PlayerCommand::Seek { position_ms: 5_000 });
        c.handle(PlayerCommand::Previous);
        let s = c.state();
        assert_eq!(s.track.unwrap().id, 1);
        assert!(s.position_ms < 1_000);

        c.handle(PlayerCommand::Previous);
        assert_eq!(c.state().track.unwrap().id, 0);
    }

    #[test]
    fn seek_while_released_is_kept_for_resume() {
        let mut c = core();
        c.handle(PlayerCommand::PlayQueue {
            tracks: tracks(1),
            start: 0,
        });
        c.handle(PlayerCommand::Pause);
        c.stop();
        c.handle(PlayerCommand::Seek { position_ms: 4_000 });
        assert_eq!(c.state().position_ms, 4_000);
        c.handle(PlayerCommand::Play);
        assert!(c.state().position_ms >= 4_000);
    }

    #[test]
    fn volume_is_clamped() {
        let mut c = core();
        c.handle(PlayerCommand::SetVolume { volume: 3.0 });
        assert_eq!(c.state().volume, 1.0);
    }

    #[test]
    fn commands_deserialize_from_ui_json() {
        let cmd: PlayerCommand =
            serde_json::from_str(r#"{"type":"seek","positionMs":1200}"#).unwrap();
        assert!(matches!(cmd, PlayerCommand::Seek { position_ms: 1200 }));
        let cmd: PlayerCommand =
            serde_json::from_str(r#"{"type":"setRepeat","repeat":"all"}"#).unwrap();
        assert!(matches!(
            cmd,
            PlayerCommand::SetRepeat {
                repeat: Repeat::All
            }
        ));
    }
}
