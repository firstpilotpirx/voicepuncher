//! One connection's audio in the streaming recognizer.
//!
//! The gate hears speech window by window, an utterance opens at its onset with the half
//! second before it replayed into every recognizer, and closes after a pause and half a
//! second more audio; its recognizers finish the silence after it on their own, so a close
//! never waits for a decode, and the gate forgets the speech so far at every close: a gate
//! that kept the state of a long dictation hears the pause after a lone word half a second
//! late.

use std::collections::VecDeque;
use std::time::SystemTime;

use crate::audio::SAMPLE_RATE;
use crate::conversions::saturating_usize;
use crate::history::CommandHistory;
use crate::matcher::PhraseMatcher;
use crate::protocol::{PhraseSelection, ServerEvent, UnknownPhrases};
use crate::streaming::Wake;

use super::ports::{SpeechGate, StreamingModel, GATE_WINDOW};

mod decoder;
mod kept;
mod utterance;
pub use decoder::{Decoding, Delivery};
use kept::Kept;
use utterance::{Closed, Utterance, Wait};

/// The audio before the onset replayed into the recognizers: the gate notices speech late.
const REPLAY_SECS: f32 = 0.5;
/// The audio past a pause the recognizers still get before their final decode.
const TAIL_SECS: f32 = 0.1;
/// 100 ms: how often the recognizers decode.
const FRAME_SAMPLES: usize = 1_600;

/// What a session is set up with beside its models.
pub struct SessionCfg<'a> {
    pub conn: u64,
    /// The silence that ends an utterance.
    pub min_silence_secs: f32,
    pub history: Option<&'a CommandHistory>,
    /// Called when a recognizer decoded more, so its events go out without waiting for audio.
    pub wake: Option<Wake>,
    /// When the session sees a decode.
    pub delivery: Delivery,
}

/// The recognizers an utterance opens in, and how their threads decode.
struct Recognizers<'a> {
    models: Vec<&'a dyn StreamingModel>,
    decoding: Decoding,
}

/// Where a closed utterance goes: the connection's log line and the command history.
struct Keeper<'a> {
    history: Option<&'a CommandHistory>,
    conn: u64,
    opened: SystemTime,
    entries: u64,
}

pub struct Session<'a> {
    recognizers: Recognizers<'a>,
    gate: Box<dyn SpeechGate + 'a>,
    keeper: Keeper<'a>,
    /// Silence that closes an utterance, its tail included.
    closing_quiet: usize,
    seen: usize,
    gate_pending: Vec<f32>,
    replay: VecDeque<f32>,
    matcher: PhraseMatcher,
    utterance: Option<Utterance>,
    /// Closed utterances whose recognizers still decode the silence after them.
    closing: Vec<Utterance>,
}

impl<'a> Session<'a> {
    pub fn new(
        models: Vec<&'a dyn StreamingModel>,
        gate: Box<dyn SpeechGate + 'a>,
        cfg: &SessionCfg<'a>,
    ) -> Self {
        Self {
            recognizers: Recognizers {
                models,
                decoding: Decoding { wake: cfg.wake.clone(), delivery: cfg.delivery },
            },
            gate,
            keeper: Keeper {
                history: cfg.history,
                conn: cfg.conn,
                opened: SystemTime::now(),
                entries: 0,
            },
            closing_quiet: samples_in(cfg.min_silence_secs + TAIL_SECS),
            seen: 0,
            gate_pending: Vec::new(),
            replay: VecDeque::new(),
            matcher: PhraseMatcher::new(&[]),
            utterance: None,
            closing: Vec::new(),
        }
    }

