//! What a request has tried so far and how long it pauses before its next attempt.

use std::time::Duration;

use voicepuncher_copy_pool::Place;

use super::deadline::{RETRY_PAUSES, SLOW_RETRIES, SLOW_RETRY_PAUSE};
use super::{AttemptFailure, OpenAiTranscriber};
use crate::adapters::secondary::http::failure::Failure;

/// The attempts of one request: their count, the copies they went to and the request's
/// place in the queue for a free copy.
pub(super) struct Tries {
    pub(super) attempt: u32,
    pub(super) tried: Vec<u16>,
    pub(super) place: Place,
}

impl Tries {
    pub(super) const fn new(place: Place) -> Self {
        Self { attempt: 0, tried: Vec::new(), place }
    }
}

/// The retries a request has used: dropped connections, slow answers and the one wait
/// for a copy to come back.
#[derive(Default)]
pub(super) struct Retries {
    dropped: usize,
    slow: u32,
    waited_for_copy: bool,
}

impl Retries {
    /// The pause before the next attempt after `failed`, or `None` when the request gives up.
    pub(super) fn pause(
        &mut self,
        failed: &AttemptFailure,
        transcriber: &OpenAiTranscriber,
    ) -> Option<Duration> {
        match failed.failure {
            Failure::Dropped => self.after_dropped(transcriber),
            Failure::TimedOut | Failure::Busy(_) => self.after_slow(failed.waiting),
            Failure::Other => None,
        }
    }

    /// The pause after a dropped connection: the next of the retry pauses, then one wait for
    /// a copy to come back when none is left.
    fn after_dropped(&mut self, transcriber: &OpenAiTranscriber) -> Option<Duration> {
        if self.dropped < RETRY_PAUSES.len() {
            self.dropped += 1;
            return Some(RETRY_PAUSES[self.dropped - 1]);
        }
        if self.waited_for_copy || transcriber.pool.has_copies() {
            return None;
        }
        self.waited_for_copy = true;
        transcriber.wait_for_copy().then(|| {
            self.dropped = 0;
            Duration::ZERO
        })
    }

    /// The pause after a slow answer, none while the request waited in the queue.
    fn after_slow(&mut self, waiting: bool) -> Option<Duration> {
        if self.slow >= SLOW_RETRIES {
            return None;
        }
        self.slow += 1;
        Some(if waiting {
            Duration::ZERO
        } else {
            SLOW_RETRY_PAUSE
        })
    }
}
