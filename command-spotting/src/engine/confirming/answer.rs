//! What an answer of the speech recognition service makes of its job.

use std::sync::Arc;

use crate::confirm::Heard;
use crate::keywords;
use crate::matcher::PhraseMatcher;
use crate::orderer::Segment;
use crate::protocol::{ServerEvent, Timing};
use crate::worker::{Answer, JobId, JobKind};

use super::super::spotted::Round;
use super::super::Session;

use super::place::HeardClip;
use super::{
    head_command, spotted_command, timed, AskedJob, EarlyAnswer, JobContext, Rehear, SegmentSpan,
};

/// What becomes of an answered job.
enum Standing {
    /// The answer applies to the job.
    Applies(AskedJob),
    /// The job stood for a segment now heard whole through its own clip: these events
    /// resolve it, and the segment's wait for that hearing.
    HeardWhole(Vec<ServerEvent>),
}

/// What the second hearings asked early made of a segment's answer.
#[derive(Default)]
struct EarlyHearing {
    /// The segment had one: it decided the segment's events.
    heard: bool,
    /// It is still pending: the segment's events wait for it.
    pending: bool,
}

impl Session<'_> {
    pub(super) fn apply(&mut self, answer: Answer) -> Vec<ServerEvent> {
        let n = answer.id.n;
        self.jobs.checks.answered(n);
        let Some(asked) = self.jobs.asked.remove(&n) else {
            return Vec::new();
        };
        let asked = match self.hear_whole_instead(n, &answer, asked) {
            Standing::Applies(asked) => asked,
            Standing::HeardWhole(events) => return events,
        };
        // An answer that comes back after the phrases changed reads with both sets: the
        // words were said under the job's phrases, or are said now under the current ones.
        let reading = if Arc::ptr_eq(&asked.matcher, &self.matcher) {
            Arc::clone(&self.matcher)
        } else {
            Arc::new(asked.matcher.combined_with(&self.matcher))
        };
        let failed = answer
            .heard
            .is_none()
            .then(|| asked.context.failed())
            .flatten();
        let first_asked = self.jobs.next_job;
        let mut events = self.apply_answer(&reading, answer, asked);
        // What the answer asks in turn reads its own answer with the same phrases.
        for (_, asked) in self
            .jobs
            .asked
            .iter_mut()
            .filter(|(job, _)| **job >= first_asked)
        {
            asked.matcher = Arc::clone(&reading);
        }
        if let Some(failed) = failed {
            log::warn!(
                "conn={} job={n} {failed:?}: no answer, a command said in it is lost",
                self.conn
            );
            events.extend(self.jobs.outbox.send(vec![failed]));
        }
        events
    }

    /// Whether `heard` says a selected phrase alone: a hearing at half the pause that heard
    /// anything else may have heard its clip cut short wrong ("okay, paste" as "okay, based").
    pub(super) fn said_alone(&self, heard: &Heard) -> bool {
        self.matcher.phrase_in(&heard.text).is_some()
    }

    /// Has the segment of job `n`, which a hearing at half the pause stood for, heard whole
    /// through its own clip when `answer` holds no phrase alone; the job is then resolved, and
    /// its events wait for that hearing. Else `asked` applies.
    fn hear_whole_instead(&mut self, n: u64, answer: &Answer, asked: AskedJob) -> Standing {
        let Some(own_from) = self.hearings.stand_ins.remove(&n) else {
            return Standing::Applies(asked);
        };
        let (Some(heard), JobContext::Segment { utterance, segment }) =
            (&answer.heard, &asked.context)
        else {
            return Standing::Applies(asked);
        };
        if self.said_alone(heard) {
            return Standing::Applies(asked);
        }
        let (utterance, segment) = (*utterance, SegmentSpan { from: own_from, ..*segment });
        let to = segment.to;
        let context = JobContext::Segment { utterance, segment };
        let Some(whole) = self.ask(JobKind::Segment, own_from, to, context) else {
            return Standing::Applies(asked);
        };
        log::debug!("job={n} heard at half the pause held no command alone: job {whole} hears the segment whole");
        self.jobs.outbox.transfer(n, whole);
        self.hearings.early.retarget(n, whole);
        self.mark(utterance, |r| r.jobs -= 1);
        Standing::HeardWhole(self.jobs.outbox.resolve(n, false, Vec::new()))
    }

    /// Applies `answer` to the job it answers, reading it with `reading`.
    fn apply_answer(
        &mut self,
        reading: &PhraseMatcher,
        answer: Answer,
        asked: AskedJob,
    ) -> Vec<ServerEvent> {
        let (id, n) = (answer.id, answer.id.n);
        let (timing, asked_at, context) = (asked.timing(), asked.asked_at, asked.context);
        self.keep_half_pause_answer(n, &context, answer.heard.as_ref());
        let (utterance, is_check) = (context.utterance(), context.is_check());
        let (tail, short_segment) = (context.tail_check(), context.short_segment());
        let is_segment = matches!(context, JobContext::Segment { .. });
        let is_rehear = matches!(context, JobContext::Rehear { .. });
        let rehear = self.near_miss_in(reading, &context, answer.heard.as_ref());
        let events: Vec<ServerEvent> = self
            .unless_stale(&context, answer)
            .and_then(|heard| self.apply_heard(reading, heard, context, asked_at))
            .into_iter()
            .map(|event| timed(event, timing))
            .collect();
        let (events, early) = self.with_early_hearing(n, is_segment, is_rehear, events);
        let heard_again = early.pending
            || match rehear {
                Some(rehear) if events.is_empty() && !early.heard => self.hear_again(n, rehear),
                _ => false,
            };
        if short_segment.is_some() {
            self.hearings.misheard = short_segment
                .filter(|_| events.is_empty() && !heard_again && !early.heard)
                .map(|segment| (n, segment));
        }
        if events.is_empty() && !heard_again {
            self.hear_tail(n, tail);
        }
        self.resolve_answer(id, (utterance, is_check), timing, events)
    }

    /// Keeps what a hearing at half the pause heard, for the segment it may stand for.
    fn keep_half_pause_answer(&mut self, n: u64, context: &JobContext, heard: Option<&Heard>) {
        if !matches!(context, JobContext::HalfPause { .. }) {
            return;
        }
        if let Some(half_pause) = self.hearings.half_pause.as_mut().filter(|h| h.job == n) {
            half_pause.answer = Some(heard.cloned());
        }
    }

    /// The second hearing a short segment's answer asks for when it nearly says a command,
    /// or when a trigger spotted in it serves one.
    fn near_miss_in(
        &self,
        reading: &PhraseMatcher,
        context: &JobContext,
        heard: Option<&Heard>,
    ) -> Option<Rehear> {
        let (JobContext::Segment { segment, .. }, Some(heard)) = (context, heard) else {
            return None;
        };
        if !segment.short {
            return None;
        }
        let phrase = reading
            .near_miss(&heard.text)
            .or_else(|| self.trigger_heard_in(reading, segment.bounds))?;
        context.rehear(phrase)
    }

    /// What `answer` heard, unless it is a check answered after its utterance's segment; a
    /// segment's answer marks its utterance's segment answered.
    fn unless_stale(&mut self, context: &JobContext, answer: Answer) -> Option<Heard> {
        let stale = context.is_check()
            && context
                .utterance()
                .is_some_and(|u| self.segment_answered(u));
        if let JobContext::Segment { utterance, .. } = context {
            self.mark(*utterance, |r| r.segment_answered = true);
        }
        if stale {
            log::debug!("job={} answered after its segment: dropped", answer.id);
        }
        answer.heard.filter(|_| !stale)
    }

    /// `events` of job `n` as the second hearings asked early leave them: a second hearing's
    /// answer waits for its segment's, and a segment's answer takes the one heard early.
    fn with_early_hearing(
        &mut self,
        n: u64,
        is_segment: bool,
        is_rehear: bool,
        mut events: Vec<ServerEvent>,
    ) -> (Vec<ServerEvent>, EarlyHearing) {
        if is_rehear {
            events = self.hearings.early.rehear_answered(n, events);
        }
        let mut early = EarlyHearing::default();
        if !is_segment {
            return (events, early);
        }
        self.hearings.early.pending.retain(|(job, _)| *job != n);
        let Some(answer) = self.hearings.early.segment_answered(n, !events.is_empty()) else {
            return (events, early);
        };
        early.heard = true;
        match answer {
            EarlyAnswer::Heard(heard) => events = heard,
            EarlyAnswer::Pending(again) => {
                // The segment's events wait for the second hearing's answer.
                self.jobs.outbox.transfer(n, again);
                early.pending = true;
            }
            EarlyAnswer::Dropped => {}
        }
        (events, early)
    }

    /// Has the tail of a segment answered by job `n` heard again; the segment's events wait
    /// for the answer about its tail.
    fn hear_tail(&mut self, n: u64, tail: Option<(u64, usize, usize)>) {
        let Some((utterance, from, to)) = tail else {
            return;
        };
        if let Some(job) = self.ask(
            JobKind::Tail,
            from,
            to,
            JobContext::Tail { utterance, from },
        ) {
            self.jobs.outbox.transfer(n, job);
        }
    }

    /// Resolves job `id` of `utterance` with `events`, a check's as checks resolve.
    fn resolve_answer(
        &mut self,
        id: JobId,
        (utterance, is_check): (Option<u64>, bool),
        timing: Timing,
        events: Vec<ServerEvent>,
    ) -> Vec<ServerEvent> {
        // The speech recognition heard the whole utterance as this command: the keyword
        // spotter's phrases of other commands in it are mishearings, and would make the
        // utterance no command.
        if let Some(ServerEvent::Phrase { phrase, .. }) = events.first() {
            self.jobs
                .outbox
                .keep_only_phrase(id.n, phrase, Some(timing));
        }
        if !events.is_empty() {
            log::info!("job={id} → {events:?}");
        }
        if let Some(utterance) = utterance {
            self.mark(utterance, |r| r.jobs -= 1);
            self.forget_if_done(utterance);
        }
        self.jobs.outbox.resolve(id.n, is_check, events)
    }

    /// The event `heard` makes of the confirmation asked with `context` when the stream had
    /// come to `asked_at`, read with `reading`.
    fn apply_heard(
        &mut self,
        reading: &PhraseMatcher,
        heard: Heard,
        context: JobContext,
        asked_at: usize,
    ) -> Option<ServerEvent> {
        let clip = |from| HeardClip { heard: &heard, from, end: Some(asked_at) };
        let whole = |from| HeardClip { heard: &heard, from, end: None };
        match context {
            JobContext::Head { utterance, from, early: false } => {
                self.place(clip(from), utterance, |w| head_command(reading, w))
            }
            // "start" heard alone may be cut off before the word after it, as the first
            // spotted check may hear it: "start stop" starts nothing.
            JobContext::Head { utterance, from, early: true } => {
                self.place(clip(from), utterance, |w| {
                    spotted_command(reading, w, true, Round::First)
                })
            }
            JobContext::Spotted { utterance, from, at_head, round, said_end } => {
                let found = self.place(clip(from), utterance, |w| {
                    spotted_command(reading, w, at_head, round)
                });
                // The first check may come before the command ends: "start" alone at the
                // head, or the second "restart" of "restart restart" cut off by the clip.
                if found.is_none() && round == Round::First {
                    self.jobs.found_nothing(said_end, asked_at, utterance);
                }
                found
            }
            JobContext::Window { utterance, from } => {
                let found = self.place(whole(from), utterance, |w| reading.anywhere(w));
                self.mark(utterance, |r| r.last_window = Some((heard, from)));
                found
            }
            JobContext::Segment { utterance, segment } => {
                self.apply_segment(reading, &heard, utterance, segment)
            }
            JobContext::Unheld { spotted, from } => {
                self.apply_unheld(reading, &heard, spotted, from)
            }
            // Its answer waits for the segment it may stand for.
            JobContext::HalfPause { .. } => None,
            JobContext::Tail { utterance, from } => {
                self.place_anywhere_or_at_end(reading, whole(from), utterance)
            }
            JobContext::Rehear { bounds, origin, phrase, .. } => {
                self.apply_rehear(reading, &heard, (bounds, origin), phrase)
            }
        }
    }

    /// The selected phrase a trigger spotted within `bounds` serves: the keyword spotter heard
    /// it where the speech recognition service heard another word ("restart" as "Restocked.").
    pub(super) fn trigger_heard_in(
        &self,
        reading: &PhraseMatcher,
        bounds: Segment,
    ) -> Option<String> {
        self.hearings
            .heard_triggers
            .iter()
            .filter(|t| t.start < bounds.end && t.end > bounds.start)
            .filter_map(|t| keywords::served_by(&t.phrase))
            .find(|phrase| reading.selects(phrase))
            .map(str::to_owned)
    }
}
