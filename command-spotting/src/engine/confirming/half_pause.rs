//! A short utterance heard at half its pause, before the pause ends.

use crate::confirm::Heard;
use crate::worker::JobKind;

use super::super::Session;

use super::JobContext;

impl Session<'_> {
    /// Hears a short utterance of standby at half its pause, whose quiet run began at
    /// `quiet_from`, from its pre-roll to `now`.
    pub(in crate::engine) fn hear_at_half_pause(&mut self, quiet_from: usize, now: usize) {
        if self.hearings.half_pause.is_some() || !self.matcher.starts_early() {
            return;
        }
        let Some((utterance, start)) = self
            .utterances
            .current
            .and_then(|id| self.utterances.running.get(&id).map(|r| (id, r.start)))
        else {
            return;
        };
        if quiet_from.saturating_sub(start) > self.engine.max_command_samples {
            return;
        }
        let from = self.pre_rolled(start);
        if let Some(job) = self.ask(
            JobKind::Segment,
            from,
            now,
            JobContext::HalfPause { utterance },
        ) {
            self.hearings.half_pause =
                Some(HalfPauseHearing { job, quiet_from, from, answer: HalfPauseAnswer::Awaited });
        }
    }

    /// Drops the hearing at half the pause once a louder frame broke its quiet run: the
    /// utterance goes on, and its segment is heard whole.
    pub(in crate::engine) fn drop_broken_half_pause_hearing(&mut self) {
        let quiet_run = self.detector.pause.quiet_run_from();
        if let Some(dropped) = self
            .hearings
            .half_pause
            .take_if(|h| quiet_run != Some(h.quiet_from))
        {
            log::debug!(
                "job={} heard at half the pause: dropped, the utterance goes on",
                dropped.job
            );
        }
    }
}

/// The hearing of a short utterance asked at half its pause.
pub(in crate::engine) struct HalfPauseHearing {
    pub(super) job: u64,
    /// Where the quiet run began: only this run closing the utterance keeps the hearing.
    pub(super) quiet_from: usize,
    /// Where its audio starts.
    pub(super) from: usize,
    /// What was heard, once answered.
    pub(super) answer: HalfPauseAnswer,
}

/// What a hearing at half the pause has answered so far.
pub(in crate::engine) enum HalfPauseAnswer {
    /// No answer yet.
    Awaited,
    Heard(Heard),
    /// It answered and heard nothing.
    NothingHeard,
}

impl HalfPauseAnswer {
    /// What it heard; none before its answer or when it heard nothing.
    pub(super) fn heard(self) -> Option<Heard> {
        match self {
            Self::Heard(heard) => Some(heard),
            Self::Awaited | Self::NothingHeard => None,
        }
    }
}
