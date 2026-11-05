//! Play order. Holds tracks, not library ids, so a rescan never invalidates
//! what is currently playing.

use crate::player::Repeat;
use crate::Track;

#[derive(Debug, Default, Clone)]
pub struct Queue {
    tracks: Vec<Track>,
    /// Play order: indexes into `tracks`. Identity unless shuffled.
    order: Vec<usize>,
    /// Position in `order`.
    cursor: usize,
    shuffle: bool,
}

impl Queue {
    pub fn new(tracks: Vec<Track>, start: usize, shuffle: bool) -> Self {
        let mut q = Queue {
            order: (0..tracks.len()).collect(),
            tracks,
            cursor: 0,
            shuffle: false,
        };
        q.cursor = start.min(q.tracks.len().saturating_sub(1));
        q.set_shuffle(shuffle);
        q
    }

    pub fn current(&self) -> Option<&Track> {
        self.order.get(self.cursor).map(|&i| &self.tracks[i])
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Toggle shuffle, keeping the current track current.
    pub fn set_shuffle(&mut self, on: bool) {
        if on == self.shuffle || self.tracks.is_empty() {
            self.shuffle = on;
            return;
        }
        let current = self.order[self.cursor];
        self.order = (0..self.tracks.len()).collect();
        if on {
            // Current track first, the rest shuffled after it.
            self.order.swap(0, current);
            fastrand::shuffle(&mut self.order[1..]);
            self.cursor = 0;
        } else {
            self.cursor = current;
        }
        self.shuffle = on;
    }

    /// Advance after a track finished (`auto`) or the user pressed next.
    /// Returns `false` when playback should stop.
    pub fn advance(&mut self, repeat: Repeat, auto: bool) -> bool {
        if self.tracks.is_empty() {
            return false;
        }
        if auto && repeat == Repeat::One {
            return true;
        }
        if self.cursor + 1 < self.order.len() {
            self.cursor += 1;
            true
        } else if repeat != Repeat::Off || !auto {
            // Wrap around on repeat-all, and when the user presses next on
            // the last track (instead of doing nothing).
            self.cursor = 0;
            true
        } else {
            false
        }
    }

    pub fn back(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        } else if !self.order.is_empty() {
            self.cursor = self.order.len() - 1;
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
                duration_ms: 1000,
                mtime: 0,
            })
            .collect()
    }

    fn id(q: &Queue) -> u32 {
        q.current().unwrap().id
    }

    #[test]
    fn plays_in_order_and_stops_at_end() {
        let mut q = Queue::new(tracks(3), 1, false);
        assert_eq!(id(&q), 1);
        assert!(q.advance(Repeat::Off, true));
        assert_eq!(id(&q), 2);
        assert!(!q.advance(Repeat::Off, true));
        // A manual "next" on the last track wraps to the first.
        assert!(q.advance(Repeat::Off, false));
        assert_eq!(id(&q), 0);
    }

    #[test]
    fn repeat_all_wraps_and_repeat_one_stays() {
        let mut q = Queue::new(tracks(2), 1, false);
        assert!(q.advance(Repeat::All, true));
        assert_eq!(id(&q), 0);
        assert!(q.advance(Repeat::One, true));
        assert_eq!(id(&q), 0);
        // A manual "next" skips even on repeat-one.
        assert!(q.advance(Repeat::One, false));
        assert_eq!(id(&q), 1);
    }

    #[test]
    fn back_wraps_to_last() {
        let mut q = Queue::new(tracks(3), 0, false);
        q.back();
        assert_eq!(id(&q), 2);
    }

    #[test]
    fn shuffle_keeps_current_and_visits_everything() {
        let mut q = Queue::new(tracks(50), 7, false);
        q.set_shuffle(true);
        assert_eq!(id(&q), 7);

        let mut seen = vec![id(&q)];
        while q.advance(Repeat::Off, true) {
            seen.push(id(&q));
        }
        seen.sort();
        assert_eq!(seen, (0..50).collect::<Vec<_>>());

        q.set_shuffle(false);
        let current = id(&q);
        q.advance(Repeat::Off, true);
        assert_eq!(id(&q), current + 1);
    }

    #[test]
    fn empty_queue_is_inert() {
        let mut q = Queue::new(Vec::new(), 3, true);
        assert!(q.current().is_none());
        assert!(!q.advance(Repeat::All, false));
        q.back();
    }
}
