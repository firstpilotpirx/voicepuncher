//! The counters a connection logs now and then.

use std::time::{Duration, Instant};

/// Audio that waited longer than this for the processing thread is logged as lag.
const LAG_LOGGED: Duration = Duration::from_secs(1);
/// How often a connection logs its counters.
const STATS_EVERY: Duration = Duration::from_secs(30);

/// Counters a connection logs every `STATS_EVERY`.
pub(super) struct Stats {
    since: Instant,
    messages: u64,
    worst_lag: Duration,
}

impl Stats {
    pub(super) fn new() -> Self {
        Self { since: Instant::now(), messages: 0, worst_lag: Duration::ZERO }
    }

    pub(super) fn audio(&mut self, waited: Duration, conn: u64) {
        self.messages += 1;
        self.worst_lag = self.worst_lag.max(waited);
        if waited > LAG_LOGGED {
            log::info!(
                "conn={conn} lag={}ms: audio waited for processing",
                waited.as_millis()
            );
        }
    }

    pub(super) fn maybe_log(&mut self, conn: u64, pending: usize) {
        if self.since.elapsed() < STATS_EVERY {
            return;
        }
        log::info!(
            "stats conn={conn} messages={} worst_lag={}ms confirmations_pending={pending}",
            self.messages,
            self.worst_lag.as_millis()
        );
        *self = Self::new();
    }
}
