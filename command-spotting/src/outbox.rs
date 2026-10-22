//! Keeps a connection's events in stream order while its confirmations are answered later:
//! a closed segment's events wait for its confirmation, and a sync is answered once every
//! confirmation asked for before it is resolved.

use std::collections::{BTreeSet, VecDeque};

use crate::protocol::{ServerEvent, Timing};

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
        self.held.push_back(Held { job, events, resolved: false });
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

    /// Phrases of the segment that ended last, spotted after it ended: they go out before
    /// its end when it is still held, or at once.
    pub fn send_in_last_segment(&mut self, events: Vec<ServerEvent>) -> Vec<ServerEvent> {
        let Some(held) = self.held.back_mut() else {
            return events;
        };
        match held
            .events
            .iter()
            .rposition(|e| matches!(e, ServerEvent::SpeechEnd { .. }))
        {
            Some(end) => {
                held.events.splice(end..end, events);
            }
            None => held.events.extend(events),
        }
        Vec::new()
    }

    /// Drops from the segment `job` holds every spotted phrase but `phrase`: its
    /// confirmation heard that command, with `timing`, which the phrases kept take on.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    pub fn keep_only_phrase(&mut self, job: u64, phrase: &str, timing: Option<Timing>) {
        if let Some(held) = self.held.iter_mut().find(|h| h.job == job) {
            held.events.retain(|e| match e {
                ServerEvent::Phrase { phrase: spotted, .. } => spotted == phrase,
                _ => true,
            });
            for event in &mut held.events {
                if let ServerEvent::Phrase { timing: kept, .. } = event {
                    *kept = timing;
                }
            }
        }
    }

    /// The segment `from` holds waits for the confirmation `to` instead.
    pub fn transfer(&mut self, from: u64, to: u64) {
        if let Some(held) = self.held.iter_mut().find(|h| h.job == from) {
            held.job = to;
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
mod tests;
