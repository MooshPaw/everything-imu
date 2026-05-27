//! Rolling-window latency / jitter tracker for a single device pipeline.
//!
//! Tracks two distributions over a fixed sample window (default 256 batches):
//!
//! 1. **inter-batch interval** — `now - prev_now` on every `ChannelInfo::ImuSamples`
//!    arrival. Reflects how steadily the driver feeds the pipeline.
//! 2. **send latency** — time spent inside `SlimeClient::send_*` on the
//!    bridge thread. Reflects how long UDP emission blocks the pipeline.
//!
//! Percentiles are computed on demand from a copy of the window — cheap
//! enough at N=256 and called at most 1 Hz by the snapshot emitter.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const WINDOW: usize = 256;

/// Number of [`LatencySnapshot`]s kept by [`LatencyHistory`].
///
/// 120 entries × 1 s sampling = two minutes of history, which fits in a
/// sparkline tile without scrolling and tracks roughly the timescale a user
/// will notice "jitter went up after I switched USB ports". Tunable per
/// device if needed; the cost is one `LatencySnapshot` per entry (~32 B).
pub const DEFAULT_HISTORY_LEN: usize = 120;

#[derive(Debug, Clone, Copy, Default)]
pub struct LatencySnapshot {
    /// Inter-batch interval percentiles (microseconds).
    pub interval_us_p50: f32,
    pub interval_us_p95: f32,
    pub interval_us_p99: f32,
    /// Jitter = stddev of inter-batch intervals (microseconds).
    pub jitter_us: f32,
    /// UDP send-call latency percentiles (microseconds).
    pub send_us_p50: f32,
    pub send_us_p95: f32,
    /// Estimated dropped batches inside the window: any interval
    /// > 2 × median is counted as one miss.
    pub dropped_estimate: u32,
    /// Number of intervals currently in the window.
    pub samples_window: u32,
}

pub struct LatencyTracker {
    intervals_us: VecDeque<u32>,
    sends_us: VecDeque<u32>,
    prev_arrival: Option<Instant>,
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self {
            intervals_us: VecDeque::with_capacity(WINDOW),
            sends_us: VecDeque::with_capacity(WINDOW),
            prev_arrival: None,
        }
    }

    /// Record a batch arrival. Returns the inter-batch interval if known.
    pub fn record_arrival(&mut self, now: Instant) -> Option<Duration> {
        let prev = self.prev_arrival.replace(now)?;
        let dt = now.saturating_duration_since(prev);
        push_capped(
            &mut self.intervals_us,
            dt.as_micros().min(u32::MAX as u128) as u32,
        );
        Some(dt)
    }

    /// Record one UDP send-call duration.
    pub fn record_send(&mut self, dur: Duration) {
        push_capped(
            &mut self.sends_us,
            dur.as_micros().min(u32::MAX as u128) as u32,
        );
    }

    pub fn snapshot(&self) -> LatencySnapshot {
        let (p50, p95, p99, jitter, dropped, n) = if self.intervals_us.is_empty() {
            (0, 0, 0, 0.0, 0, 0)
        } else {
            let mut iv: Vec<u32> = self.intervals_us.iter().copied().collect();
            iv.sort_unstable();
            let p50 = percentile(&iv, 0.50);
            let drops = iv.iter().filter(|&&v| v > p50.saturating_mul(2)).count() as u32;
            (
                p50,
                percentile(&iv, 0.95),
                percentile(&iv, 0.99),
                stddev_us(&iv),
                drops,
                iv.len() as u32,
            )
        };
        let (s50, s95) = if self.sends_us.is_empty() {
            (0, 0)
        } else {
            let mut sv: Vec<u32> = self.sends_us.iter().copied().collect();
            sv.sort_unstable();
            (percentile(&sv, 0.50), percentile(&sv, 0.95))
        };
        LatencySnapshot {
            interval_us_p50: p50 as f32,
            interval_us_p95: p95 as f32,
            interval_us_p99: p99 as f32,
            jitter_us: jitter,
            send_us_p50: s50 as f32,
            send_us_p95: s95 as f32,
            dropped_estimate: dropped,
            samples_window: n,
        }
    }
}

fn push_capped(q: &mut VecDeque<u32>, v: u32) {
    if q.len() == WINDOW {
        q.pop_front();
    }
    q.push_back(v);
}

fn percentile(sorted: &[u32], p: f32) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f32 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn stddev_us(values: &[u32]) -> f32 {
    let n = values.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mean = values.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = values
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / (n - 1.0);
    var.sqrt() as f32
}

