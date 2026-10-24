//! Second hearings: a near miss heard again, early or after its first hearing.

use std::collections::HashMap;

use crate::confirm::Heard;
use crate::keywords;
use crate::matcher::PhraseMatcher;
use crate::orderer::{Segment, Spotted};
use crate::protocol::ServerEvent;
use crate::worker::JobKind;

use super::super::clip::steady_clip;
use super::super::{samples_in, seconds, Session};

use super::JobContext;

impl Session<'_> {
    /// Hears the last short segment heard as no command again when `trigger`, reported
    /// after that answer, lies in it.
    pub(in crate::engine) fn hear_misheard_again(&mut self, trigger: &Spotted) {
        let Some(phrase) =
            keywords::served_by(&trigger.phrase).filter(|phrase| self.matcher.selects(phrase))
        else {
            return;
        };
        let Some((job, segment)) = self.hearings.misheard.take_if(|(_, segment)| {
            trigger.start < segment.bounds.end && trigger.end > segment.bounds.start
        }) else {
            return;
        };
        let phrase = phrase.to_owned();
        self.hear_again(job, Rehear { phrase, ..segment });
    }

    /// Hears the short segment of job `job`, still being heard, again at once when a trigger
    /// already lies in it; else when one is spotted in it before its answer comes back.
    pub(super) fn hear_early_when_triggered(&mut self, job: u64, segment: Rehear) {
        match self.trigger_heard_in(&self.matcher, segment.bounds) {
            Some(phrase) => self.hear_early(job, Rehear { phrase, ..segment }),
            None => self.hearings.early.pending.push((job, segment)),
        }
    }

    /// Hears a short segment still being heard again at once when `trigger` lies in it.
    pub(in crate::engine) fn hear_pending_again(&mut self, trigger: &Spotted) {
        let Some(phrase) =
            keywords::served_by(&trigger.phrase).filter(|phrase| self.matcher.selects(phrase))
        else {
            return;
        };
        let Some(at) = self.hearings.early.pending.iter().position(|(_, segment)| {
            trigger.start < segment.bounds.end && trigger.end > segment.bounds.start
        }) else {
            return;
        };
        let (job, segment) = self.hearings.early.pending.remove(at);
        let phrase = phrase.to_owned();
        self.hear_early(job, Rehear { phrase, ..segment });
    }

    /// Asks the second hearing of the segment of job `job` beside its first one.
    pub(super) fn hear_early(&mut self, job: u64, rehear: Rehear) {
        if let Some(again) = self.ask_rehear(rehear) {
            log::debug!(
                "conn={} job {job} heard again at once by job {again}",
                self.conn
            );
            self.hearings
                .early
                .asked
                .insert(again, Early::Waiting { segment: job });
        }
    }

    /// Hears a short segment heard as a near miss again, through its steady clip; the
    /// segment's events wait for that answer. False when it cannot be asked.
    pub(super) fn hear_again(&mut self, job: u64, rehear: Rehear) -> bool {
        match self.ask_rehear(rehear) {
            Some(again) => {
                self.jobs.outbox.transfer(job, again);
                true
            }
            None => false,
        }
    }

    /// Asks the second hearing of a short segment, through its steady clip.
    pub(super) fn ask_rehear(&mut self, rehear: Rehear) -> Option<u64> {
        let Rehear { utterance, bounds, from, to, quiet_level, phrase } = rehear;
        let audio = self.recent.between(from, to);
        let clip = steady_clip(&audio, from, quiet_level);
        log::debug!(
            "conn={} near miss of {phrase:?}: heard again from {:.2}s, gain {:.1}",
            self.conn,
            seconds(clip.origin),
            clip.gain
        );
        let origin = clip.origin;
        let context = JobContext::Rehear { utterance, bounds, origin, phrase };
        self.ask_clip(JobKind::Rehear, clip.samples, origin, to, context)
    }

    /// The near-missed command, when the second hearing says it as a whole: of the segment
    /// within `bounds`, heard through a clip whose first sample stands for `origin`.
    pub(super) fn apply_rehear(
        &self,
        reading: &PhraseMatcher,
        heard: &Heard,
        (bounds, origin): (Segment, usize),
        phrase: String,
    ) -> Option<ServerEvent> {
        log::debug!("conn={} heard again: {:?}", self.conn, heard.text);
        if reading.phrase_in(&heard.text).as_deref() != Some(phrase.as_str()) {
            return None;
        }
        log::info!("conn={} near miss heard again → {phrase:?}", self.conn);
        let start = heard
            .words
            .first()
            .map_or(bounds.start, |w| seconds(origin + samples_in(w.start)));
        Some(ServerEvent::Phrase {
            phrase,
            start: start.min(bounds.start),
            end: bounds.end,
            timing: None,
        })
    }
}

