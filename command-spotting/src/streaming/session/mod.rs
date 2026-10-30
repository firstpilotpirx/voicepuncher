//! One connection's audio in the streaming recognizer: the gate hears speech window by
//! window, an utterance opens at its onset with the half second before it replayed into every
//! recognizer, and closes after a pause and half a second more audio.

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use crate::audio::SAMPLE_RATE;
use crate::conversions::saturating_usize;
use crate::history::CommandHistory;
use crate::keywords::UnknownPhrases;
use crate::matcher::PhraseMatcher;
use crate::protocol::{PhraseSelection, ServerEvent};

use super::decide::Moment;
use super::ports::{SpeechGate, StreamingModel, GATE_WINDOW};

mod kept;
mod utterance;
use kept::Kept;
use utterance::Utterance;

/// The audio before the onset replayed into the recognizers: the gate notices speech late.
const REPLAY_SECS: f32 = 0.5;
/// The audio past a pause the recognizers still get before their final decode.
const TAIL_SECS: f32 = 0.5;
/// 100 ms: how often the recognizers decode.
const FRAME_SAMPLES: usize = 1_600;

/// What a session is set up with beside its models.
pub struct SessionCfg<'a> {
    pub conn: u64,
    /// The silence that ends an utterance.
    pub min_silence_secs: f32,
    pub history: Option<&'a CommandHistory>,
}

pub struct Session<'a> {
    models: Vec<&'a dyn StreamingModel>,
    gate: Box<dyn SpeechGate + 'a>,
    history: Option<&'a CommandHistory>,
    conn: u64,
    opened: SystemTime,
    entries: u64,
    /// Silence that closes an utterance, its tail included.
    closing_quiet: usize,
    seen: usize,
    gate_pending: Vec<f32>,
    replay: VecDeque<f32>,
    matcher: PhraseMatcher,
    utterance: Option<Utterance<'a>>,
}

impl<'a> Session<'a> {
    pub fn new(
        models: Vec<&'a dyn StreamingModel>,
        gate: Box<dyn SpeechGate + 'a>,
        cfg: &SessionCfg<'a>,
    ) -> Self {
        Self {
            models,
            gate,
            history: cfg.history,
            conn: cfg.conn,
            opened: SystemTime::now(),
            entries: 0,
            closing_quiet: samples_in(cfg.min_silence_secs + TAIL_SECS),
            seen: 0,
            gate_pending: Vec::new(),
            replay: VecDeque::new(),
            matcher: PhraseMatcher::new(&[]),
            utterance: None,
        }
    }

    /// Listens for the phrases of `selection` from now on.
    pub fn select(&mut self, selection: &PhraseSelection) -> Result<(), UnknownPhrases> {
        self.matcher =
            PhraseMatcher::new(&selection.phrases).with_early_start(selection.early_start);
        Ok(())
    }

    /// Hears `samples` frame by frame, so a command goes out within a frame of its decoding.
    pub fn accept(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        samples
            .chunks(FRAME_SAMPLES)
            .flat_map(|frame| self.frame(frame))
            .collect()
    }

    /// Every event of the audio before it is out already: the answer is at once.
    pub fn sync(&mut self, id: u64) -> Vec<ServerEvent> {
        vec![ServerEvent::Synced { id }]
    }

    /// Nothing is answered later: no event waits.
    pub fn poll(&mut self) -> Vec<ServerEvent> {
        Vec::new()
    }

    /// No confirmation runs, so none stalls.
    pub fn stalled(&self) -> Option<Duration> {
        None
    }

    pub fn pending_confirmations(&self) -> usize {
        0
    }

    fn frame(&mut self, frame: &[f32]) -> Vec<ServerEvent> {
        let speech = self.gate_hears(frame);
        let mut events = Vec::new();
        if self.utterance.is_none() && speech {
            events.push(self.open());
        }
        self.remember(frame);
        self.seen += frame.len();
        let now = seconds(self.seen);
        let Some(utterance) = self.utterance.as_mut() else {
            return events;
        };
        utterance.feed(frame);
        utterance.listen(speech, frame.len(), self.seen);
        if utterance.quiet() < self.closing_quiet {
            events.extend(utterance.step(&self.matcher, Moment::Decoding { now }, now));
        } else {
            events.extend(self.close(now));
        }
        events
    }

    /// Whether the gate heard speech in any whole window of `frame`.
    fn gate_hears(&mut self, frame: &[f32]) -> bool {
        self.gate_pending.extend_from_slice(frame);
        let windows = self.gate_pending.len() / GATE_WINDOW;
        let mut speech = false;
        for window in self.gate_pending.chunks_exact(GATE_WINDOW).take(windows) {
            speech |= self.gate.hears(window);
        }
        self.gate_pending.drain(..windows * GATE_WINDOW);
        speech
    }

    fn remember(&mut self, frame: &[f32]) {
        self.replay.extend(frame);
        let over = self.replay.len().saturating_sub(samples_in(REPLAY_SECS));
        self.replay.drain(..over);
    }

    /// Opens an utterance at the frame about to be heard, the replay before it.
    fn open(&mut self) -> ServerEvent {
        let start = self.seen - self.replay.len();
        let replay: Vec<f32> = self.replay.iter().copied().collect();
        let (start, onset) = (seconds(start), seconds(self.seen));
        self.utterance = Some(Utterance::open(&self.models, start, onset, &replay));
        ServerEvent::SpeechStart { t: onset }
    }

    fn close(&mut self, now: f32) -> Vec<ServerEvent> {
        let Some(utterance) = self.utterance.take() else {
            return Vec::new();
        };
        let closed = utterance.close(&self.matcher, now);
        log::info!("conn={} {}", self.conn, closed.trace.log_line());
        if let Some(history) = self.history {
            self.entries += 1;
            let source = format!("{}-{}-{}", std::process::id(), self.conn, self.entries);
            let kept = Kept { opened: self.opened, source };
            if let Some(entry) = kept::entry(&closed.trace, &closed.heard, closed.audio, kept) {
                history.keep(entry);
            }
        }
        closed.events
    }
}

fn samples_in(secs: f32) -> usize {
    saturating_usize(secs * SAMPLE_RATE as f32)
}

fn seconds(samples: usize) -> f32 {
    samples as f32 / SAMPLE_RATE as f32
}

#[cfg(test)]
mod tests;