/// Bounded-capacity ring of past [`LatencySnapshot`]s suitable for rendering
/// a sparkline chart in the UI without growing memory unboundedly.
///
/// Append cadence is owned by the caller (typically the pipeline pushes one
/// snapshot per second). The ring evicts the oldest entry once full so the
/// chart always shows the most recent `cap` seconds of pipeline health.
#[derive(Debug, Clone)]
pub struct LatencyHistory {
    samples: VecDeque<LatencySnapshot>,
    cap: usize,
}

impl LatencyHistory {
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            samples: VecDeque::with_capacity(cap.min(1024)),
            cap,
        }
    }

    /// Append a snapshot, evicting the oldest entry if the buffer is full.
    pub fn push(&mut self, snap: LatencySnapshot) {
        if self.samples.len() == self.cap {
            self.samples.pop_front();
        }
        self.samples.push_back(snap);
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Snapshot the ring as a vector for serialisation / IPC. Oldest first.
    /// Cheap — `LatencySnapshot` is `Copy`-able 32-byte struct.
    pub fn snapshot(&self) -> Vec<LatencySnapshot> {
        self.samples.iter().copied().collect()
    }

    /// Most recent entry, if any.
    pub fn latest(&self) -> Option<LatencySnapshot> {
        self.samples.back().copied()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

impl Default for LatencyHistory {
    fn default() -> Self {
        Self::new(DEFAULT_HISTORY_LEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tracker_yields_zero_snapshot() {
        let snap = LatencyTracker::new().snapshot();
        assert_eq!(snap.samples_window, 0);
        assert_eq!(snap.interval_us_p50, 0.0);
    }

    #[test]
    fn percentiles_track_synthetic_intervals() {
        let mut t = LatencyTracker::new();
        let mut now = Instant::now();
        // 100 batches at exactly 1000us interval
        for _ in 0..101 {
            t.record_arrival(now);
            now += Duration::from_micros(1000);
        }
        let snap = t.snapshot();
        assert_eq!(snap.samples_window, 100);
        // tight distribution — all percentiles equal, zero jitter
        assert_eq!(snap.interval_us_p50 as u32, 1000);
        assert_eq!(snap.interval_us_p95 as u32, 1000);
        assert_eq!(snap.interval_us_p99 as u32, 1000);
        assert!(snap.jitter_us < 1.0);
        assert_eq!(snap.dropped_estimate, 0);
    }

    #[test]
    fn drops_detected_on_doubled_intervals() {
        let mut t = LatencyTracker::new();
        let mut now = Instant::now();
        for i in 0..50 {
            t.record_arrival(now);
            // every 10th sample is a 3000us gap
            now += Duration::from_micros(if i % 10 == 9 { 3000 } else { 1000 });
        }
        let snap = t.snapshot();
        // median = 1000, gaps of 3000 > 2*1000 → counted
        assert!(snap.dropped_estimate >= 4);
    }

    #[test]
    fn history_appends_in_order_and_caps() {
        let mut h = LatencyHistory::new(3);
        for i in 0..5u32 {
            let snap = LatencySnapshot {
                interval_us_p50: i as f32,
                ..Default::default()
            };
            h.push(snap);
        }
        assert_eq!(h.len(), 3, "buffer must cap at requested capacity");
        let snap = h.snapshot();
        let p50s: Vec<u32> = snap.iter().map(|s| s.interval_us_p50 as u32).collect();
        assert_eq!(p50s, vec![2, 3, 4], "oldest entries evicted first");
        assert_eq!(h.latest().unwrap().interval_us_p50, 4.0);
    }

    #[test]
    fn history_default_capacity_is_documented_constant() {
        let h = LatencyHistory::default();
        assert_eq!(h.capacity(), DEFAULT_HISTORY_LEN);
    }

    #[test]
    fn history_empty_yields_no_latest() {
        let h = LatencyHistory::new(8);
        assert!(h.is_empty());
        assert!(h.latest().is_none());
        assert!(h.snapshot().is_empty());
    }

    #[test]
    fn history_clear_drops_everything() {
        let mut h = LatencyHistory::new(8);
        h.push(LatencySnapshot::default());
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn history_capacity_zero_treated_as_one() {
        let mut h = LatencyHistory::new(0);
        let s1 = LatencySnapshot {
            interval_us_p50: 1.0,
            ..Default::default()
        };
        let s2 = LatencySnapshot {
            interval_us_p50: 2.0,
            ..Default::default()
        };
        h.push(s1);
        h.push(s2);
        // Cap clamped to 1; only most recent retained.
        assert_eq!(h.len(), 1);
        assert_eq!(h.latest().unwrap().interval_us_p50, 2.0);
    }

    #[test]
    fn send_latency_recorded() {
        let mut t = LatencyTracker::new();
        t.record_send(Duration::from_micros(500));
        t.record_send(Duration::from_micros(1500));
        t.record_send(Duration::from_micros(2500));
        let snap = t.snapshot();
        assert_eq!(snap.send_us_p50 as u32, 1500);
    }
}
