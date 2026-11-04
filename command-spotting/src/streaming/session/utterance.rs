//! One utterance in every recognizer: its streams, its audio, the pause after it and what
//! each recognizer heard and reported of it.

use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;
use crate::streaming::words::TimedWord;
use crate::streaming::Wake;

use super::super::decide::{decide, doubles, Finding, Moment};
use super::super::ports::{Decoded, StreamingModel};
use super::super::trace::Trace;
use super::super::words::timed_words;
use super::decoder::{Decoder, Update};
use super::{samples_in, seconds};

/// The longest audio an entry of the command history keeps.
const MAX_AUDIO_SECS: f32 = 30.0;
/// The silence fed to every recognizer at the gate's pause, ahead of the stream.
pub const FLUSH_SECS: f32 = 0.6;
/// The silence fed to every recognizer ahead of the replay, so the first sound of an utterance
/// is not the first sound of its stream.
pub const LEAD_SECS: f32 = 0.3;
/// How much later than the gate's onset a recognizer may time the first word of an utterance.
const OPENING_SLACK_SECS: f32 = 0.5;

pub struct Utterance {
    /// Where its audio starts, in seconds of the stream.
    start: f32,
    decoders: Vec<Decoder>,
    respelled: Vec<bool>,
    /// What each recognizer held after its last decode.
    last: Vec<Decoded>,
    audio: Vec<f32>,
    /// Samples fed to the recognizers, the flushed silence too.
    streamed: usize,
    /// Where silence was flushed into the recognizers' stream, and how much, in samples.
    flushes: Vec<(usize, usize)>,
    /// Samples since the gate last heard speech.
    quiet: usize,
    /// Where the app's sync stands in the stream while it is answered: the words before it
    /// are said, whatever the gate hears.
    synced_at: Option<f32>,
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
            streamed: 0,
            flushes: Vec::new(),
            quiet: 0,
            synced_at: None,
            reported_until: 0.0,
            last_report: None,
            trace: Trace::new(start, onset, &names),
        };
        utterance.silence(LEAD_SECS);
        utterance.feed(replay);
        Ok(utterance)
    }

    pub fn feed(&mut self, samples: &[f32]) {
        if self.audio.len() < samples_in(MAX_AUDIO_SECS) {
            self.audio.extend_from_slice(samples);
        }
        self.stream(samples);
    }

    fn stream(&mut self, samples: &[f32]) {
        self.streamed += samples.len();
        for decoder in &mut self.decoders {
            decoder.feed(samples);
        }
    }

    /// The gate heard speech in the `samples` up to `seen`, or did not, or judged none of them:
    /// they then go on as speech or the pause before them.
    pub fn listen(&mut self, speech: Option<bool>, samples: usize, seen: usize) {
        let speech = speech.unwrap_or(self.quiet == 0);
        if speech {
            self.quiet = 0;
            self.trace.speech_end = seconds(seen);
        } else {
            if self.quiet == 0 {
                self.flush();
            }
            self.quiet += samples;
        }
    }

    /// Feeds silence ahead of the stream at the gate's pause, so a recognizer that emits a
    /// word's last sound only once more audio came emits it now.
    fn flush(&mut self) {
        self.silence(FLUSH_SECS);
    }

    /// Feeds `secs` of silence to every recognizer, left out of the times they give.
    fn silence(&mut self, secs: f32) {
        let silence = vec![0.0; samples_in(secs)];
        self.flushes.push((self.streamed, silence.len()));
        self.stream(&silence);
    }

    /// The time in the stream of `at`, a time in the recognizers' stream, the flushed silence
    /// before it left out; a time inside a flush falls at its start.
    fn unflushed(&self, at: f32) -> f32 {
        let since = at - self.start;
        let flushed: f32 = self
            .flushes
            .iter()
            .map(|&(from, len)| (since - seconds(from)).clamp(0.0, seconds(len)))
            .sum();
        at - flushed
    }

    pub const fn quiet(&self) -> usize {
        self.quiet
    }

    /// Reports what the recognizers decoded up to the app's sync at `now` in the stream: a
    /// command said right before it counts as said, the gate's pause or not.
    pub fn sync(&mut self, matcher: &PhraseMatcher, now: f32) -> Vec<ServerEvent> {
        self.synced_at = Some(now);
        let events = self.collect(matcher, now, Wait::CaughtUp);
        self.synced_at = None;
        events
    }

    /// Reports what the recognizers decoded since the last call, at `now` in the stream.
    pub fn collect(&mut self, matcher: &PhraseMatcher, now: f32, wait: Wait) -> Vec<ServerEvent> {
        let mut arrived: Vec<(usize, Vec<Update>)> = (0..self.decoders.len())
            .map(|r| {
                let updates = match wait {
                    Wait::No => self.decoders[r].arrived(),
                    Wait::CaughtUp => self.decoders[r].caught_up(),
                };
                (r, updates)
            })
            .collect();
        // WHY: a recognizer that heard only the end of a command word ("start" of a quiet
        // "restart") must not speak first when the other heard all of it.
        arrived.sort_by_key(|(r, updates)| self.ends_a_longer_word(matcher, *r, updates));
        let mut events = Vec::new();
        for (r, updates) in arrived {
            for update in updates {
                events.extend(self.heard(matcher, r, update, now));
            }
        }
        events
    }

    /// Whether the last word recognizer `r` holds after `updates` may be the end of a longer
    /// command word.
    fn ends_a_longer_word(&self, matcher: &PhraseMatcher, r: usize, updates: &[Update]) -> bool {
        updates.last().is_some_and(|update| {
            self.words(r, &update.decoded)
                .last()
                .is_some_and(|word| matcher.ends_a_longer_word(&word.text))
        })
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
                now: self.unflushed(self.start + seconds(update.fed)),
                pause: (self.quiet > 0)
                    .then_some(self.trace.speech_end)
                    .or(self.synced_at),
            }
        };
        let found = decide(matcher, &words, moment, self.reported_until)?;
        let found = self.started_at_onset(found, &words);
        self.report(r, found, now)
    }

    /// `found` starting at the gate's onset when it opens the utterance: a recognizer times a
    /// first word later than it is said, and the dictation it ends would keep its first sounds.
    fn started_at_onset(&self, mut found: Finding, words: &[TimedWord]) -> Finding {
        let onset = self.trace.onset;
        let opens = words.first().is_some_and(|w| w.start >= found.start);
        if opens && found.start - onset <= OPENING_SLACK_SECS {
            found.start = found.start.min(onset);
        }
        found
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
        let mut words = timed_words(
            &decoded.tokens,
            &decoded.times,
            self.start,
            self.respelled[r],
        );
        for word in &mut words {
            word.start = self.unflushed(word.start);
            word.end = self.unflushed(word.end);
        }
        words
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
    }
}
