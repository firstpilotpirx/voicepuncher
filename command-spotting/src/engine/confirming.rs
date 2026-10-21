//! The confirmations of a session: what each asks the speech recognition service, and what
//! its answer makes of the utterance once it arrives.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::confirm::{Heard, TimedWord};
use crate::keywords;
use crate::matcher::{Found, PhraseMatcher, Placement};
use crate::orderer::{Segment, Spotted};
use crate::protocol::{ServerEvent, Timing};
use crate::worker::{Answer, Job, JobId, JobKind};

use super::STARTED_EARLY;

use super::clip::steady_clip;
use super::pause::first_sound;
use super::queued::QueuedCheck;
use super::spotted::{Round, CHECKED_WHEN_SPOTTED};
use super::{
    samples_in, seconds, Session, FIRST_WINDOW_SECS, POST_ROLL_SECS, PRE_ROLL_SECS,
    TAIL_SLACK_SECS, WINDOW_SECS,
};

/// How much of an utterance's end is heard again when a quiet pause closed it and no command
/// was heard in it: in the whole utterance the speech recognition may join a command to the
/// words before it ("at ten enter" heard as "at 10am").
const TAIL_CHECK_SECS: f32 = 1.5;
/// How many deadlines `settle` waits for one answer: a segment's confirmation may be tried
/// twice, and each attempt may take its deadline.
const SETTLE_DEADLINES: u32 = 3;
/// Speech heard this long after a clip that ends in "please" is no longer the word's rest.
const CUT_WORD_SECS: f32 = 0.25;

/// What a confirmation asked for, kept until its answer is applied.
pub(super) enum JobContext {
    /// A check of the utterance's start; `early`: asked before the start could be heard
    /// alone, so only a command opening speech counts.
    Head {
        utterance: u64,
        from: usize,
        early: bool,
    },
    Window {
        utterance: u64,
        from: usize,
    },
    /// A check asked because the keyword spotter heard "start" or "restart" ending at sample
    /// `said_end`; `at_head`: its audio starts where the utterance did.
    Spotted {
        utterance: u64,
        from: usize,
        at_head: bool,
        round: Round,
        said_end: usize,
    },
    Segment {
        utterance: u64,
        segment: SegmentSpan,
    },
    Unheld {
        spotted: Spotted,
        from: usize,
    },
    Tail {
        utterance: u64,
        from: usize,
    },
    /// A short utterance in standby heard at half its pause, before its segment closed.
    HalfPause {
        utterance: u64,
    },
    /// A short segment heard as a near miss of `phrase`, heard again through a clip whose
    /// first sample stands for stream position `origin`.
    Rehear {
        utterance: u64,
        bounds: Segment,
        origin: usize,
        phrase: String,
    },
}

/// A confirmation asked for: what for, how far the stream had come, and when.
pub(super) struct AskedJob {
    context: JobContext,
    asked_at: usize,
    asked: Instant,
    /// The phrases listened for when the job was asked.
    matcher: Arc<PhraseMatcher>,
}

impl AskedJob {
    fn timing(&self) -> Timing {
        Timing {
            asked_at: seconds(self.asked_at),
            took_ms: self.asked.elapsed().as_millis() as u64,
        }
    }
}

/// A closed speech segment by sample position, and the audio sent to confirm it.
pub(super) struct SegmentSpan {
    end: usize,
    bounds: Segment,
    short: bool,
    from: usize,
    to: usize,
    /// At or below this loudness a frame of the segment is quiet.
    quiet_level: f32,
    /// A quiet pause closed it, and it ran long enough for its tail to be heard again.
    tail_check: bool,
}

