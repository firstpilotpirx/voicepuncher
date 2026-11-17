//! The second look at the last second of an utterance's speech: once every recognizer finished
//! the utterance and none reported a command, each decodes that second again, alone, in a fresh
//! stream. A fast "enter" glued to the last word, which the running decode lost to the words
//! before it, is heard there and reported after the close.

use voicepuncher_voice_commands::ENTER;

use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;

use super::super::super::decide::{decide, Finding, Moment};
use super::super::super::ports::{Decoded, StreamingModel};
use super::super::super::words::timed_words;
use super::super::decoder::{Decoder, Decoding};
use super::super::{samples_in, Recognizers};
use super::{text_of, Utterance, Wait, CLOSE_SILENCE_SECS, LEAD_SECS};

/// How much speech before its end each second look decodes. Of the five fast enters the
/// running decode missed (2026-11-16..17), 0.8 s hears all, 1.0 s four, 0.6 s three and
/// 0.9 s four: a longer window lets the words before "enter" pull the models off it, a shorter
/// one cuts the word before it. Both windows are looked at, so one heard is enough.
const WINDOWS_SECS: [f32; 2] = [0.8, 1.0];
/// How much past the gate's speech end the second look still takes: the gate stops hearing a
/// quiet word before its last sound.
const PAST_END_SECS: f32 = 0.1;

/// Where the second look of a closing utterance stands.
#[derive(Default)]
pub(in super::super) enum Look {
    /// Not started: the recognizers still decode the utterance.
    #[default]
    Waiting,
    Running(Vec<Glance>),
    /// It reported nothing, or the utterance needs none.
    Done,
    /// It reported `Finding`, heard by the recognizer the trace names at the index.
    Found(usize, Finding),
}

/// One recognizer's second look.
pub(in super::super) struct Glance {
    decoder: Decoder,
    respelled: bool,
    /// Its recognizer in the utterance's trace.
    traced: usize,
    /// Where its stream starts, in seconds of the session's stream.
    offset: f32,
    last: Decoded,
}

impl Utterance {
    /// Starts, collects or ends the second look at `now`, once every recognizer finished the
    /// utterance and everything decoded is reported: an "enter" it hears is reported.
    pub fn look_again(
        &mut self,
        matcher: &PhraseMatcher,
        recognizers: &Recognizers<'_>,
        now: f32,
        wait: Wait,
    ) -> Option<ServerEvent> {
        if !self.decoded_all() || !self.reports.held.is_empty() {
            return None;
        }
        let mut glances = match std::mem::take(&mut self.look) {
            Look::Waiting if self.reports.sent.is_empty() => {
                self.glances(&recognizers.models, &recognizers.decoding, now)
            }
            Look::Running(glances) => glances,
            look => {
                self.look = Look::Done.or(look);
                return None;
            }
        };
        for glance in &mut glances {
            self.collect_glance(glance, now, wait);
        }
        if glances.iter().all(|g| g.decoder.finished()) {
            return self.glanced(matcher, &glances, now);
        }
        self.look = Look::Running(glances);
        None
    }

    /// Takes what `glance` decoded so far into the trace and its last result.
    fn collect_glance(&mut self, glance: &mut Glance, now: f32, wait: Wait) {
        let updates = match wait {
            Wait::No => glance.decoder.arrived(now),
            Wait::CaughtUp => glance.decoder.caught_up(),
        };
        for update in updates {
            self.trace.decoded(glance.traced, update.took);
            self.trace
                .heard(glance.traced, now, &text_of(&update.decoded));
            glance.last = update.decoded;
        }
    }

    /// Opens fresh streams in every recognizer, one per window, each fed the window's last
    /// speech between the lead silence and the close's.
    fn glances(
        &mut self,
        models: &[&dyn StreamingModel],
        decoding: &Decoding,
        now: f32,
    ) -> Vec<Glance> {
        WINDOWS_SECS
            .iter()
            .flat_map(|&secs| self.glances_over(secs, models, decoding, now))
            .collect()
    }

    /// The glances of every recognizer at the last `secs` of speech.
    fn glances_over(
        &mut self,
        secs: f32,
        models: &[&dyn StreamingModel],
        decoding: &Decoding,
        now: f32,
    ) -> Vec<Glance> {
        let end = self.trace.speech_end + PAST_END_SECS;
        let from = (self.trace.speech_end - secs).max(self.start);
        let until = samples_in(end - self.start).min(self.audio.len());
        let window = self.audio[samples_in(from - self.start).min(until)..until].to_vec();
        if window.is_empty() {
            return Vec::new();
        }
        let audio = [
            vec![0.0; samples_in(LEAD_SECS)],
            window,
            vec![0.0; samples_in(CLOSE_SILENCE_SECS)],
        ]
        .concat();
        models
            .iter()
            .filter_map(|model| {
                let started = Decoder::start(model.name(), model.stream(), decoding);
                let mut decoder = started
                    .inspect_err(|e| log::error!("cannot start a second look: {e}"))
                    .ok()?;
                decoder.feed(&audio, now);
                decoder.finish(now);
                Some(Glance {
                    decoder,
                    respelled: model.respelled(),
                    traced: self
                        .trace
                        .add(&format!("{} second look {secs} s", model.name())),
                    offset: from - LEAD_SECS,
                    last: Decoded::default(),
                })
            })
            .collect()
    }

    /// Reports the first "enter" a finished second look heard at the end of its words.
    fn glanced(
        &mut self,
        matcher: &PhraseMatcher,
        glances: &[Glance],
        now: f32,
    ) -> Option<ServerEvent> {
        self.look = Look::Done;
        let (traced, found) = glances.iter().find_map(|glance| {
            let words = timed_words(
                &glance.last.tokens,
                &glance.last.times,
                glance.offset,
                glance.respelled,
            );
            let found = decide(matcher, &words, Moment::Closing, &self.reported())?;
            let enter = found.commands.iter().all(|c| c == ENTER);
            let handed_over = self
                .reports
                .handover
                .is_some_and(|from| found.start >= from);
            (enter && !handed_over).then_some((glance.traced, found))
        })?;
        self.look = Look::Found(traced, found.clone());
        self.report(traced, found, now)
    }

    /// Whether the second look ended or was never needed.
    pub(super) const fn looked(&self) -> bool {
        matches!(self.look, Look::Done | Look::Found(..))
    }

    /// What the second look reported, for the command history.
    pub(super) fn found_again(&self) -> Option<(usize, Finding)> {
        match &self.look {
            Look::Found(traced, found) => Some((*traced, found.clone())),
            _ => None,
        }
    }
}

impl Look {
    /// `self`, unless `look` already ended.
    fn or(self, look: Self) -> Self {
        match look {
            Look::Found(..) => look,
            _ => self,
        }
    }
}
