//! Whether every connection's processing keeps up: a connection with audio waiting that made
//! no progress for a while is stuck, and the service scripts restart a stuck service.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a connection with audio waiting may make no progress before it counts as stuck;
/// well above the longest a confirmation holds anything, since none holds the audio.
const STUCK_AFTER: Duration = Duration::from_secs(10);

/// One connection's progress, written by its threads.
pub struct Beat {
    last_progress: Mutex<Instant>,
    waiting: AtomicUsize,
}

impl Beat {
    fn new() -> Self {
        Self { last_progress: Mutex::new(Instant::now()), waiting: AtomicUsize::new(0) }
    }

    /// A message of audio waits for the processing thread.
    pub fn queued(&self) {
        self.waiting.fetch_add(1, Ordering::Relaxed);
    }

    /// The processing thread took a message of audio.
    pub fn processed(&self) {
        let _ = self
            .waiting
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1));
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
        self.waiting.load(Ordering::Relaxed) > 0 && idle > after
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
mod tests {
    use super::*;

    const LATER: Duration = Duration::from_secs(11);

    #[test]
    fn should_be_healthy_without_connections() {
        assert_eq!(Health::default().stuck(), None);
    }

    #[test]
    fn should_count_a_connection_with_audio_waiting_and_no_progress_as_stuck() {
        // Given
        let health = Health::default();
        let beat = health.register(4);
        // When
        beat.queued();
        // Then
        assert_eq!(health.stuck_at(Instant::now() + LATER), Some(4));
    }

    #[test]
    fn should_not_count_an_idle_connection_as_stuck() {
        // Given
        let health = Health::default();
        let _beat = health.register(4);
        // Then
        assert_eq!(health.stuck_at(Instant::now() + LATER), None);
    }

    #[test]
    fn should_not_count_a_connection_that_keeps_up_as_stuck() {
        // Given
        let health = Health::default();
        let beat = health.register(4);
        // When
        beat.queued();
        beat.processed();
        beat.alive();
        // Then
        assert_eq!(health.stuck_at(Instant::now() + LATER), None);
    }

    #[test]
    fn should_forget_a_closed_connection() {
        // Given
        let health = Health::default();
        health.register(4).queued();
        // When
        health.unregister(4);
        // Then
        assert_eq!(health.stuck_at(Instant::now() + LATER), None);
    }
}
