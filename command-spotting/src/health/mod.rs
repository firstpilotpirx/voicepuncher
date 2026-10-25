//! Whether every connection's processing keeps up: a connection with audio waiting that made
//! no progress for a while is stuck, and the service scripts restart a stuck service.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a connection with audio waiting may make no progress before it counts as stuck;
/// well above the longest a confirmation holds anything, since none holds the audio.
const STUCK_AFTER: Duration = Duration::from_secs(10);
/// The longest audio may wait for the processing thread: past it, the service no longer
/// keeps up. Audio sent faster than real time (a replay) waits only while it is processed.
const LONGEST_WAIT: Duration = Duration::from_secs(10);

/// One connection's progress, written by its threads.
pub struct Beat {
    last_progress: Mutex<Instant>,
    /// When each audio frame waiting for the processing thread arrived, oldest first.
    waiting: Mutex<VecDeque<Instant>>,
    /// A confirmation of the connection runs past its attempts: its thread hangs.
    stalled: AtomicBool,
}

impl Beat {
    fn new() -> Self {
        Self {
            last_progress: Mutex::new(Instant::now()),
            waiting: Mutex::default(),
            stalled: AtomicBool::new(false),
        }
    }

    /// Whether an audio frame arriving now may wait for the processing thread, counting it
    /// when it may; audio is to be dropped once the oldest waiting frame waited past 10 s.
    pub fn queued(&self) -> bool {
        self.queued_at(Instant::now())
    }

    fn queued_at(&self, now: Instant) -> bool {
        let Ok(mut waiting) = self.waiting.lock() else {
            return true;
        };
        let behind = waiting
            .front()
            .is_some_and(|oldest| now.saturating_duration_since(*oldest) > LONGEST_WAIT);
        if !behind {
            waiting.push_back(now);
        }
        !behind
    }

    /// The processing thread took the oldest waiting audio frame.
    pub fn processed(&self) {
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.pop_front();
        }
    }

    /// Whether a confirmation of the connection runs past its attempts.
    pub fn stalled(&self, stalled: bool) {
        self.stalled.store(stalled, Ordering::Relaxed);
    }

    /// The processing thread went round its loop.
    pub fn alive(&self) {
        if let Ok(mut last) = self.last_progress.lock() {
            *last = Instant::now();
        }
    }

    fn stuck(&self, now: Instant, after: Duration) -> bool {
        let idle = self
            .last_progress
            .lock()
            .map(|last| now.saturating_duration_since(*last))
            .unwrap_or_default();
        let waiting = self.waiting.lock().is_ok_and(|waiting| !waiting.is_empty());
        self.stalled.load(Ordering::Relaxed) || waiting && idle > after
    }
}

/// Every open connection's progress, shared by the accept thread and the connections.
#[derive(Clone)]
pub struct Health {
    beats: Arc<Mutex<HashMap<u64, Arc<Beat>>>>,
    stuck_after: Duration,
}

impl Default for Health {
    fn default() -> Self {
        Self::stuck_after(STUCK_AFTER)
    }
}

impl Health {
    /// Counts a connection as stuck once it made no progress for `after` with audio waiting.
    pub fn stuck_after(after: Duration) -> Self {
        Self { beats: Arc::default(), stuck_after: after }
    }

    pub fn register(&self, conn: u64) -> Arc<Beat> {
        let beat = Arc::new(Beat::new());
        if let Ok(mut beats) = self.beats.lock() {
            beats.insert(conn, Arc::clone(&beat));
        }
        beat
    }

    pub fn unregister(&self, conn: u64) {
        if let Ok(mut beats) = self.beats.lock() {
            beats.remove(&conn);
        }
    }

    /// The first connection that is stuck, if any.
    pub fn stuck(&self) -> Option<u64> {
        self.stuck_at(Instant::now())
    }

    fn stuck_at(&self, now: Instant) -> Option<u64> {
        let beats = self.beats.lock().ok()?;
        beats
            .iter()
            .find(|(_, beat)| beat.stuck(now, self.stuck_after))
            .map(|(conn, _)| *conn)
    }
}

#[cfg(test)]
mod tests;
