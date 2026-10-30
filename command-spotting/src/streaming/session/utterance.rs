//! One utterance in every recognizer: its streams, its audio, the pause after it and what
//! each recognizer heard and reported of it.

use crate::confirm::TimedWord;
use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;
use crate::worker::Wake;

use super::super::decide::{decide, doubles, Finding, Moment};
use super::super::ports::{Decoded, StreamingModel};
use super::super::trace::Trace;
use super::super::words::timed_words;
use super::decoder::{Decoder, Update};
use super::{samples_in, seconds};

/// The longest audio an entry of the command history keeps.
const MAX_AUDIO_SECS: f32 = 30.0;

pub struct Utterance {
    /// Where its audio starts, in seconds of the stream.
    start: f32,
    decoders: Vec<Decoder>,
    respelled: Vec<bool>,
    /// What each recognizer held after its last decode.
    last: Vec<Decoded>,
    audio: Vec<f32>,
    /// Samples since the gate last heard speech.
    quiet: usize,
    /// Commands ending before this are reported already, by whichever recognizer.
    reported_until: f32,
    last_report: Option<Finding>,
    trace: Trace,
}

/// What an utterance left once it closed.
pub struct Closed {
    pub events: Vec<ServerEvent>,
    pub trace: Trace,
    /// What each recognizer's final text says, for the command history.
    pub heard: Vec<(usize, Finding)>,
    pub audio: Vec<f32>,
}

/// Whether collecting waits for every recognizer to decode all the audio fed to it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    No,
    CaughtUp,
}

impl Utterance {
    /// Opens it at `onset`, its audio from `start` on, with `replay` fed to every recognizer,
    /// each decoding on a thread of its own that calls `wake` when it decoded more.
    pub fn open(
        models: &[&dyn StreamingModel],
        start: f32,
        onset: f32,
        replay: &[f32],
        wake: Option<&Wake>,
    ) -> std::io::Result<Self> {
        let names: Vec<&str> = models.iter().map(|m| m.name()).collect();
        let decoders = models
            .iter()
            .map(|m| Decoder::start(m.name(), m.stream(), wake.cloned()))
            .collect::<std::io::Result<_>>()?;
        let mut utterance = Self {
            start,
            decoders,
            respelled: models.iter().map(|m| m.respelled()).collect(),
            last: vec![Decoded::default(); models.len()],
            audio: Vec::new(),
            quiet: 0,
            reported_until: 0.0,
            last_report: None,
            trace: Trace::new(start, onset, &names),
        };
        utterance.feed(replay);
        Ok(utterance)
    }

    pub fn feed(&mut self, samples: &[f32]) {
        if self.audio.len() < samples_in(MAX_AUDIO_SECS) {
            self.audio.extend_from_slice(samples);
        }
        for decoder in &mut self.decoders {
            decoder.feed(samples);
        }
    }

    /// The gate heard speech in the `samples` up to `seen`, or did not.
    pub fn listen(&mut self, speech: bool, samples: usize, seen: usize) {
        if speech {
            self.quiet = 0;
            self.trace.speech_end = seconds(seen);
        } else {
            self.quiet += samples;
        }
    }

    pub const fn quiet(&self) -> usize {
        self.quiet
    }

    /// Reports what the recognizers decoded since the last call, at `now` in the stream.
    pub fn collect(&mut self, matcher: &PhraseMatcher, now: f32, wait: Wait) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        for r in 0..self.decoders.len() {
            let updates = match wait {
                Wait::No => self.decoders[r].arrived(),
                Wait::CaughtUp => self.decoders[r].caught_up(),
            };
            for update in updates {
                events.extend(self.heard(matcher, r, update, now));
            }
        }
        events
    }

    /// Recognizer `r` decoded `update`: what it says is reported at `now`.
    fn heard(
        &mut self,
        matcher: &PhraseMatcher,
        r: usize,
        update: Update,
        now: f32,
    ) -> Option<ServerEvent> {
        self.trace.decoded(r, update.took);
        self.trace.heard(r, now, &text_of(&update.decoded));
        let words = self.words(r, &update.decoded);
        self.last[r] = update.decoded;
        let moment = if update.finished {
            Moment::Closing
        } else {
            Moment::Decoding {
                now: self.start + seconds(update.fed),
                pause: (self.quiet > 0).then_some(self.trace.speech_end),
            }
        };
        let found = decide(matcher, &words, moment, self.reported_until)?;
        self.report(r, found, now)
    }

    /// Reports `found`, heard by recognizer `r` at `now`, unless it doubles the last report.
    fn report(&mut self, r: usize, found: Finding, now: f32) -> Option<ServerEvent> {
        self.reported_until = found.end;
        let doubled = doubles(self.last_report.as_ref(), &found);
        self.last_report = Some(found.clone());
        (!doubled).then(|| {
            self.trace.reported(r, &found, now);
            command(found)
        })
    }

    /// Ends every stream, waits for what is left to be decoded and reports it; the speech end
    /// follows.
    pub fn close(mut self, matcher: &PhraseMatcher, now: f32) -> Closed {
        for decoder in &mut self.decoders {
            decoder.finish();
        }
        let mut events = self.collect(matcher, now, Wait::CaughtUp);
        events.push(ServerEvent::SpeechEnd { t: self.trace.speech_end });
        self.trace.closed = now;
        let heard = (0..self.last.len())
            .filter_map(|r| {
                let words = self.words(r, &self.last[r]);
                decide(matcher, &words, Moment::Closing, 0.0).map(|found| (r, found))
            })
            .collect();
        Closed { events, trace: self.trace, heard, audio: self.audio }
    }

    fn words(&self, r: usize, decoded: &Decoded) -> Vec<TimedWord> {
        timed_words(
            &decoded.tokens,
            &decoded.times,
            self.start,
            self.respelled[r],
        )
    }
}

/// The text the tokens spell, as the recognizer wrote it.
fn text_of(decoded: &Decoded) -> String {
    decoded.tokens.concat().replace('▁', " ").trim().to_owned()
}

fn command(found: Finding) -> ServerEvent {
    ServerEvent::Command {
        commands: found.commands,
        start: found.start,
        end: found.end,
        placement: found.placement,
        timing: None,
    }
}
