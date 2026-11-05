//! One clip streamed through a streaming session, as the app streams the microphone.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::protocol::{PhraseSelection, ServerEvent};
use crate::streaming::engine::StreamingEngine;

use super::{label_word, Clip, Outcome};

/// How the audio is handed to the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// As the microphone gives it: the delay of each report is measured by the wall clock.
    RealTime,
    /// As fast as the session takes it, the session replaying the service's clock: the delay
    /// of each report is measured by the stream position.
    Fast,
}

/// When a command was reported: by the wall clock, and by the stream position (the seconds of
/// audio handed over) at the end of the chunk that reported it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stamp {
    pub wall: Instant,
    pub stream: f32,
}

/// 20 ms: about how often the app hands the service new audio.
const CHUNK: usize = 320;
/// Silence after the clip, so its last utterance closes.
const TAIL_SECS: f32 = 1.0;

/// Streams `clip` through a new session of `engine` listening for the clip's commands.
pub fn run_clip(engine: &StreamingEngine, clip: &Clip, pace: Pace) -> Result<Outcome> {
    let mut session = match pace {
        Pace::RealTime => engine.session_for(0)?,
        Pace::Fast => engine.session_replaying(0)?,
    };
    let phrases = clip.listening.clone();
    session.select(&PhraseSelection { phrases, ..PhraseSelection::default() })?;
    let tail = vec![0.0f32; crate::conversions::saturating_usize(TAIL_SECS * 16_000.0)];
    let audio = [clip.audio.as_slice(), tail.as_slice()].concat();
    let began = Instant::now();
    let mut heard: Vec<(String, Stamp)> = Vec::new();
    let mut processing = Duration::ZERO;
    let mut streamed = 0;
    for (i, chunk) in audio.chunks(CHUNK).enumerate() {
        if pace == Pace::RealTime {
            wait_until(began + chunk_end(i, chunk.len()));
        }
        streamed += chunk.len();
        let called = Instant::now();
        let mut events = session.accept(chunk);
        events.extend(session.poll());
        processing += called.elapsed();
        note(&mut heard, events, streamed);
    }
    let called = Instant::now();
    note(&mut heard, session.sync(0), streamed);
    processing += called.elapsed();
    let delay_ms = delay_ms(pace, clip.word_end, heard.first().map(|(_, at)| at), began);
    Ok(Outcome {
        id: clip.id.clone(),
        label: clip.label.clone(),
        heard: heard.into_iter().map(|(w, _)| w).collect(),
        delay_ms,
        processing_secs: processing.as_secs_f64(),
        audio_secs: audio.len() as f64 / 16_000.0,
    })
}

/// The first report's delay from the clip's word end in milliseconds: by the wall clock in
/// real time, by the stream position in the fast pace, where the session replays the
/// service's clock and the position is known to the chunk (20 ms).
pub fn delay_ms(
    pace: Pace,
    word_end: Option<f32>,
    reported: Option<&Stamp>,
    began: Instant,
) -> Option<f64> {
    let end = word_end?;
    let reported = reported?;
    Some(match pace {
        Pace::RealTime => signed_ms(reported.wall, began + Duration::from_secs_f32(end)),
        Pace::Fast => f64::from(reported.stream - end) * 1000.0,
    })
}

fn note(heard: &mut Vec<(String, Stamp)>, events: Vec<ServerEvent>, streamed: usize) {
    let at = Stamp { wall: Instant::now(), stream: streamed as f32 / 16_000.0 };
    for event in events {
        if let ServerEvent::Command { commands, .. } = event {
            heard.extend(commands.iter().map(|c| (label_word(c).to_owned(), at)));
        }
    }
}

fn chunk_end(i: usize, len: usize) -> Duration {
    Duration::from_secs_f64((i * CHUNK + len) as f64 / 16_000.0)
}

fn wait_until(at: Instant) {
    let now = Instant::now();
    if at > now {
        std::thread::sleep(at - now);
    }
}

/// `at − since` in milliseconds, negative when `at` came first.
fn signed_ms(at: Instant, since: Instant) -> f64 {
    if at >= since {
        (at - since).as_secs_f64() * 1000.0
    } else {
        -(since - at).as_secs_f64() * 1000.0
    }
}
