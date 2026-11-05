//! Remaining-time estimation for transfers.
//!
//! Naive `remaining / current_speed` jumps around because throughput is noisy
//! (TCP bursts, disk flushes, other traffic). This estimator does it in
//! three layers:
//!
//! 1. **Rate filter.** A scalar Kalman filter tracks throughput. Process noise
//!    is relative to the current rate, so it adapts at the same speed whether
//!    a transfer runs at 50 KB/s or 500 MB/s. Measurement noise is also
//!    relative, which absorbs bursts without lagging real speed changes for
//!    long. The steady-state gain is about 0.2, so a real change in speed
//!    shows up within roughly 5 to 10 s.
//! 2. **Blend with the session average.** A quarter of the weight goes to the
//!    overall average rate since the transfer started, which anchors the
//!    estimate against short-term swings.
//! 3. **Display smoothing.** The shown ETA counts down in real time as long as
//!    a fresh estimate agrees with it (within 15%). When they disagree, it moves
//!    30% of the way toward the new estimate per sample, so it converges without
//!    ever jumping.
//!
//! A transfer that has made no progress for [`STALL_AFTER`] seconds is
//! reported as stalled, with no ETA, rather than counting up to infinity.

/// Seconds without progress before a transfer counts as stalled.
pub const STALL_AFTER: f64 = 4.0;
/// Minimum observation time before an ETA is offered.
const WARMUP: f64 = 1.5;
/// Samples closer together than this are merged into the next one.
const MIN_DT: f64 = 0.25;
/// Relative process noise (rate drift per √s).
const PROCESS_REL: f64 = 0.15;
/// Relative measurement noise.
const MEASURE_REL: f64 = 0.6;
/// Weight of the filtered rate against the session average.
const RECENT_WEIGHT: f64 = 0.75;
/// Tolerance within which the shown ETA simply keeps counting down.
const AGREE: f64 = 0.15;
/// Fraction of the gap closed per sample when estimates disagree.
const CONVERGE: f64 = 0.3;

#[derive(Debug, Clone, Default)]
pub struct EtaEstimator {
    first: Option<(f64, f64)>,
    last: Option<(f64, f64)>,
    last_progress: f64,
    rate: f64,
    var: f64,
    initialized: bool,
    shown: Option<(f64, f64)>,
}

impl EtaEstimator {
    /// Feed an observation: `done` units completed at time `t` (seconds).
    pub fn update(&mut self, t: f64, done: f64) {
        let Some((lt, ld)) = self.last else {
            self.first = Some((t, done));
            self.last = Some((t, done));
            self.last_progress = t;
            return;
        };
        let dt = t - lt;
        if dt < MIN_DT {
            return;
        }
        if done < ld {
            // The source restarted (e.g. a resumed download). Start over.
            *self = EtaEstimator::default();
            return self.update(t, done);
        }
        if done > ld {
            self.last_progress = t;
        }
        let z = (done - ld) / dt;
        if !self.initialized {
            self.rate = z;
            self.var = (MEASURE_REL * z).powi(2) + 1e-9;
            self.initialized = true;
        } else {
            self.var += (PROCESS_REL * self.rate).powi(2) * dt + 1e-12;
            let r = (MEASURE_REL * self.rate.max(z)).powi(2) + 1e-12;
            let gain = self.var / (self.var + r);
            self.rate += gain * (z - self.rate);
            self.var *= 1.0 - gain;
        }
        self.last = Some((t, done));
    }

    /// Smoothed throughput in units per second.
    pub fn rate(&self) -> Option<f64> {
        if !self.initialized {
            return None;
        }
        let (ft, fd) = self.first?;
        let (lt, ld) = self.last?;
        let span = lt - ft;
        let average = if span > 0.0 { (ld - fd) / span } else { self.rate };
        Some((RECENT_WEIGHT * self.rate + (1.0 - RECENT_WEIGHT) * average).max(0.0))
    }

    pub fn stalled(&self, t: f64) -> bool {
        self.last.is_some() && t - self.last_progress > STALL_AFTER
    }

