use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Progress {
    pub discovery_total: u64,
    pub discovery_done: u64,
    pub discovery_seconds: f64,
    costs: HashMap<String, (u64, f64)>,
    pause_started: Option<Instant>,
    paused: Duration,
    discovery_started: Option<Instant>,
    discovery_pause_base: f64,
}
impl Progress {
    pub fn reset_discovery(&mut self) {
        self.discovery_total = 0;
        self.discovery_done = 0;
        self.discovery_seconds = 0.;
        self.discovery_started = None;
    }
    pub fn set_paused(&mut self, paused: bool) {
        if paused && self.pause_started.is_none() {
            self.pause_started = Some(Instant::now());
        }
        if !paused && let Some(started) = self.pause_started.take() {
            self.paused += started.elapsed();
        }
    }
    pub fn paused_seconds(&self) -> f64 {
        self.paused.as_secs_f64() + self.pause_started.map_or(0., |s| s.elapsed().as_secs_f64())
    }
    pub fn start_discovery(&mut self) {
        self.discovery_started = Some(Instant::now());
        self.discovery_pause_base = self.paused_seconds();
    }
    pub fn discovered_file(&mut self) {
        self.discovery_done += 1;
        if let Some(started) = self.discovery_started {
            self.discovery_seconds = (started.elapsed().as_secs_f64()
                - (self.paused_seconds() - self.discovery_pause_base))
                .max(0.);
        }
    }
    pub fn discovery_eta(&self) -> Option<f64> {
        (self.discovery_done >= 128 && self.discovery_seconds > 0.).then(|| {
            self.discovery_total.saturating_sub(self.discovery_done) as f64 * self.discovery_seconds
                / self.discovery_done as f64
        })
    }
    pub fn record(&mut self, kind: &str, seconds: f64) {
        let cost = self.costs.entry(kind.to_owned()).or_default();
        cost.0 += 1;
        cost.1 = if cost.0 == 1 {
            seconds
        } else {
            cost.1 * 0.8 + seconds * 0.2
        };
    }
    pub fn index_eta(&self, pending: &[(String, u64)]) -> (Option<f64>, bool) {
        if pending.is_empty() {
            return (Some(0.), false);
        }
        let samples: u64 = self.costs.values().map(|c| c.0).sum();
        if samples < 3 {
            return (None, true);
        }
        let fallback = self.costs.values().map(|c| c.1 * c.0 as f64).sum::<f64>() / samples as f64;
        let mut provisional = false;
        let seconds = pending
            .iter()
            .map(|(kind, n)| {
                let cost = self.costs.get(kind).filter(|c| c.0 >= 3);
                provisional |= cost.is_none();
                *n as f64 * cost.map_or(fallback, |c| c.1)
            })
            .sum();
        (Some(seconds), provisional)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn estimates_wait_for_samples_and_adapt_to_media() {
        let mut p = Progress::default();
        assert_eq!(p.discovery_eta(), None);
        p.discovery_total = 1024;
        p.discovery_done = 256;
        p.discovery_seconds = 2.;
        assert_eq!(p.discovery_eta(), Some(6.));
        assert_eq!(p.index_eta(&[("text".into(), 10)]), (None, true));
        for _ in 0..3 {
            p.record("text", 0.5);
        }
        assert_eq!(p.index_eta(&[("text".into(), 10)]), (Some(5.), false));
        assert!(p.index_eta(&[("image".into(), 10)]).1);
        for _ in 0..3 {
            p.record("image", 2.);
        }
        assert_eq!(
            p.index_eta(&[("text".into(), 10), ("image".into(), 10)]),
            (Some(25.), false)
        );
        p.reset_discovery();
        assert_eq!(p.discovery_eta(), None);
    }
}