impl Session<'_> {
    /// The events whose confirmations were answered since the last call, without waiting.
    pub(super) fn take_answers(&mut self) -> Vec<ServerEvent> {
        let answers = self
            .worker
            .as_ref()
            .map(|w| w.answers())
            .unwrap_or_default();
        let events = answers.into_iter().flat_map(|a| self.apply(a)).collect();
        self.ask_queued_check();
        events
    }

    /// The events of the confirmations answered since the last call, without new audio.
    pub fn poll(&mut self) -> Vec<ServerEvent> {
        self.take_answers()
    }

    /// The events still waiting for confirmations, once every one is answered or given up.
    pub fn settle(&mut self) -> Vec<ServerEvent> {
        let wait = self.engine.deadline * SETTLE_DEADLINES;
        let mut events = Vec::new();
        while self.outbox.pending() > 0 {
            let Some(answer) = self.worker.as_ref().and_then(|w| w.next_answer(wait)) else {
                break;
            };
            events.extend(self.apply(answer));
            self.ask_queued_check();
        }
        events
    }

    /// Asks for `synced` with `id` once every event for the audio accepted so far is out.
    pub fn sync(&mut self, id: u64) -> Vec<ServerEvent> {
        let mut events = self.take_answers();
        events.extend(self.outbox.sync(id));
        events
    }

    /// Confirmations asked for and not answered yet.
    pub fn pending_confirmations(&self) -> usize {
        self.outbox.pending()
    }

    /// Asks the worker what was said from sample `from` to `to`; none without a worker.
    pub(super) fn ask(
        &mut self,
        kind: JobKind,
        from: usize,
        to: usize,
        context: JobContext,
    ) -> Option<u64> {
        self.worker.as_ref()?;
        let audio = self.recent.between(from, to);
        self.ask_clip(kind, audio, from, to, context)
    }

    /// Asks the worker what was said in `audio`, the stream from sample `from` to `to`.
    fn ask_clip(
        &mut self,
        kind: JobKind,
        audio: Vec<f32>,
        from: usize,
        to: usize,
        context: JobContext,
    ) -> Option<u64> {
        let n = self.reserve(kind, context)?;
        self.submit(n, kind, audio, from, to);
        Some(n)
    }

    /// A job number for a confirmation with `context`, counted as asked from now; none
    /// without a worker.
    fn reserve(&mut self, kind: JobKind, context: JobContext) -> Option<u64> {
        self.worker.as_ref()?;
        let n = self.next_job;
        self.next_job += 1;
        if let Some(utterance) = context.utterance() {
            self.mark(utterance, |r| r.jobs += 1);
        }
        self.outbox.asked(n, kind.is_check());
        self.jobs.insert(
            n,
            AskedJob {
                context,
                asked_at: self.samples_seen,
                asked: Instant::now(),
                matcher: Arc::clone(&self.matcher),
            },
        );
        Some(n)
    }

    /// Sends job `n` to the worker: what was said in `audio`, the stream from `from` to `to`.
    fn submit(&mut self, n: u64, kind: JobKind, audio: Vec<f32>, from: usize, to: usize) {
        log::debug!(
            "job={} {kind:?} asks for {:.2}–{:.2}s",
            JobId { conn: self.conn, n },
            seconds(from),
            seconds(to)
        );
        if let Some(asked) = self.jobs.get_mut(&n) {
            asked.asked = Instant::now();
        }
        if let Some(worker) = self.worker.as_ref() {
            worker.submit(Job {
                id: JobId { conn: self.conn, n },
                kind,
                audio,
            });
        }
    }

    /// Asks a check of a running utterance about the stream from `from` to `to`, or queues it
    /// with that audio while another check waits; a window takes the place of a queued window
    /// of the same utterance.
    pub(super) fn ask_check(&mut self, kind: JobKind, from: usize, to: usize, context: JobContext) {
        if !self.outbox.check_busy() {
            if let Some(n) = self.ask(kind, from, to, context) {
                self.checks.asked(n);
            }
            return;
        }
        let window_of = match context {
            JobContext::Window { utterance, .. } => Some(utterance),
            _ => None,
        };
        let taken_over = window_of.and_then(|u| self.checks.replace_window(u, from, to));
        log::debug!(
            "conn={} {kind:?} for {:.2}–{:.2}s waits its turn: a check still waits",
            self.conn,
            seconds(from),
            seconds(to)
        );
        if let Some(asked) = taken_over.and_then(|n| self.jobs.get_mut(&n)) {
            asked.context = context;
            asked.asked_at = self.samples_seen;
            return;
        }
        if let Some(job) = self.reserve(kind, context) {
            self.checks.push(QueuedCheck {
                job,
                kind,
                from,
                to,
                window_of,
            });
        }
    }

    /// Asks the check whose turn it is, once no other one waits for its answer.
    pub(super) fn ask_queued_check(&mut self) {
        let Some(QueuedCheck {
            job,
            kind,
            from,
            to,
            ..
        }) = self.checks.next()
        else {
            return;
        };
        let audio = self.recent.between(from, to);
        self.submit(job, kind, audio, from, to);
        self.checks.asked(job);
    }

    /// The events of a closed segment: held until its confirmation is answered, or out at
    /// once without a speech recognition service. Its confirmation clip runs past its end
    /// until the post-roll is over or a sound louder than `quiet_level` starts.
    pub(super) fn close_segment(
        &mut self,
        start: usize,
        end: usize,
        bounds: Segment,
        events: Vec<ServerEvent>,
        quiet_level: f32,
        by_pause: bool,
    ) -> Vec<ServerEvent> {
        if self.worker.is_none() {
            return self.outbox.send(events);
        }
        let utterance = self
            .current
            .take()
            .unwrap_or_else(|| self.new_utterance(start, usize::MAX, true));
        self.mark(utterance, |r| {
            r.closed = true;
            r.head_checked = true;
            r.next_window = usize::MAX;
        });
        let short = end - start <= self.engine.max_command_samples;
        // The same quiet run closes it: what was heard at its half stands for it, when that
        // is a command alone or not in yet.
        let half_pause = self.half_pause.take().filter(|h| {
            short
                && match &h.answer {
                    None => true,
                    Some(heard) => heard.as_ref().is_some_and(|h| self.said_alone(h)),
                }
        });
        let own_from = if short {
            self.pre_rolled(start)
        } else {
            end.saturating_sub(samples_in(WINDOW_SECS))
        };
        let from = half_pause.as_ref().map_or(own_from, |h| h.from);
        let post_roll = self.recent.between(end, end + samples_in(POST_ROLL_SECS));
        let to = end + first_sound(&post_roll, quiet_level);
        let segment = SegmentSpan {
            end,
            bounds,
            short,
            from,
            to,
            quiet_level,
            tail_check: by_pause && end - start > samples_in(FIRST_WINDOW_SECS),
        };
        let context = JobContext::Segment { utterance, segment };
        let again = context.rehear(String::new()).filter(|_| short);
        let (job, heard) = match half_pause {
            Some(HalfPauseHearing {
                job, answer: None, ..
            }) => {
                log::debug!("job={job} heard at half the pause stands for its segment");
                if let Some(asked) = self.jobs.get_mut(&job) {
                    asked.context = context;
                }
                self.stand_ins.insert(job, own_from);
                (Some(job), None)
            }
            Some(HalfPauseHearing {
                answer: Some(heard),
                ..
            }) => (self.reserve(JobKind::Segment, context), heard),
            None => (self.ask(JobKind::Segment, from, to, context), None),
        };
        let Some(job) = job else {
            return self.outbox.send(events);
        };
        self.outbox.hold(job, events);
        if let Some(again) = again {
            self.hear_early_when_triggered(job, again);
        }
        match heard {
            // Heard at half the pause: the answer is in already.
            Some(heard) => self.apply(Answer {
                id: JobId {
                    conn: self.conn,
                    n: job,
                },
                heard: Some(heard),
            }),
            None => Vec::new(),
        }
    }

    /// A phrase the keyword spotter found but no speech segment took: the voice activity
    /// detector missed the utterance. It counts when the speech recognition service hears
    /// only the command around it.
    pub(super) fn confirm_unheld(&mut self) {
        if self.worker.is_none() {
            return;
        }
        for spotted in self.orderer.unheld(seconds(self.samples_seen)) {
            let around = samples_in(PRE_ROLL_SECS / 2.0);
            let from = samples_in(spotted.start)
                .saturating_sub(around)
                .max(self.last_segment_end);
            let to = samples_in(spotted.end) + around;
            self.ask(
                JobKind::Unheld,
                from,
                to,
                JobContext::Unheld { spotted, from },
            );
        }
    }

    /// Hears a short utterance of standby at half its pause, whose quiet run began at
    /// `quiet_from`, from its pre-roll to `now`.
    pub(super) fn hear_at_half_pause(&mut self, quiet_from: usize, now: usize) {
        if self.half_pause.is_some() || !self.matcher.selects(STARTED_EARLY) {
            return;
        }
        let Some((utterance, start)) = self
            .current
            .and_then(|id| self.utterances.get(&id).map(|r| (id, r.start)))
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
            self.half_pause = Some(HalfPauseHearing {
                job,
                quiet_from,
                from,
                answer: None,
            });
        }
    }

    /// Drops the hearing at half the pause once a louder frame broke its quiet run: the
    /// utterance goes on, and its segment is heard whole.
    pub(super) fn drop_broken_half_pause_hearing(&mut self) {
        let quiet_run = self.pause.quiet_run_from();
        if let Some(dropped) = self.half_pause.take_if(|h| quiet_run != Some(h.quiet_from)) {
            log::debug!(
                "job={} heard at half the pause: dropped, the utterance goes on",
                dropped.job
            );
        }
    }

    fn apply(&mut self, answer: Answer) -> Vec<ServerEvent> {
        let n = answer.id.n;
        self.checks.answered(n);
        let Some(asked) = self.jobs.remove(&n) else {
            return Vec::new();
        };
        let asked = match self.hear_whole_instead(n, &answer, asked) {
            Ok(asked) => asked,
            Err(events) => return events,
        };
        // An answer that comes back after the phrases changed reads with both sets: the
        // words were said under the job's phrases, or are said now under the current ones.
        let reading = if Arc::ptr_eq(&asked.matcher, &self.matcher) {
            Arc::clone(&self.matcher)
        } else {
            Arc::new(asked.matcher.combined_with(&self.matcher))
        };
        let current = std::mem::replace(&mut self.matcher, reading);
        let events = self.apply_answer(answer, asked);
        self.matcher = current;
        events
    }

    /// Whether `heard` says a selected phrase alone: a hearing at half the pause that heard
    /// anything else may have heard its clip cut short wrong ("okay, paste" as "okay, based").
    fn said_alone(&self, heard: &Heard) -> bool {
        self.matcher.phrase_in(&heard.text).is_some()
    }

    /// Has the segment of job `n`, which a hearing at half the pause stood for, heard whole
    /// through its own clip when `answer` holds no phrase alone; the job is then resolved, and
    /// its events wait for that hearing. Else gives `asked` back.
    fn hear_whole_instead(
        &mut self,
        n: u64,
        answer: &Answer,
        asked: AskedJob,
    ) -> Result<AskedJob, Vec<ServerEvent>> {
        let Some(own_from) = self.stand_ins.remove(&n) else {
            return Ok(asked);
        };
        let (Some(heard), JobContext::Segment { utterance, segment }) =
            (&answer.heard, &asked.context)
        else {
            return Ok(asked);
        };
        if self.said_alone(heard) {
            return Ok(asked);
        }
        let (utterance, segment) = (
            *utterance,
            SegmentSpan {
                from: own_from,
                ..*segment
            },
        );
        let to = segment.to;
        let context = JobContext::Segment { utterance, segment };
        let Some(whole) = self.ask(JobKind::Segment, own_from, to, context) else {
            return Ok(asked);
        };
        log::debug!("job={n} heard at half the pause held no command alone: job {whole} hears the segment whole");
        self.outbox.transfer(n, whole);
        self.early.retarget(n, whole);
        self.mark(utterance, |r| r.jobs -= 1);
        Err(self.outbox.resolve(n, false, Vec::new()))
    }

    /// Applies `answer` to the job it answers, reading it with `self.matcher`.
    fn apply_answer(&mut self, answer: Answer, asked: AskedJob) -> Vec<ServerEvent> {
        let n = answer.id.n;
        let timing = asked.timing();
        let asked_at = asked.asked_at;
        let context = asked.context;
        if matches!(context, JobContext::HalfPause { .. }) {
            if let Some(half_pause) = self.half_pause.as_mut().filter(|h| h.job == n) {
                half_pause.answer = Some(answer.heard.clone());
            }
        }
        let utterance = context.utterance();
        let is_check = context.is_check();
        let tail = context.tail_check();
        let near_miss = match (&context, &answer.heard) {
            (JobContext::Segment { segment, .. }, Some(heard)) if segment.short => self
                .matcher
                .near_miss(&heard.text)
                .or_else(|| self.trigger_heard_in(segment.bounds)),
            _ => None,
        };
        let short_segment = match &context {
            JobContext::Segment { segment, .. } if segment.short => context.rehear(String::new()),
            _ => None,
        };
        let mut rehear = near_miss.and_then(|phrase| context.rehear(phrase));
        let is_segment = matches!(context, JobContext::Segment { .. });
        let is_rehear = matches!(context, JobContext::Rehear { .. });
        let stale = is_check && utterance.is_some_and(|u| self.segment_answered(u));
        if let JobContext::Segment { utterance, .. } = &context {
            self.mark(*utterance, |r| r.segment_answered = true);
        }
        let heard = answer.heard.filter(|_| !stale);
        if stale {
            log::debug!("job={} answered after its segment: dropped", answer.id);
        }
        let mut events: Vec<ServerEvent> = match heard {
            Some(heard) => self.apply_heard(heard, context, asked_at),
            None => Vec::new(),
        }
        .into_iter()
        .map(|event| timed(event, timing))
        .collect();
        if is_rehear {
            events = self.early.rehear_answered(n, events);
        }
        let mut heard_early = false;
        let mut early_pending = false;
        if is_segment {
            self.early.pending.retain(|(job, _)| *job != n);
            if let Some(early) = self.early.segment_answered(n, !events.is_empty()) {
                heard_early = true;
                rehear = None;
                match early {
                    EarlyAnswer::Heard(heard) => events = heard,
                    EarlyAnswer::Pending(again) => {
                        // The segment's events wait for the second hearing's answer.
                        self.outbox.transfer(n, again);
                        early_pending = true;
                    }
                    EarlyAnswer::Dropped => {}
                }
            }
        }
        let heard_again = early_pending
            || match rehear {
                Some(rehear) if events.is_empty() => self.hear_again(n, rehear),
                _ => false,
            };
        if short_segment.is_some() {
            self.misheard = short_segment
                .filter(|_| events.is_empty() && !heard_again && !heard_early)
                .map(|segment| (n, segment));
        }
        if let (true, false, Some((utterance, from, to))) = (events.is_empty(), heard_again, tail) {
            // The segment's events wait for the answer about its tail.
            if let Some(job) = self.ask(
                JobKind::Tail,
                from,
                to,
                JobContext::Tail { utterance, from },
            ) {
                self.outbox.transfer(n, job);
            }
        }
        // The speech recognition heard the whole utterance as this command: the keyword
        // spotter's phrases of other commands in it are mishearings, and would make the
        // utterance no command.
        if let Some(ServerEvent::Phrase { phrase, .. }) = events.first() {
            self.outbox.keep_only_phrase(n, phrase, Some(timing));
        }
        if !events.is_empty() {
            log::info!("job={} → {events:?}", answer.id);
        }
        if let Some(utterance) = utterance {
            self.mark(utterance, |r| r.jobs -= 1);
            self.forget_if_done(utterance);
        }
        self.outbox.resolve(n, is_check, events)
    }

    /// The segment of `utterance` was heard: it decides what the utterance said.
    fn segment_answered(&self, utterance: u64) -> bool {
        self.utterances
            .get(&utterance)
            .is_some_and(|r| r.segment_answered)
    }

    /// The events `heard` makes of the confirmation asked with `context` when the stream
    /// had come to `asked_at`.
    fn apply_heard(
        &mut self,
        heard: Heard,
        context: JobContext,
        asked_at: usize,
    ) -> Vec<ServerEvent> {
        let event = match context {
            JobContext::Head {
                utterance,
                from,
                early: false,
            } => self.place(&heard, from, utterance, asked_at, head_command),
            // "start" heard alone may be cut off before the word after it, as the first
            // spotted check may hear it: "start stop" starts nothing.
            JobContext::Head {
                utterance,
                from,
                early: true,
            } => self.place(&heard, from, utterance, asked_at, |m, w| {
                spotted_command(m, w, true, Round::First)
            }),
            JobContext::Spotted {
                utterance,
                from,
                at_head,
                round,
                said_end,
            } => {
                let found = self.place(&heard, from, utterance, asked_at, |m, w| {
                    spotted_command(m, w, at_head, round)
                });
                // The first check may come before the command ends: "start" alone at the
                // head, or the second "restart" of "restart restart" cut off by the clip.
                if found.is_none() && round == Round::First {
                    self.spotted_checks
                        .found_nothing(said_end, asked_at, utterance);
                }
                found
            }
            JobContext::Window { utterance, from } => {
                let found = self.place(&heard, from, utterance, usize::MAX, |m, w| m.anywhere(w));
                self.mark(utterance, |r| r.last_window = Some((heard, from)));
                found
            }
            JobContext::Segment { utterance, segment } => {
                self.apply_segment(heard, utterance, segment)
            }
            JobContext::Unheld { spotted, from } => self.apply_unheld(&heard, spotted, from),
            // Its answer waits for the segment it may stand for.
            JobContext::HalfPause { .. } => None,
            JobContext::Tail { utterance, from } => self
                .place(&heard, from, utterance, usize::MAX, |m, w| m.anywhere(w))
                .or_else(|| self.place(&heard, from, utterance, usize::MAX, |m, w| m.at_end(w))),
            JobContext::Rehear {
                bounds,
                origin,
                phrase,
                ..
            } => self.apply_rehear(&heard, bounds, origin, phrase),
        };
        event.into_iter().collect()
    }

    /// The selected phrase a trigger spotted within `bounds` serves: the keyword spotter heard
    /// it where the speech recognition service heard another word ("restart" as "Restocked.").
    fn trigger_heard_in(&self, bounds: Segment) -> Option<String> {
        self.heard_triggers
            .iter()
            .filter(|t| t.start < bounds.end && t.end > bounds.start)
            .filter_map(|t| keywords::served_by(&t.phrase))
            .find(|phrase| self.matcher.selects(phrase))
            .map(str::to_owned)
    }

    /// Hears the last short segment heard as no command again when `trigger`, reported
    /// after that answer, lies in it.
    pub(super) fn hear_misheard_again(&mut self, trigger: &Spotted) {
        let Some(phrase) =
            keywords::served_by(&trigger.phrase).filter(|phrase| self.matcher.selects(phrase))
        else {
            return;
        };
        let Some((job, segment)) = self.misheard.take_if(|(_, segment)| {
            trigger.start < segment.bounds.end && trigger.end > segment.bounds.start
        }) else {
            return;
        };
        let phrase = phrase.to_owned();
        self.hear_again(job, Rehear { phrase, ..segment });
    }

    /// Hears the short segment of job `job`, still being heard, again at once when a trigger
    /// already lies in it; else when one is spotted in it before its answer comes back.
    fn hear_early_when_triggered(&mut self, job: u64, segment: Rehear) {
        match self.trigger_heard_in(segment.bounds) {
            Some(phrase) => self.hear_early(job, Rehear { phrase, ..segment }),
            None => self.early.pending.push((job, segment)),
        }
    }

    /// Hears a short segment still being heard again at once when `trigger` lies in it.
    pub(super) fn hear_pending_again(&mut self, trigger: &Spotted) {
        let Some(phrase) =
            keywords::served_by(&trigger.phrase).filter(|phrase| self.matcher.selects(phrase))
        else {
            return;
        };
        let Some(at) = self.early.pending.iter().position(|(_, segment)| {
            trigger.start < segment.bounds.end && trigger.end > segment.bounds.start
        }) else {
            return;
        };
        let (job, segment) = self.early.pending.remove(at);
        let phrase = phrase.to_owned();
        self.hear_early(job, Rehear { phrase, ..segment });
    }

    /// Asks the second hearing of the segment of job `job` beside its first one.
    fn hear_early(&mut self, job: u64, rehear: Rehear) {
        if let Some(again) = self.ask_rehear(rehear) {
            log::debug!(
                "conn={} job {job} heard again at once by job {again}",
                self.conn
            );
            self.early
                .asked
                .insert(again, Early::Waiting { segment: job });
        }
    }

    /// Hears a short segment heard as a near miss again, through its steady clip; the
    /// segment's events wait for that answer. False when it cannot be asked.
    fn hear_again(&mut self, job: u64, rehear: Rehear) -> bool {
        match self.ask_rehear(rehear) {
            Some(again) => {
                self.outbox.transfer(job, again);
                true
            }
            None => false,
        }
    }

    /// Asks the second hearing of a short segment, through its steady clip.
    fn ask_rehear(&mut self, rehear: Rehear) -> Option<u64> {
        let Rehear {
            utterance,
            bounds,
            from,
            to,
            quiet_level,
            phrase,
        } = rehear;
        let audio = self.recent.between(from, to);
        let clip = steady_clip(&audio, from, quiet_level);
        log::debug!(
            "conn={} near miss of {phrase:?}: heard again from {:.2}s, gain {:.1}",
            self.conn,
            seconds(clip.origin),
            clip.gain
        );
        let origin = clip.origin;
        let context = JobContext::Rehear {
            utterance,
            bounds,
            origin,
            phrase,
        };
        self.ask_clip(JobKind::Rehear, clip.samples, origin, to, context)
    }

    /// The near-missed command, when the second hearing says it as a whole.
    fn apply_rehear(
        &self,
        heard: &Heard,
        bounds: Segment,
        origin: usize,
        phrase: String,
    ) -> Option<ServerEvent> {
        log::debug!("conn={} heard again: {:?}", self.conn, heard.text);
        if self.matcher.phrase_in(&heard.text).as_deref() != Some(phrase.as_str()) {
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

    /// The command the segment says: the whole utterance, when it is short enough for a
    /// command; else a doubled or chained command in its last seconds the running search
    /// did not report yet, or a command that ends it.
    fn apply_segment(
        &mut self,
        heard: Heard,
        utterance: u64,
        segment: SegmentSpan,
    ) -> Option<ServerEvent> {
        let SegmentSpan {
            end,
            bounds,
            short,
            from,
            ..
        } = segment;
        if short {
            if let Some(phrase) = self.matcher.phrase_in(&heard.text) {
                log::info!("conn={} utterance → {phrase:?}", self.conn);
                // The detector notices speech up to 0.7 s late: the first word's time is
                // where the command starts, so a recording cut there keeps none of it.
                let start = heard
                    .words
                    .first()
                    .map_or(bounds.start, |w| seconds(from + samples_in(w.start)));
                return Some(ServerEvent::Phrase {
                    phrase,
                    start: start.min(bounds.start),
                    end: bounds.end,
                    timing: None,
                });
            }
        }
        let last_window = self
            .utterances
            .get_mut(&utterance)
            .and_then(|r| r.last_window.take());
        self.place(&heard, from, utterance, usize::MAX, |m, w| m.anywhere(w))
            .or_else(|| self.place(&heard, from, utterance, usize::MAX, |m, w| m.at_end(w)))
            .or_else(|| self.tail_heard_while_running(last_window, end, utterance))
    }

    fn apply_unheld(
        &mut self,
        heard: &Heard,
        spotted: Spotted,
        from: usize,
    ) -> Option<ServerEvent> {
        let phrase = self.matcher.phrase_in(&heard.text)?;
        let (start, end) = match (heard.words.first(), heard.words.last()) {
            (Some(first), Some(last)) => (
                seconds(from + samples_in(first.start)),
                seconds(from + samples_in(last.end)),
            ),
            _ => (spotted.start, spotted.end),
        };
        log::info!("conn={} missed utterance → {phrase:?}", self.conn);
        Some(ServerEvent::Command {
            commands: vec![phrase],
            start,
            end,
            placement: Placement::Tail,
            timing: None,
        })
    }

    /// The commands `find` sees in `heard` (audio sent from sample `from`), as an event
    /// placed in the stream; of a chain the utterance already reported in part, only the
    /// commands after what it reported ("start" after a check heard "cancel enter"), none
    /// when nothing is left.
    fn place(
        &mut self,
        heard: &Heard,
        from: usize,
        utterance: u64,
        clip_end: usize,
        find: impl Fn(&PhraseMatcher, &[&str]) -> Option<Found>,
    ) -> Option<ServerEvent> {
        let words = heard.word_texts();
        let found = find(&self.matcher, &words)?;
        let at =
            |word: usize, time: fn(&TimedWord) -> f32| from + samples_in(time(&heard.words[word]));
        // A start closed by "please" with nothing after it ends where the speech did: a clip
        // cut while "please" is still said times the word too soon, and the rest of it would
        // open the dictation. Without a gap after the word yet, it waits for a later hearing
        // while the speech may still be the word's; speech going on longer than that is the
        // dictation, said at once.
        let speech_end = if self.matcher.start_closed_by_please(&words, &found) {
            let middle = (at(found.last, |w| w.start) + at(found.last, |w| w.end)) / 2;
            let went_on = clip_end != usize::MAX
                && self.pause.heard_until() >= clip_end.saturating_add(samples_in(CUT_WORD_SECS));
            match self.pause.gap_after(middle) {
                Some(gap) => Some(gap),
                None if clip_end == usize::MAX || went_on => None,
                None => return None,
            }
        } else {
            None
        };
        let reported_until = self
            .utterances
            .get(&utterance)
            .map_or(0, |r| r.reported_until);
        // Word times differ by about 0.1 s between two clips of the same speech: a command
        // lies after the report when the middle of its first word does; a doubled command
        // whose first word was reported alone ("start" heard before "start start") is not.
        let after_report = |&(first, _): &(usize, usize)| {
            (at(first, |w| w.start) + at(first, |w| w.end)) / 2 > reported_until
        };
        let kept = if at(found.first, |w| w.start) < reported_until {
            found.spans.iter().position(after_report)?
        } else {
            0
        };
        let start = at(found.spans[kept].0, |w| w.start);
        let end = at(found.last, |w| w.end).max(speech_end.unwrap_or(0));
        if let Some(running) = self.utterances.get_mut(&utterance) {
            running.reported_until = end;
        }
        let commands = found.commands[kept..].to_vec();
        log::info!("conn={} {:?} {:?}", self.conn, found.placement, commands);
        Some(ServerEvent::Command {
            commands,
            start: seconds(start),
            end: seconds(end),
            placement: found.placement,
            timing: None,
        })
    }

    /// A command that ended the last window heard while the utterance ran, when it also
    /// ends the utterance: the closing window, cut elsewhere, may hear its words wrong
    /// ("at ten enter" as "at 10am").
    fn tail_heard_while_running(
        &mut self,
        last_window: Option<(Heard, usize)>,
        end: usize,
        utterance: u64,
    ) -> Option<ServerEvent> {
        let (heard, from) = last_window?;
        let event = self.place(&heard, from, utterance, usize::MAX, |m, w| m.at_end(w))?;
        match event {
            ServerEvent::Command { end: said_end, .. }
                if said_end >= seconds(end.saturating_sub(samples_in(TAIL_SLACK_SECS))) =>
            {
                Some(event)
            }
            _ => None,
        }
    }
}

/// The hearing of a short utterance asked at half its pause.
pub(super) struct HalfPauseHearing {
    job: u64,
    /// Where the quiet run began: only this run closing the utterance keeps the hearing.
    quiet_from: usize,
    /// Where its audio starts.
    from: usize,
    /// What was heard, once answered.
    answer: Option<Option<Heard>>,
}

/// The second hearings of short segments asked beside their first hearing, so a trigger in a
/// short segment costs one hearing's time, not two in a row.
#[derive(Default)]
pub(super) struct EarlyRehears {
    /// Short segments whose first hearing is pending, by job: a trigger spotted in one has it
    /// heard again at once.
    pending: Vec<(u64, Rehear)>,
    /// The second hearings asked early, by job.
    asked: HashMap<u64, Early>,
}

/// A second hearing asked beside its segment's first one.
enum Early {
    /// Neither hearing answered yet, or only this one is pending.
    Waiting { segment: u64 },
    /// It answered first, with these events; they stand when the first hearing holds none.
    Answered {
        segment: u64,
        events: Vec<ServerEvent>,
    },
    /// The first hearing held a command: its answer is dropped.
    Dropped,
}

/// What a segment's first hearing makes of the second one asked beside it.
enum EarlyAnswer {
    /// Its answer came first: these events stand for the segment.
    Heard(Vec<ServerEvent>),
    /// It is still pending, as job `.0`: the segment waits for it.
    Pending(u64),
    /// The first hearing held a command: the second one is dropped.
    Dropped,
}

impl EarlyRehears {
    /// The second hearings of segment job `from` belong to segment job `to` now.
    fn retarget(&mut self, from: u64, to: u64) {
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
    fn rehear_answered(&mut self, job: u64, events: Vec<ServerEvent>) -> Vec<ServerEvent> {
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
    fn segment_answered(&mut self, segment: u64, held_command: bool) -> Option<EarlyAnswer> {
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
pub(super) struct Rehear {
    utterance: u64,
    bounds: Segment,
    from: usize,
    to: usize,
    quiet_level: f32,
    phrase: String,
}

impl JobContext {
    fn utterance(&self) -> Option<u64> {
        match self {
            JobContext::Head { utterance, .. }
            | JobContext::Window { utterance, .. }
            | JobContext::Spotted { utterance, .. }
            | JobContext::Segment { utterance, .. }
            | JobContext::Tail { utterance, .. }
            | JobContext::HalfPause { utterance }
            | JobContext::Rehear { utterance, .. } => Some(*utterance),
            JobContext::Unheld { .. } => None,
        }
    }

    fn is_check(&self) -> bool {
        matches!(
            self,
            JobContext::Head { .. } | JobContext::Window { .. } | JobContext::Spotted { .. }
        )
    }

    /// What a second hearing of this segment as a near miss of `phrase` needs.
    fn rehear(&self, phrase: String) -> Option<Rehear> {
        match self {
            JobContext::Segment { utterance, segment } => Some(Rehear {
                utterance: *utterance,
                bounds: segment.bounds,
                from: segment.from,
                to: segment.to,
                quiet_level: segment.quiet_level,
                phrase,
            }),
            _ => None,
        }
    }

    /// The utterance and the audio to hear again when this segment's answer holds no
    /// command.
    fn tail_check(&self) -> Option<(u64, usize, usize)> {
        match self {
            JobContext::Segment { utterance, segment } if segment.tail_check => Some((
                *utterance,
                segment.end.saturating_sub(samples_in(TAIL_CHECK_SECS)),
                segment.to,
            )),
            _ => None,
        }
    }
}

/// The command the start of a running utterance says: a doubled start or a chain ending in
/// one, "start" opening it, or "start" heard alone so far, since the utterance goes on and
/// has speech after it.
fn head_command(matcher: &PhraseMatcher, words: &[&str]) -> Option<Found> {
    starting_anywhere(matcher, words)
        .or_else(|| matcher.start_at_head(words))
        .or_else(|| matcher.start_so_far(words))
}

/// The command a check asked by a spotted "start" or "restart" hears: a doubled command or
/// a chain anywhere, and in audio from the utterance's start also what the head check takes.
fn spotted_command(
    matcher: &PhraseMatcher,
    words: &[&str],
    at_head: bool,
    round: Round,
) -> Option<Found> {
    let found = starting_anywhere(matcher, words);
    if !at_head {
        return found;
    }
    // "start" heard alone may be cut off before the word after it: "start stop" starts nothing.
    found
        .or_else(|| matcher.start_at_head(words))
        .or_else(|| (round == Round::Second).then(|| matcher.start_so_far(words))?)
}

/// A doubled command or a chain in `words` that starts a dictation: other ones are left to
/// the checks of the utterance's last seconds and its segment, which the keyword spotter's
/// phrase of the segment does not repeat.
fn starting_anywhere(matcher: &PhraseMatcher, words: &[&str]) -> Option<Found> {
    matcher.anywhere(words).filter(|found| {
        found
            .commands
            .last()
            .is_some_and(|last| CHECKED_WHEN_SPOTTED.contains(&last.as_str()))
    })
}

/// `event` with the timing of the confirmation that heard it, when it names a command.
fn timed(event: ServerEvent, confirmation: Timing) -> ServerEvent {
    match event {
        ServerEvent::Phrase {
            phrase, start, end, ..
        } => ServerEvent::Phrase {
            phrase,
            start,
            end,
            timing: Some(confirmation),
        },
        ServerEvent::Command {
            commands,
            start,
            end,
            placement,
            ..
        } => ServerEvent::Command {
            commands,
            start,
            end,
            placement,
            timing: Some(confirmation),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::Placement;

    fn standby() -> PhraseMatcher {
        PhraseMatcher::new(&[
            "start dictation".into(),
            "press enter".into(),
            "clear all text".into(),
        ])
    }

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    #[test]
    fn should_take_a_doubled_start_at_the_head_of_a_running_utterance() {
        // Given the start of an utterance heard as "start start" and the dictation after it
        let said = words("Start, start, we meet on Friday");
        // When
        let found = head_command(&standby(), &said);
        // Then
        let found = found.expect("start start is a command");
        assert_eq!(found.commands, ["start dictation"]);
        assert_eq!((found.first, found.last), (0, 1));
    }

    #[test]
    fn should_take_a_chain_ending_in_start_at_the_head_of_a_running_utterance() {
        // Given the start of an utterance heard as "enter start" and the dictation after it
        let said = words("Enter, start, we meet on Friday");
        // When
        let found = head_command(&standby(), &said);
        // Then
        let found = found.expect("enter start is a chain");
        assert_eq!(found.commands, ["press enter", "start dictation"]);
        assert_eq!(found.placement, Placement::Anywhere);
    }

    #[test]
    fn should_leave_doubled_commands_other_than_start_to_the_later_checks() {
        // Given a short utterance heard as a doubled "clear": its segment reports it once
        let said = words("Fine, clear clear");
        // When
        let at_head = head_command(&standby(), &said);
        let spotted = spotted_command(&standby(), &said, false, Round::First);
        // Then
        assert_eq!((at_head, spotted), (None, None));
    }

    #[test]
    fn should_not_take_start_heard_alone_by_the_first_spotted_check() {
        // Given the first check of a spotted "start" heard nothing after it yet
        let said = words("Start.");
        // When
        let found = spotted_command(&standby(), &said, true, Round::First);
        // Then
        assert_eq!(found, None);
    }

    #[test]
    fn should_take_start_please_heard_alone_by_the_first_spotted_check() {
        // Given the first check of a spotted "start" heard a start closed by "please" and
        // nothing after it
        for said in ["Start please.", "Start dictation, please."] {
            // When
            let found = spotted_command(&standby(), &words(said), true, Round::First);
            // Then
            assert_eq!(
                found.map(|f| f.commands),
                Some(vec!["start dictation".to_owned()]),
                "{said}"
            );
        }
    }

    #[test]
    fn should_take_start_heard_alone_by_the_second_spotted_check() {
        // Given the second check of a spotted "start" still heard "start" alone
        let said = words("Start.");
        // When
        let found = spotted_command(&standby(), &said, true, Round::Second);
        // Then
        assert_eq!(
            found.map(|f| f.commands),
            Some(vec!["start dictation".to_owned()])
        );
    }

    #[test]
    fn should_take_start_opening_speech_by_the_first_spotted_check() {
        // Given
        let said = words("Start, we meet on Friday");
        // When
        let found = spotted_command(&standby(), &said, true, Round::First);
        // Then
        assert_eq!(found.map(|f| f.placement), Some(Placement::Head));
    }
}
