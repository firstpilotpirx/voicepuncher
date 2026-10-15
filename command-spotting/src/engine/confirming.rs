//! The confirmations of a session: what each asks the speech recognition service, and what
//! its answer makes of the utterance once it arrives.

use crate::confirm::Heard;
use crate::matcher::{Found, PhraseMatcher, Placement};
use crate::orderer::{Segment, Spotted};
use crate::protocol::ServerEvent;
use crate::worker::{Answer, Job, JobId, JobKind};

use super::pause::first_sound;
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
    Head {
        utterance: u64,
        from: usize,
    },
    Window {
        utterance: u64,
        from: usize,
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
}

/// A closed speech segment by sample position, and the audio sent to confirm it.
pub(super) struct SegmentSpan {
    end: usize,
    bounds: Segment,
    short: bool,
    from: usize,
    to: usize,
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
        answers.into_iter().flat_map(|a| self.apply(a)).collect()
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
        let n = self.next_job;
        self.next_job += 1;
        let audio = self.recent.between(from, to);
        if let Some(utterance) = context.utterance() {
            self.mark(utterance, |r| r.jobs += 1);
        }
        self.outbox.asked(n, kind.is_check());
        log::debug!(
            "job={} {kind:?} asks for {:.2}–{:.2}s",
            JobId { conn: self.conn, n },
            seconds(from),
            seconds(to)
        );
        if let Some(worker) = self.worker.as_ref() {
            worker.submit(Job {
                id: JobId { conn: self.conn, n },
                kind,
                audio,
            });
        }
        self.jobs.insert(n, context);
        Some(n)
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
        let Some(context) = self.jobs.remove(&n) else {
            return Vec::new();
        };
        let utterance = context.utterance();
        let is_check = context.is_check();
        let tail = context.tail_check();
        let events = match answer.heard {
            Some(heard) => self.apply_heard(heard, context),
            None => Vec::new(),
        };
        if let (true, Some((utterance, from, to))) = (events.is_empty(), tail) {
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
            self.outbox.keep_only_phrase(n, phrase);
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

    fn apply_heard(&mut self, heard: Heard, context: JobContext) -> Vec<ServerEvent> {
        let event = match context {
            JobContext::Head { utterance, from } => self.place(&heard, from, utterance, |m, w| {
                // The utterance goes on: a "start" heard alone so far has speech after it.
                m.start_at_head(w).or_else(|| m.start_so_far(w))
            }),
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
        };
        event.into_iter().collect()
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
        })
    }

    /// The command `find` sees in `heard` (audio sent from sample `from`), as an event
    /// placed in the stream; none when the utterance already reported it.
    fn place(
        &mut self,
        heard: &Heard,
        from: usize,
        utterance: u64,
        find: impl Fn(&PhraseMatcher, &[&str]) -> Option<Found>,
    ) -> Option<ServerEvent> {
        let found = find(&self.matcher, &heard.word_texts())?;
        let (first, last) = (&heard.words[found.first], &heard.words[found.last]);
        let start = from + samples_in(first.start);
        let end = from + samples_in(last.end);
        if let Some(running) = self.utterances.get_mut(&utterance) {
            if start < running.reported_until {
                return None;
            }
            running.reported_until = end;
        }
        log::info!(
            "conn={} {:?} {:?}",
            self.conn,
            found.placement,
            found.commands
        );
        Some(ServerEvent::Command {
            commands: found.commands,
            start: seconds(start),
            end: seconds(end),
            placement: found.placement,
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

impl JobContext {
    fn utterance(&self) -> Option<u64> {
        match self {
            JobContext::Head { utterance, .. }
            | JobContext::Window { utterance, .. }
            | JobContext::Segment { utterance, .. }
            | JobContext::Tail { utterance, .. } => Some(*utterance),
            JobContext::Unheld { .. } => None,
        }
    }

    fn is_check(&self) -> bool {
        matches!(self, JobContext::Head { .. } | JobContext::Window { .. })
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
