//! One utterance in every recognizer: its streams, its audio, the pause after it and what
//! each recognizer heard and reported of it.

use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;
use crate::streaming::words::TimedWord;

use super::super::decide::{decide, doubles, Finding, Moment};
use super::super::ports::{Decoded, StreamingModel};
use super::super::trace::Trace;
use super::super::words::timed_words;
use super::decoder::{Decoder, Decoding, Update};
use super::{samples_in, seconds};

/// The longest audio an entry of the command history keeps.
const MAX_AUDIO_SECS: f32 = 30.0;
/// The silence fed to every recognizer at the gate's pause, ahead of the stream.
pub const FLUSH_SECS: f32 = 0.6;
/// The silence fed to every recognizer at the close, before its final decode: a quiet word
/// the gate stopped hearing before its last sound is decoded only once the pause after it
/// is heard, and the flush at the gate's pause came before that sound. 0.5 s loses one of
/// the user's two quiet starts of session 6, 0.8 s decodes both; 1 s keeps a margin.
pub const CLOSE_SILENCE_SECS: f32 = 1.0;
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
    /// While the app's sync is answered: the words before it are said, whatever the gate
    /// hears.
    synced: bool,
    reports: Reports,
    trace: Trace,
}

/// What an utterance reports: its last report, what waits to be decided and what it leaves to
/// the next utterance.
#[derive(Default)]
struct Reports {
    /// The last report, by whichever recognizer: commands ending before its end are reported.
    last: Option<Finding>,
    /// Finished updates held back at the close: a recognizer that heard only the end of a
    /// command word ("start" of a quiet "restart") waits for the other to finish, and every
    /// one waits a frame for the next utterance to open over this one's tail.
    held: Vec<(usize, Update)>,
    /// Where the next utterance's audio starts: the words from there on are its to report.
    handover: Option<f32>,
}

