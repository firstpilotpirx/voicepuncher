//! One utterance in every recognizer: its streams, its audio, the pause after it and what
//! each recognizer heard and reported of it.

use std::time::{Duration, Instant};

use crate::confirm::TimedWord;
use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;

use super::super::decide::{decide, Finding, Moment};
use super::super::ports::{Decoded, RecognizerStream, StreamingModel};
use super::super::trace::Trace;
use super::super::words::timed_words;
use super::{samples_in, seconds};

/// The longest audio an entry of the command history keeps.
const MAX_AUDIO_SECS: f32 = 30.0;

pub struct Utterance<'a> {
    /// Where its audio starts, in seconds of the stream.
    start: f32,
    streams: Vec<Box<dyn RecognizerStream + 'a>>,
    respelled: Vec<bool>,
    audio: Vec<f32>,
    /// Samples since the gate last heard speech.
    quiet: usize,
    /// Commands ending before this are reported already, by whichever recognizer.
    reported_until: f32,
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

impl<'a> Utterance<'a> {
    /// Opens it at `onset`, its audio from `start` on, with `replay` fed to every recognizer.
    pub fn open(models: &[&'a dyn StreamingModel], start: f32, onset: f32, replay: &[f32]) -> Self {
        let names: Vec<&str> = models.iter().map(|m| m.name()).collect();
        let mut utterance = Self {
            start,
            streams: models.iter().map(|m| m.stream()).collect(),
            respelled: models.iter().map(|m| m.respelled()).collect(),
            audio: Vec::new(),
            quiet: 0,
            reported_until: 0.0,
            trace: Trace::new(start, onset, &names),
        };
        utterance.feed(replay);
        utterance
    }

    pub fn feed(&mut self, samples: &[f32]) {
        if self.audio.len() < samples_in(MAX_AUDIO_SECS) {
            self.audio.extend_from_slice(samples);
        }
        for stream in &mut self.streams {
            stream.accept(samples);
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

    /// Decodes the new audio in every recognizer and reports what they heard, at `now`.
    pub fn step(&mut self, matcher: &PhraseMatcher, moment: Moment, now: f32) -> Vec<ServerEvent> {
        let took = decode_all(&mut self.streams);
        let mut events = Vec::new();
        for (r, took) in took.into_iter().enumerate() {
            self.trace.decoded(r, took);
            let decoded = self.streams[r].decoded();
            self.trace.heard(r, now, &text_of(&decoded));
            let words = self.words(r, &decoded);
            if let Some(found) = decide(matcher, &words, moment, self.reported_until) {
                self.reported_until = found.end;
                self.trace.reported(r, &found, now);
                events.push(command(found));
            }
        }
        events
    }

    /// Ends every stream, decodes what is left and reports it; the speech end follows.
    pub fn close(mut self, matcher: &PhraseMatcher, now: f32) -> Closed {
        for stream in &mut self.streams {
            stream.finish();
        }
        let mut events = self.step(matcher, Moment::Closing, now);
        events.push(ServerEvent::SpeechEnd { t: self.trace.speech_end });
        self.trace.closed = now;
        let heard = (0..self.streams.len())
            .filter_map(|r| {
                let words = self.words(r, &self.streams[r].decoded());
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

/// Decodes every stream at once, each on its own thread; how long each took.
fn decode_all(streams: &mut [Box<dyn RecognizerStream + '_>]) -> Vec<Duration> {
    std::thread::scope(|scope| {
        let decoding: Vec<_> = streams
            .iter_mut()
            .map(|stream| {
                scope.spawn(move || {
                    let began = Instant::now();
                    stream.decode();
                    began.elapsed()
                })
            })
            .collect();
        decoding
            .into_iter()
            .map(|d| d.join().unwrap_or_default())
            .collect()
    })
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