/// The second hearings of short segments asked beside their first hearing, so a trigger in a
/// short segment costs one hearing's time, not two in a row.
#[derive(Default)]
pub(in crate::engine) struct EarlyRehears {
    /// Short segments whose first hearing is pending, by job: a trigger spotted in one has it
    /// heard again at once.
    pub(super) pending: Vec<(u64, Rehear)>,
    /// The second hearings asked early, by job.
    pub(super) asked: HashMap<u64, Early>,
}

/// A second hearing asked beside its segment's first one.
pub(super) enum Early {
    /// Neither hearing answered yet, or only this one is pending.
    Waiting { segment: u64 },
    /// It answered first, with these events; they stand when the first hearing holds none.
    Answered { segment: u64, events: Vec<ServerEvent> },
    /// The first hearing held a command: its answer is dropped.
    Dropped,
}

/// What a segment's first hearing makes of the second one asked beside it.
pub(super) enum EarlyAnswer {
    /// Its answer came first: these events stand for the segment.
    Heard(Vec<ServerEvent>),
    /// It is still pending, as job `.0`: the segment waits for it.
    Pending(u64),
    /// The first hearing held a command: the second one is dropped.
    Dropped,
}

impl EarlyRehears {
    /// The second hearings of segment job `from` belong to segment job `to` now.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    pub(super) fn retarget(&mut self, from: u64, to: u64) {
        for (job, _) in &mut self.pending {
            if *job == from {
                *job = to;
            }
        }
        for early in self.asked.values_mut() {
            if let Early::Waiting { segment } | Early::Answered { segment, .. } = early {
                if *segment == from {
                    *segment = to;
                }
            }
        }
    }

    /// The events the answer of second hearing `job` sends now: none while its segment's
    /// first hearing is pending, or when that one held a command.
    pub(super) fn rehear_answered(
        &mut self,
        job: u64,
        events: Vec<ServerEvent>,
    ) -> Vec<ServerEvent> {
        match self.asked.remove(&job) {
            Some(Early::Waiting { segment }) => {
                self.asked.insert(job, Early::Answered { segment, events });
                Vec::new()
            }
            Some(Early::Dropped) => Vec::new(),
            Some(answered @ Early::Answered { .. }) => {
                self.asked.insert(job, answered);
                Vec::new()
            }
            None => events,
        }
    }

    /// The second hearing of segment job `segment`, now that its first one answered, with or
    /// without a command; none when it has none.
    pub(super) fn segment_answered(
        &mut self,
        segment: u64,
        held_command: bool,
    ) -> Option<EarlyAnswer> {
        let (&job, _) = self.asked.iter().find(|(_, early)| match early {
            Early::Waiting { segment: s } | Early::Answered { segment: s, .. } => *s == segment,
            Early::Dropped => false,
        })?;
        let early = self.asked.remove(&job)?;
        Some(match (early, held_command) {
            (Early::Answered { .. }, true) => EarlyAnswer::Dropped,
            (Early::Answered { events, .. }, false) => EarlyAnswer::Heard(events),
            (_, true) => {
                self.asked.insert(job, Early::Dropped);
                EarlyAnswer::Dropped
            }
            (_, false) => EarlyAnswer::Pending(job),
        })
    }
}

/// What the second hearing of a near miss needs from its segment.
pub(in crate::engine) struct Rehear {
    pub(super) utterance: u64,
    pub(super) bounds: Segment,
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) quiet_level: f32,
    pub(super) phrase: String,
}