    /// Seconds remaining for `remaining` units, smoothed for display.
    /// Call once per observation.
    pub fn eta(&mut self, t: f64, remaining: f64) -> Option<f64> {
        let span = match (self.first, self.last) {
            (Some((ft, _)), Some((lt, _))) => lt - ft,
            _ => 0.0,
        };
        let rate = self.rate().filter(|r| *r > 0.0);
        let (Some(rate), false, true) = (rate, self.stalled(t), span >= WARMUP) else {
            self.shown = None;
            return None;
        };
        let raw = remaining.max(0.0) / rate;
        let eta = match self.shown {
            None => raw,
            Some((st, se)) => {
                let predicted = (se - (t - st)).max(0.0);
                let deviation = (raw - predicted) / predicted.max(5.0);
                if deviation.abs() <= AGREE {
                    predicted
                } else {
                    predicted + CONVERGE * (raw - predicted)
                }
            }
        };
        self.shown = Some((t, eta));
        Some(eta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic noise in [-1, 1].
    fn noise(seed: &mut u64) -> f64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*seed >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
    }

    #[test]
    fn constant_rate_gives_exact_eta() {
        let mut e = EtaEstimator::default();
        let total = 100.0e6;
        let mut eta = None;
        for s in 0..=10 {
            let t = s as f64;
            e.update(t, t * 1.0e6);
            eta = e.eta(t, total - t * 1.0e6);
        }
        let eta = eta.unwrap();
        assert!((eta - 90.0).abs() < 90.0 * 0.03, "eta {eta}");
    }

    #[test]
    fn no_eta_during_warmup() {
        let mut e = EtaEstimator::default();
        e.update(0.0, 0.0);
        e.update(1.0, 10.0);
        assert_eq!(e.eta(1.0, 100.0), None);
        e.update(2.0, 20.0);
        assert!(e.eta(2.0, 80.0).is_some());
    }

    #[test]
    fn noisy_rate_counts_down_smoothly_and_lands_close() {
        let mut e = EtaEstimator::default();
        let mut seed = 7;
        let total = 300.0e6;
        let mut done = 0.0;
        let mut shown: Vec<f64> = Vec::new();
        for s in 0..=150 {
            let t = s as f64;
            if s > 0 {
                done += 2.0e6 * (1.0 + 0.4 * noise(&mut seed)); // 2 MB/s ±40%
            }
            e.update(t, done);
            if let Some(eta) = e.eta(t, total - done) {
                shown.push(eta);
            }
        }
        // Never jumps up by more than a little between one-second samples,
        // even though the instantaneous rate swings by ±40%.
        for w in shown.windows(2) {
            assert!(w[1] <= w[0] + 0.1 * w[0].max(10.0), "jumped {} -> {}", w[0], w[1]);
        }
        // Lands near the truth: (300 - ~300) MB left at ~2 MB/s.
        let truth = (total - done) / 2.0e6;
        let last = *shown.last().unwrap();
        assert!((last - truth).abs() < 8.0, "eta {last} vs {truth}");
    }

    #[test]
    fn stall_hides_eta_and_recovers() {
        let mut e = EtaEstimator::default();
        for s in 0..=5 {
            e.update(s as f64, s as f64 * 10.0);
            e.eta(s as f64, 1000.0);
        }
        for s in 6..=11 {
            e.update(s as f64, 50.0);
        }
        assert!(e.stalled(11.0));
        assert_eq!(e.eta(11.0, 950.0), None);

        for s in 12..=15 {
            e.update(s as f64, 50.0 + (s - 11) as f64 * 10.0);
        }
        assert!(!e.stalled(15.0));
        assert!(e.eta(15.0, 910.0).is_some());
    }

    #[test]
    fn adapts_to_a_speed_change() {
        let mut e = EtaEstimator::default();
        let total = 1.0e9;
        let mut done = 0.0;
        let mut eta = None;
        for s in 0..=60 {
            let t = s as f64;
            if s > 0 {
                done += if s <= 20 { 10.0e6 } else { 5.0e6 };
            }
            e.update(t, done);
            eta = e.eta(t, total - done);
        }
        let truth = (total - done) / 5.0e6;
        let eta = eta.unwrap();
        assert!((eta - truth).abs() / truth < 0.2, "eta {eta} vs {truth}");
    }

    #[test]
    fn restart_resets_the_estimate() {
        let mut e = EtaEstimator::default();
        e.update(0.0, 0.0);
        e.update(1.0, 500.0);
        e.update(2.0, 100.0); // went backwards
        assert_eq!(e.rate(), None);
    }
}