/// What an utterance left once every recognizer finished it.
pub struct Closed {
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
    /// each decoding on a thread of its own as `decoding` says.
    pub fn open(
        models: &[&dyn StreamingModel],
        start: f32,
        onset: f32,
        replay: &[f32],
        decoding: &Decoding,
    ) -> std::io::Result<Self> {
        let names: Vec<&str> = models.iter().map(|m| m.name()).collect();
        let decoders = models
            .iter()
            .map(|m| Decoder::start(m.name(), m.stream(), decoding))
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
            synced: false,
            reports: Reports::default(),
            trace: Trace::new(start, onset, &names),
        };
        utterance.silence(LEAD_SECS, onset);
        utterance.feed(replay, onset);
        Ok(utterance)
    }

    /// Feeds `samples`, the stream at `now`.
    pub fn feed(&mut self, samples: &[f32], now: f32) {
        if self.audio.len() < samples_in(MAX_AUDIO_SECS) {
            self.audio.extend_from_slice(samples);
        }
        self.stream(samples, now);
    }

    fn stream(&mut self, samples: &[f32], now: f32) {
        self.streamed += samples.len();
        for decoder in &mut self.decoders {
            decoder.feed(samples, now);
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
                self.flush(seconds(seen));
            }
            self.quiet += samples;
        }
    }

    /// Feeds silence ahead of the stream at the gate's pause at `now`, so a recognizer that
    /// emits a word's last sound only once more audio came emits it now.
    fn flush(&mut self, now: f32) {
        self.silence(FLUSH_SECS, now);
    }

    /// Feeds `secs` of silence to every recognizer at `now`, left out of the times they give.
    fn silence(&mut self, secs: f32, now: f32) {
        let silence = vec![0.0; samples_in(secs)];
        self.flushes.push((self.streamed, silence.len()));
        self.stream(&silence, now);
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
        self.synced = true;
        let events = self.collect(matcher, now, Wait::CaughtUp);
        self.synced = false;
        events
    }

    /// Reports what the recognizers decoded since the last call, at `now` in the stream.
    pub fn collect(&mut self, matcher: &PhraseMatcher, now: f32, wait: Wait) -> Vec<ServerEvent> {
        let mut arrived: Vec<(usize, Vec<Update>)> = (0..self.decoders.len())
            .map(|r| {
                let updates = match wait {
                    Wait::No => self.decoders[r].arrived(now),
                    Wait::CaughtUp => self.decoders[r].caught_up(),
                };
                (r, updates)
            })
            .collect();
        self.hold_or_release(matcher, &mut arrived, now, wait);
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

    /// Holds back what is not decided yet at `now`, or adds what was held to `arrived`.
    fn hold_or_release(
        &mut self,
        matcher: &PhraseMatcher,
        arrived: &mut Vec<(usize, Vec<Update>)>,
        now: f32,
        wait: Wait,
    ) {
        if wait == Wait::No && now <= self.trace.closed {
            // WHY: the next utterance opens over this one's tail only in the frame after the
            // close; its words are reported once, by the utterance that hears them whole.
            self.hold_finished(arrived);
        } else if wait == Wait::No && !self.decoded_all() {
            self.hold_ends_of_longer_words(matcher, arrived);
        } else {
            arrived.extend(
                self.reports
                    .held
                    .drain(..)
                    .map(|(r, update)| (r, vec![update])),
            );
        }
    }

    /// Holds back every finished update of `arrived`.
    fn hold_finished(&mut self, arrived: &mut [(usize, Vec<Update>)]) {
        for (r, updates) in arrived.iter_mut() {
            let finished = updates.last().is_some_and(|u| u.finished);
            if let Some(update) = finished.then(|| updates.pop()).flatten() {
                self.reports.held.push((*r, update));
            }
        }
    }

    /// The next utterance opened with its audio from `start` on: the words from there on
    /// are its to report, as the gate heard their first sound late, in this one's tail.
    pub fn hands_over_from(&mut self, start: f32) {
        self.reports.handover = Some(start);
    }

    /// Holds back a finished update of `arrived` whose last word may be the end of a longer
    /// command word until every recognizer finished: the other may have heard the whole word,
    /// and only the sorted reports of both keep the longer one first.
    fn hold_ends_of_longer_words(
        &mut self,
        matcher: &PhraseMatcher,
        arrived: &mut [(usize, Vec<Update>)],
    ) {
        for (r, updates) in arrived.iter_mut() {
            let holds = updates.last().is_some_and(|u| u.finished)
                && self.ends_a_longer_word(matcher, *r, updates);
            if let Some(update) = holds.then(|| updates.pop()).flatten() {
                self.reports.held.push((*r, update));
            }
        }
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
                pause: (self.quiet > 0).then_some(self.trace.speech_end),
                synced: self.synced,
            }
        };
        let found = decide(matcher, &words, moment, self.reported_until())?;
        if self
            .reports
            .handover
            .is_some_and(|from| found.start >= from)
        {
            return None;
        }
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

    /// The end of the last report: commands ending before it are reported already.
    fn reported_until(&self) -> f32 {
        self.reports.last.as_ref().map_or(0.0, |found| found.end)
    }

    /// Reports `found`, heard by recognizer `r` at `now`, unless it doubles the last report.
    fn report(&mut self, r: usize, found: Finding, now: f32) -> Option<ServerEvent> {
        let doubled = doubles(self.reports.last.as_ref(), &found);
        self.reports.last = Some(found.clone());
        (!doubled).then(|| {
            self.trace.reported(r, &found, now);
            command(found)
        })
    }

    /// Ends every stream at `now`, the silence after the words fed first: the speech end goes
    /// out at once, and what the recognizers still decode is reported by the later collects,
    /// as a decode of that silence under load takes longer than a command may wait.
    pub fn begin_close(&mut self, now: f32) -> ServerEvent {
        self.silence(CLOSE_SILENCE_SECS, now);
        for decoder in &mut self.decoders {
            decoder.finish(now);
        }
        self.trace.closed = now;
        ServerEvent::SpeechEnd { t: self.trace.speech_end }
    }

    /// Whether every recognizer decoded all of it, the close's silence included.
    fn decoded_all(&self) -> bool {
        self.decoders.iter().all(Decoder::finished)
    }

    /// Whether every recognizer decoded all of it and everything decoded is reported.
    pub fn finished(&self) -> bool {
        self.decoded_all() && self.reports.held.is_empty()
    }

    /// What it left, once every recognizer finished it.
    pub fn into_closed(self, matcher: &PhraseMatcher) -> Closed {
        let heard = (0..self.last.len())
            .filter_map(|r| {
                let words = self.words(r, &self.last[r]);
                decide(matcher, &words, Moment::Closing, 0.0).map(|found| (r, found))
            })
            .collect();
        Closed { trace: self.trace, heard, audio: self.audio }
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