    /// Listens for the phrases of `selection` from now on; refuses a phrase that is no command.
    pub fn select(&mut self, selection: &PhraseSelection) -> Result<(), UnknownPhrases> {
        let unknown: Vec<String> = selection
            .phrases
            .iter()
            .filter(|p| !voicepuncher_voice_commands::COMMANDS.contains(&p.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(UnknownPhrases(unknown));
        }
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

    /// Answers once every recognizer decoded the audio before it, its events first: a command
    /// said right before the sync is reported before the answer.
    pub fn sync(&mut self, id: u64) -> Vec<ServerEvent> {
        let now = seconds(self.seen);
        let mut events = self.collect_closing(Wait::CaughtUp);
        if let Some(utterance) = self.utterance.as_mut() {
            events.extend(utterance.sync(&self.matcher, now));
        }
        events.push(ServerEvent::Synced { id });
        events
    }

    /// What the recognizers decoded since the last call.
    pub fn poll(&mut self) -> Vec<ServerEvent> {
        let now = seconds(self.seen);
        let mut events = self.collect_closing(Wait::No);
        if let Some(utterance) = self.utterance.as_mut() {
            events.extend(utterance.collect(&self.matcher, now, Wait::No));
        }
        events
    }

    /// What the recognizers of the closing utterances decoded since the last call; an
    /// utterance they finished is logged and kept in the history.
    fn collect_closing(&mut self, wait: Wait) -> Vec<ServerEvent> {
        let now = seconds(self.seen);
        let mut events = Vec::new();
        for utterance in &mut self.closing {
            events.extend(utterance.collect(&self.matcher, now, wait));
        }
        let (finished, closing): (Vec<Utterance>, Vec<Utterance>) =
            self.closing.drain(..).partition(Utterance::finished);
        self.closing = closing;
        for utterance in finished {
            self.keeper.keep(utterance.into_closed(&self.matcher));
        }
        events
    }

    fn frame(&mut self, frame: &[f32]) -> Vec<ServerEvent> {
        let speech = self.gate_hears(frame);
        let mut events = Vec::new();
        if self.utterance.is_none() && speech == Some(true) {
            events.push(self.open());
        }
        self.remember(frame);
        self.seen += frame.len();
        let now = seconds(self.seen);
        if let Some(utterance) = self.utterance.as_mut() {
            utterance.feed(frame, now);
            utterance.listen(speech, frame.len(), self.seen);
            if utterance.quiet() < self.closing_quiet {
                events.extend(utterance.collect(&self.matcher, now, Wait::No));
            } else {
                events.extend(self.close(now));
            }
        }
        events.extend(self.collect_closing(Wait::No));
        events
    }

    /// Whether the gate heard speech in any whole window of `frame`; nothing when `frame`
    /// completed no window.
    fn gate_hears(&mut self, frame: &[f32]) -> Option<bool> {
        self.gate_pending.extend_from_slice(frame);
        let windows = self.gate_pending.len() / GATE_WINDOW;
        let mut speech = false;
        for window in self.gate_pending.chunks_exact(GATE_WINDOW).take(windows) {
            speech |= self.gate.hears(window);
        }
        self.gate_pending.drain(..windows * GATE_WINDOW);
        (windows > 0).then_some(speech)
    }

    fn remember(&mut self, frame: &[f32]) {
        self.replay.extend(frame);
        let over = self.replay.len().saturating_sub(samples_in(REPLAY_SECS));
        self.replay.drain(..over);
    }

    /// Opens an utterance at the frame about to be heard, the replay before it; the words a
    /// closing utterance decodes from the replay on are the new one's to report.
    fn open(&mut self) -> ServerEvent {
        let start = self.seen - self.replay.len();
        let replay: Vec<f32> = self.replay.iter().copied().collect();
        let (start, onset) = (seconds(start), seconds(self.seen));
        for closing in &mut self.closing {
            closing.hands_over_from(start);
        }
        match Utterance::open(
            &self.recognizers.models,
            start,
            onset,
            &replay,
            &self.recognizers.decoding,
        ) {
            Ok(utterance) => self.utterance = Some(utterance),
            Err(e) => log::error!(
                "conn={} cannot start the recognizers' threads: {e}",
                self.keeper.conn
            ),
        }
        ServerEvent::SpeechStart { t: onset }
    }

    /// Closes the utterance at `now`: its speech end goes out now, what its recognizers
    /// still decode of the silence after it comes with the later collects, and the gate
    /// starts the next utterance afresh.
    fn close(&mut self, now: f32) -> Option<ServerEvent> {
        let mut utterance = self.utterance.take()?;
        // WHY: the next utterance's replay must not start with this one's last words; the
        // models hear a quiet "restart" after "…broken" as "let's start".
        let over = self.replay.len().saturating_sub(utterance.quiet());
        self.replay.drain(..over);
        let quiet: Vec<f32> = self.replay.iter().copied().collect();
        self.gate.forgets(&quiet);
        let end = utterance.begin_close(now);
        self.closing.push(utterance);
        Some(end)
    }
}

/// The connection ended: an utterance still open is closed now, and one whose recognizers
/// still decode the silence after it is waited for, so its log line and its entry of the
/// history are not lost.
impl Drop for Session<'_> {
    fn drop(&mut self) {
        let now = seconds(self.seen);
        self.close(now);
        self.collect_closing(Wait::CaughtUp);
    }
}

impl Keeper<'_> {
    fn keep(&mut self, closed: Closed) {
        log::info!("conn={} {}", self.conn, closed.trace.log_line());
        if let Some(history) = self.history {
            self.entries += 1;
            let source = format!("{}-{}-{}", std::process::id(), self.conn, self.entries);
            let kept = Kept { opened: self.opened, source };
            history.keep(kept::entry(
                &closed.trace,
                &closed.heard,
                closed.audio,
                kept,
            ));
        }
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
