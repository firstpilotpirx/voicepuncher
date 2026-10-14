//! Keeps a connection's events in stream order while its confirmations are answered later:
//! a closed segment's events wait for its confirmation, and a sync is answered once every
//! confirmation asked for before it is resolved.

use std::collections::{BTreeSet, VecDeque};

use crate::protocol::ServerEvent;

#[derive(Default)]
pub struct Outbox {
    /// Confirmations asked for and not resolved yet, by job number.
    pending: BTreeSet<u64>,
    /// How many of the pending confirmations check a running utterance.
    checks: usize,
    held: VecDeque<Held>,
    /// Syncs not answered yet, with the last job asked for before each.
    syncs: VecDeque<(u64, Option<u64>)>,
    last_job: Option<u64>,
}

struct Held {
    job: u64,
    events: Vec<ServerEvent>,
    resolved: bool,
}

impl Outbox {
    pub fn asked(&mut self, job: u64, is_check: bool) {
        self.pending.insert(job);
        self.checks += usize::from(is_check);
        self.last_job = Some(job);
    }

    /// A check of a running utterance waits for its answer.
    pub fn check_busy(&self) -> bool {
        self.checks > 0
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Holds a closed segment's `events` until its confirmation `job` is resolved.
    pub fn hold(&mut self, job: u64, events: Vec<ServerEvent>) {
        self.held.push_back(Held {
            job,
            events,
            resolved: false,
        });
    }

    /// Events of no confirmation: they go out after the segments still held, or at once.
    pub fn send(&mut self, events: Vec<ServerEvent>) -> Vec<ServerEvent> {
        match self.held.back_mut() {
            Some(held) => {
                held.events.extend(events);
                Vec::new()
            }
            None => events,
        }
    }

    /// Resolves `job`: `events` go out after the segment it held, if any, and every held
    /// segment that is now first in line goes out with them, then the syncs it unblocks.
    pub fn resolve(
        &mut self,
        job: u64,
        is_check: bool,
        events: Vec<ServerEvent>,
    ) -> Vec<ServerEvent> {
        if self.pending.remove(&job) && is_check {
            self.checks -= 1;
        }
        let mut out = Vec::new();
        match self.held.iter_mut().find(|h| h.job == job) {
            Some(held) => {
                held.events.extend(events);
                held.resolved = true;
            }
            // A check of a running utterance answers before the utterance's own segment.
            None if is_check => out.extend(events),
            None => out.extend(self.send(events)),
        }
        while self.held.front().is_some_and(|h| h.resolved) {
            out.extend(self.held.pop_front().map(|h| h.events).unwrap_or_default());
        }
        out.extend(self.answered_syncs());
        out
    }

    /// Answers sync `id` now when nothing asked for before it is pending, else later.
    pub fn sync(&mut self, id: u64) -> Vec<ServerEvent> {
        self.syncs.push_back((id, self.last_job));
        self.answered_syncs()
    }

    fn answered_syncs(&mut self) -> Vec<ServerEvent> {
        let oldest_pending = self.pending.first().copied();
        let mut out = Vec::new();
        while let Some(&(id, last)) = self.syncs.front() {
            // Held segments belong to pending jobs, so no pending job up to `last` means
            // nothing asked for before the sync still waits.
            if matches!((last, oldest_pending), (Some(last), Some(oldest)) if oldest <= last) {
                break;
            }
            self.syncs.pop_front();
            out.push(ServerEvent::Synced { id });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(t: f32) -> ServerEvent {
        ServerEvent::SpeechStart { t }
    }

    fn end(t: f32) -> ServerEvent {
        ServerEvent::SpeechEnd { t }
    }

    fn phrase(name: &str) -> ServerEvent {
        ServerEvent::Phrase {
            phrase: name.to_owned(),
            start: 0.0,
            end: 0.0,
        }
    }

    #[test]
    fn should_send_a_held_segment_with_its_confirmation_after_it() {
        // Given
        let mut outbox = Outbox::default();
        outbox.asked(1, false);
        outbox.hold(1, vec![start(1.0), end(2.0)]);
        // When
        let out = outbox.resolve(1, false, vec![phrase("press enter")]);
        // Then
        assert_eq!(out, [start(1.0), end(2.0), phrase("press enter")]);
    }

    #[test]
    fn should_keep_a_later_segment_behind_an_earlier_one_still_waiting() {
        // Given
        let mut outbox = Outbox::default();
        outbox.asked(1, false);
        outbox.hold(1, vec![start(1.0), end(2.0)]);
        outbox.asked(2, false);
        outbox.hold(2, vec![start(3.0), end(4.0)]);
        // When
        let early = outbox.resolve(2, false, Vec::new());
        let late = outbox.resolve(1, false, Vec::new());
        // Then
        assert!(early.is_empty());
        assert_eq!(late, [start(1.0), end(2.0), start(3.0), end(4.0)]);
    }

    #[test]
    fn should_send_the_answer_of_a_check_at_once() {
        // Given
        let mut outbox = Outbox::default();
        outbox.asked(1, true);
        // When
        let out = outbox.resolve(1, true, vec![phrase("stop dictation")]);
        // Then
        assert_eq!(out, [phrase("stop dictation")]);
        assert!(!outbox.check_busy());
    }

    #[test]
    fn should_send_other_events_after_the_segments_still_held() {
        // Given
        let mut outbox = Outbox::default();
        outbox.asked(1, false);
        outbox.hold(1, vec![end(2.0)]);
        // When
        let early = outbox.send(vec![phrase("press enter")]);
        let out = outbox.resolve(1, false, Vec::new());
        // Then
        assert!(early.is_empty());
        assert_eq!(out, [end(2.0), phrase("press enter")]);
    }

    #[test]
    fn should_be_busy_while_a_check_waits() {
        // When
        let mut outbox = Outbox::default();
        outbox.asked(1, true);
        // Then
        assert!(outbox.check_busy());
    }

    #[test]
    fn should_answer_a_sync_at_once_when_nothing_waits() {
        assert_eq!(Outbox::default().sync(7), [ServerEvent::Synced { id: 7 }]);
    }

    #[test]
    fn should_answer_a_sync_only_after_the_confirmations_asked_for_before_it() {
        // Given
        let mut outbox = Outbox::default();
        outbox.asked(1, false);
        outbox.hold(1, vec![end(2.0)]);
        // When
        let early = outbox.sync(7);
        outbox.asked(2, true);
        let out = outbox.resolve(1, false, Vec::new());
        // Then
        assert!(early.is_empty());
        assert_eq!(out, [end(2.0), ServerEvent::Synced { id: 7 }]);
    }
}
