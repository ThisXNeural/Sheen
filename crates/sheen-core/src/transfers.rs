//! Turns raw provider samples into what the island shows: progress, speed,
//! a stable ETA, and a short "done" flash when a transfer completes.

use crate::eta::EtaEstimator;
use crate::event::{TransferKind, TransferSample, TransferUnit};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long a finished transfer stays on screen.
pub const DONE_FLASH: Duration = Duration::from_millis(2500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferState {
    Active,
    Stalled,
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: u64,
    pub kind: TransferKind,
    pub app: String,
    pub name: String,
    /// 0.0..=1.0 when the total is known.
    pub progress: Option<f32>,
    pub done_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<u32>,
    pub state: TransferState,
}

#[derive(Debug, Clone)]
struct Entry {
    sample: TransferSample,
    est: EtaEstimator,
    eta: Option<f64>,
    stalled: bool,
    finished_at: Option<Instant>,
    order: u64,
}

#[derive(Debug, Clone)]
pub struct TransferTracker {
    epoch: Instant,
    entries: HashMap<u64, Entry>,
    next_order: u64,
}

impl Default for TransferTracker {
    fn default() -> Self {
        Self { epoch: Instant::now(), entries: HashMap::new(), next_order: 0 }
    }
}

impl TransferTracker {
    pub fn update(&mut self, samples: Vec<TransferSample>, now: Instant) {
        let t = now.saturating_duration_since(self.epoch).as_secs_f64();
        let mut seen = Vec::with_capacity(samples.len());

        for sample in samples {
            seen.push(sample.id);
            let order = self.next_order;
            let entry = self.entries.entry(sample.id).or_insert_with(|| Entry {
                sample: sample.clone(),
                est: EtaEstimator::default(),
                eta: None,
                stalled: false,
                finished_at: None,
                order,
            });
            if entry.order == order {
                self.next_order += 1;
            }
            entry.est.update(t, sample.done as f64);
            entry.stalled = entry.est.stalled(t);
            entry.eta = sample
                .total
                .and_then(|total| entry.est.eta(t, total.saturating_sub(sample.done) as f64));
            entry.finished_at = None;
            entry.sample = sample;
        }

        // Transfers that vanished: flash "done" if they were (nearly)
        // complete, otherwise they were cancelled and simply go away.
        self.entries.retain(|id, e| {
            if seen.contains(id) || e.finished_at.is_some() {
                return true;
            }
            let complete = e
                .sample
                .total
                .is_some_and(|total| total > 0 && e.sample.done as f64 / total as f64 >= 0.95);
            if complete {
                e.finished_at = Some(now);
                e.sample.done = e.sample.total.unwrap_or(e.sample.done);
            }
            complete
        });
    }

    /// Drop expired "done" flashes. Returns `true` if anything changed.
    pub fn tick(&mut self, now: Instant) -> bool {
        let before = self.entries.len();
        self.entries.retain(|_, e| e.finished_at.is_none_or(|at| now < at + DONE_FLASH));
        self.entries.len() != before
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.entries.values().filter_map(|e| e.finished_at).map(|at| at + DONE_FLASH).min()
    }

    /// Transfers worth showing: those whose source is not in front of the
    /// user, plus fresh completions. Oldest first.
    pub fn visible(&self) -> Vec<Transfer> {
        let mut list: Vec<&Entry> = self
            .entries
            .values()
            .filter(|e| e.finished_at.is_some() || !e.sample.source_in_view)
            .collect();
        list.sort_by_key(|e| e.order);
        list.into_iter().map(present).collect()
    }
}

fn present(e: &Entry) -> Transfer {
    let s = &e.sample;
    let bytes = s.unit == TransferUnit::Bytes;
    let state = if e.finished_at.is_some() {
        TransferState::Done
    } else if e.stalled {
        TransferState::Stalled
    } else {
        TransferState::Active
    };
    Transfer {
        id: s.id,
        kind: s.kind,
        app: s.app.clone(),
        name: s.name.clone(),
        progress: s
            .total
            .filter(|t| *t > 0)
            .map(|t| (s.done as f64 / t as f64).clamp(0.0, 1.0) as f32),
        done_bytes: bytes.then_some(s.done),
        total_bytes: if bytes { s.total } else { None },
        speed_bps: if bytes && state == TransferState::Active { e.est.rate() } else { None },
        eta_secs: if state == TransferState::Active {
            e.eta.map(|s| s.round() as u32)
        } else {
            None
        },
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: u64, done: u64, total: u64, in_view: bool) -> TransferSample {
        TransferSample {
            id,
            kind: TransferKind::Download,
            app: "Chrome".into(),
            name: format!("file{id}.zip"),
            done,
            total: Some(total),
            unit: TransferUnit::Bytes,
            source_in_view: in_view,
        }
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn hides_transfers_whose_source_is_in_view() {
        let t0 = Instant::now();
        let mut tr = TransferTracker::default();
        tr.update(vec![sample(1, 10, 100, true), sample(2, 10, 100, false)], t0);
        let v = tr.visible();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, 2);
    }

    #[test]
    fn reports_progress_speed_and_eta() {
        let t0 = Instant::now();
        let mut tr = TransferTracker::default();
        for s in 0..=5 {
            tr.update(vec![sample(1, s * 1_000_000, 20_000_000, false)], t0 + secs(s));
        }
        let v = &tr.visible()[0];
        assert_eq!(v.progress, Some(0.25));
        assert!((v.speed_bps.unwrap() - 1.0e6).abs() < 1.0e5);
        assert_eq!(v.eta_secs, Some(15));
        assert_eq!(v.state, TransferState::Active);
    }

    #[test]
    fn completion_flashes_then_disappears() {
        let t0 = Instant::now();
        let mut tr = TransferTracker::default();
        tr.update(vec![sample(1, 99, 100, true)], t0);
        tr.update(vec![], t0 + secs(1));
        let v = tr.visible();
        assert_eq!(v[0].state, TransferState::Done);
        assert_eq!(v[0].progress, Some(1.0));
        assert_eq!(tr.next_deadline(), Some(t0 + secs(1) + DONE_FLASH));
        assert!(tr.tick(t0 + secs(4)));
        assert!(tr.visible().is_empty());
    }

    #[test]
    fn cancelled_transfers_vanish_silently() {
        let t0 = Instant::now();
        let mut tr = TransferTracker::default();
        tr.update(vec![sample(1, 10, 100, false)], t0);
        tr.update(vec![], t0 + secs(1));
        assert!(tr.visible().is_empty());
        assert_eq!(tr.next_deadline(), None);
    }

    #[test]
    fn keeps_arrival_order() {
        let t0 = Instant::now();
        let mut tr = TransferTracker::default();
        tr.update(vec![sample(9, 1, 10, false)], t0);
        tr.update(vec![sample(3, 1, 10, false), sample(9, 2, 10, false)], t0 + secs(1));
        let ids: Vec<u64> = tr.visible().iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![9, 3]);
    }
}
