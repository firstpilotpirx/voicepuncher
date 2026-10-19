//! The confirmations of a session: what each asks the speech recognition service, and what
//! its answer makes of the utterance once it arrives.

use std::time::Instant;

use crate::confirm::{Heard, TimedWord};
use crate::keywords;
use crate::matcher::{Found, PhraseMatcher, Placement};
use crate::orderer::{Segment, Spotted};
use crate::protocol::{ServerEvent, Timing};
use crate::worker::{Answer, Job, JobId, JobKind};

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
        let from = if short {
            self.pre_rolled(start)
        } else {
            end.saturating_sub(samples_in(WINDOW_SECS))
        };
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
        match self.ask(
            JobKind::Segment,
            from,
            to,
            JobContext::Segment { utterance, segment },
        ) {
            Some(job) => {
                self.outbox.hold(job, events);
                Vec::new()
            }
            None => self.outbox.send(events),
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

    fn apply(&mut self, answer: Answer) -> Vec<ServerEvent> {
        let n = answer.id.n;
        self.checks.answered(n);
        let Some(asked) = self.jobs.remove(&n) else {
            return Vec::new();
        };
        let timing = asked.timing();
        let asked_at = asked.asked_at;
        let context = asked.context;
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
        let rehear = near_miss.and_then(|phrase| context.rehear(phrase));
        let stale = is_check && utterance.is_some_and(|u| self.segment_answered(u));
        if let JobContext::Segment { utterance, .. } = &context {
            self.mark(*utterance, |r| r.segment_answered = true);
        }
        let heard = answer.heard.filter(|_| !stale);
        if stale {
            log::debug!("job={} answered after its segment: dropped", answer.id);
        }
        let events: Vec<ServerEvent> = match heard {
            Some(heard) => self.apply_heard(heard, context, asked_at),
            None => Vec::new(),
        }
        .into_iter()
        .map(|event| timed(event, timing))
        .collect();
        let heard_again = match rehear {
            Some(rehear) if events.is_empty() => self.hear_again(n, rehear),
            _ => false,
        };
        if short_segment.is_some() {
            self.misheard = short_segment
                .filter(|_| events.is_empty() && !heard_again)
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
            } => self.place(&heard, from, utterance, head_command),
            // "start" heard alone may be cut off before the word after it, as the first
            // spotted check may hear it: "start stop" starts nothing.
            JobContext::Head {
                utterance,
                from,
                early: true,
            } => self.place(&heard, from, utterance, |m, w| {
                spotted_command(m, w, true, Round::First)
            }),
            JobContext::Spotted {
                utterance,
                from,
                at_head,
                round,
                said_end,
            } => {
                let found = self.place(&heard, from, utterance, |m, w| {
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
                let found = self.place(&heard, from, utterance, |m, w| m.anywhere(w));
                self.mark(utterance, |r| r.last_window = Some((heard, from)));
                found
            }
            JobContext::Segment { utterance, segment } => {
                self.apply_segment(heard, utterance, segment)
            }
            JobContext::Unheld { spotted, from } => self.apply_unheld(&heard, spotted, from),
            JobContext::Tail { utterance, from } => self
                .place(&heard, from, utterance, |m, w| m.anywhere(w))
                .or_else(|| self.place(&heard, from, utterance, |m, w| m.at_end(w))),
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

    /// Hears a short segment heard as a near miss again, through its steady clip; the
    /// segment's events wait for that answer. False when it cannot be asked.
    fn hear_again(&mut self, job: u64, rehear: Rehear) -> bool {
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
        match self.ask_clip(JobKind::Rehear, clip.samples, origin, to, context) {
            Some(again) => {
                self.outbox.transfer(job, again);
                true
            }
            None => false,
        }
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
        self.place(&heard, from, utterance, |m, w| m.anywhere(w))
            .or_else(|| self.place(&heard, from, utterance, |m, w| m.at_end(w)))
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
        find: impl Fn(&PhraseMatcher, &[&str]) -> Option<Found>,
    ) -> Option<ServerEvent> {
        let found = find(&self.matcher, &heard.word_texts())?;
        let at =
            |word: usize, time: fn(&TimedWord) -> f32| from + samples_in(time(&heard.words[word]));
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
        let end = at(found.last, |w| w.end);
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
        let event = self.place(&heard, from, utterance, |m, w| m.at_end(w))?;
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
