//! A closed utterance asked as a whole, and what its answer makes of it.

use crate::confirm::Heard;
use crate::matcher::PhraseMatcher;
use crate::orderer::Segment;
use crate::protocol::ServerEvent;
use crate::worker::{Answer, JobId, JobKind};

use super::super::pause::first_sound;
use super::super::{samples_in, seconds, Session, FIRST_WINDOW_SECS, POST_ROLL_SECS, WINDOW_SECS};

use super::place::HeardClip;
use super::{HalfPauseHearing, JobContext, SegmentSpan};

/// A speech segment the detector closed: samples `start..end` of the stream, `bounds` in
/// seconds, the loudness at or below which a frame of it is quiet, and whether a quiet pause
/// closed it.
#[derive(Clone, Copy)]
pub(in crate::engine) struct ClosedSegment {
    pub(in crate::engine) start: usize,
    pub(in crate::engine) end: usize,
    pub(in crate::engine) bounds: Segment,
    pub(in crate::engine) quiet_level: f32,
    pub(in crate::engine) by_pause: bool,
}

impl Session<'_> {
    /// The events of a closed segment: held until its confirmation is answered, or out at
    /// once without a speech recognition service. Its confirmation clip runs past its end
    /// until the post-roll is over or a sound louder than its quiet level starts.
    pub(in crate::engine) fn close_segment(
        &mut self,
        closed: &ClosedSegment,
        events: Vec<ServerEvent>,
    ) -> Vec<ServerEvent> {
        if self.worker.is_none() {
            return self.jobs.outbox.send(events);
        }
        let utterance = self.close_utterance(closed.start);
        let short = closed.end - closed.start <= self.engine.max_command_samples;
        let half_pause = self.half_pause_standing_in(short);
        let own_from = if short {
            self.pre_rolled(closed.start)
        } else {
            closed.end.saturating_sub(samples_in(WINDOW_SECS))
        };
        let from = half_pause.as_ref().map_or(own_from, |h| h.from);
        let segment = self.segment_span(closed, short, from);
        let span = (segment.from, segment.to);
        let context = JobContext::Segment { utterance, segment };
        let again = context.rehear(String::new()).filter(|_| short);
        let (job, heard) = self.ask_segment(context, half_pause, own_from, span);
        let Some(job) = job else {
            return self.jobs.outbox.send(events);
        };
        self.jobs.outbox.hold(job, events);
        if let Some(again) = again {
            self.hear_early_when_triggered(job, again);
        }
        match heard {
            // Heard at half the pause: the answer is in already.
            Some(heard) => {
                self.apply(Answer { id: JobId { conn: self.conn, n: job }, heard: Some(heard) })
            }
            None => Vec::new(),
        }
    }

    /// The utterance a closed segment ends, marked closed.
    fn close_utterance(&mut self, start: usize) -> u64 {
        let utterance = self
            .utterances
            .current
            .take()
            .unwrap_or_else(|| self.new_utterance(start, usize::MAX, true));
        self.mark(utterance, |r| {
            r.closed = true;
            r.head_checked = true;
            r.next_window = usize::MAX;
        });
        utterance
    }

    /// The hearing at half the pause that stands for a closed segment: the same quiet run
    /// closes it, and what it heard is a command alone or not in yet.
    fn half_pause_standing_in(&mut self, short: bool) -> Option<HalfPauseHearing> {
        self.hearings.half_pause.take().filter(|h| {
            short
                && match &h.answer {
                    None => true,
                    Some(heard) => heard.as_ref().is_some_and(|h| self.said_alone(h)),
                }
        })
    }

    /// The span of `closed` and the audio sent to confirm it, from sample `from`.
    fn segment_span(&self, closed: &ClosedSegment, short: bool, from: usize) -> SegmentSpan {
        let ClosedSegment { start, end, bounds, quiet_level, by_pause } = *closed;
        let post_roll = self.recent.between(end, end + samples_in(POST_ROLL_SECS));
        SegmentSpan {
            end,
            bounds,
            short,
            from,
            to: end + first_sound(&post_roll, quiet_level),
            quiet_level,
            tail_check: by_pause && end - start > samples_in(FIRST_WINDOW_SECS),
        }
    }

    /// Asks the confirmation of a closed segment over `from..to`, or has the hearing at half
    /// its pause stand for it; with what that hearing heard when it answered already.
    fn ask_segment(
        &mut self,
        context: JobContext,
        half_pause: Option<HalfPauseHearing>,
        own_from: usize,
        (from, to): (usize, usize),
    ) -> (Option<u64>, Option<Heard>) {
        match half_pause {
            Some(HalfPauseHearing { job, answer: None, .. }) => {
                log::debug!("job={job} heard at half the pause stands for its segment");
                if let Some(asked) = self.jobs.asked.get_mut(&job) {
                    asked.context = context;
                }
                self.hearings.stand_ins.insert(job, own_from);
                (Some(job), None)
            }
            Some(HalfPauseHearing { answer: Some(heard), .. }) => {
                (self.reserve(JobKind::Segment, context), heard)
            }
            None => (self.ask(JobKind::Segment, from, to, context), None),
        }
    }

    /// The segment of `utterance` was heard: it decides what the utterance said.
    pub(super) fn segment_answered(&self, utterance: u64) -> bool {
        self.utterances
            .running
            .get(&utterance)
            .is_some_and(|r| r.segment_answered)
    }

    /// The command the segment says: the whole utterance, when it is short enough for a
    /// command; else a doubled or chained command in its last seconds the running search
    /// did not report yet, or a command that ends it.
    pub(super) fn apply_segment(
        &mut self,
        reading: &PhraseMatcher,
        heard: &Heard,
        utterance: u64,
        segment: SegmentSpan,
    ) -> Option<ServerEvent> {
        let SegmentSpan { end, bounds, short, from, .. } = segment;
        if short {
            if let Some(phrase) = reading.phrase_in(&heard.text) {
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
            .running
            .get_mut(&utterance)
            .and_then(|r| r.last_window.take());
        let clip = HeardClip { heard, from, end: None };
        self.place_anywhere_or_at_end(reading, clip, utterance)
            .or_else(|| self.tail_heard_while_running(reading, last_window, end, utterance))
    }
}
