//! Whether a confirmation of the session hangs.

use std::time::{Duration, Instant};

use super::Session;

impl Session<'_> {
    /// How long a confirmation has run, when one runs past its attempts: its thread hangs.
    pub fn stalled(&self) -> Option<Duration> {
        self.worker.as_ref()?.stalled_at(Instant::now())
    }
}
