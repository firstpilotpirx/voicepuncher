//! Asking speech recognition about the head of an utterance and the spotted phrases.

use super::{
    samples_in, JobContext, JobKind, Running, Session, DECODE_FRAME_SAMPLES, EARLY_HEAD_CHECK_SECS,
    HEAD_CHECK_SECS, ONSET_SECS, WINDOW_EVERY_SECS, WINDOW_SECS,
};

impl Session<'_> {
    /// Searches the utterance being spoken: its start for "start", and every second its
    /// last seconds for a doubled or chained command; a check due while another one waits
    /// for its answer waits its turn with the audio it was due for.
    /// Searches the start of running utterance `id` for a command opening speech: early,
    /// when the app asks for it, and once its first words are in.
    pub(super) fn check_head(&mut self, id: u64, now: usize) {
        let Some(&Running { start, head, .. }) = self.utterances.running.get(&id) else {
            return;
        };
        // WHY: only "start": a check this early may hear the words of a doubled "restart" at
        // the clip's end, and its segment would then carry out the last one again.
        let early = head == HeadCheck::Unchecked
            && self.matcher.starts_early()
            && now - start >= samples_in(EARLY_HEAD_CHECK_SECS);
        let full = head != HeadCheck::Checked && now - start >= samples_in(HEAD_CHECK_SECS);
        if early {
            self.ask_head(id, start, now, true);
        }
        if full {
            self.ask_head(id, start, now, false);
        }
    }

    /// Asks the check of the start of utterance `id`, which started at `start`, early or not.
    pub(super) fn ask_head(&mut self, id: u64, start: usize, now: usize, early: bool) {
        self.mark(id, |r| {
            r.head = if early {
                HeadCheck::CheckedEarly
            } else {
                HeadCheck::Checked
            };
        });
        let from = self.pre_rolled(start);
        self.ask_check(
            JobKind::Head,
            from,
            now,
            JobContext::Head { utterance: id, from, early },
        );
    }

    pub(super) fn watch_running(&mut self) {
        if self.worker.is_none() || !self.detector.vad.detected() {
            self.close_current();
            return;
        }
        let now = self.detector.samples_seen;
        let floor = self.utterances.last_segment_end;
        let id = self.current_or_new(now.saturating_sub(samples_in(ONSET_SECS)).max(floor));
        let Some(&Running { start, next_window, .. }) = self.utterances.running.get(&id) else {
            return;
        };
        self.check_head(id, now);
        if now >= next_window {
            self.mark(id, |r| r.next_window = now + samples_in(WINDOW_EVERY_SECS));
            let from = now
                .saturating_sub(samples_in(WINDOW_SECS))
                .max(self.pre_rolled(start));
            self.ask_check(
                JobKind::Window,
                from,
                now,
                JobContext::Window { utterance: id, from },
            );
        }
    }

    /// Asks the check a spotted "start" or "restart" fell due for, when the utterance it was
    /// said in still runs, or is still to be answered for the second check: of its start, or
    /// of its last seconds once it ran longer. The timed checks stay: this one may hear too
    /// little of the speech after the command.
    pub(super) fn ask_spotted_check(&mut self) {
        let now = self.detector.samples_seen;
        let Some(check) = self.jobs.spotted_checks.take_due(now) else {
            return;
        };
        let Some((id, start)) = check
            .utterance
            .or(self.utterances.current)
            .and_then(|id| self.utterances.running.get(&id).map(|r| (id, r.start)))
        else {
            return;
        };
        let head = self.pre_rolled(start);
        if head > check.said_end {
            return;
        }
        // WHY: a second check falls due once the first is answered, late when answers are
        // slow; it hears the audio it would have heard had the first been answered at once.
        let to = now.min(check.due + DECODE_FRAME_SAMPLES);
        let from = to.saturating_sub(samples_in(WINDOW_SECS)).max(head);
        let at_head = from == head;
        self.ask_check(
            JobKind::Spotted,
            from,
            to,
            JobContext::Spotted {
                utterance: id,
                from,
                at_head,
                round: check.round,
                said_end: check.said_end,
            },
        );
    }
}

/// How far the start of a running utterance was searched for a command opening speech.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeadCheck {
    Unchecked,
    /// Searched early, before its first words are all in.
    CheckedEarly,
    /// Searched once its first words are in, or never to be searched.
    Checked,
}
