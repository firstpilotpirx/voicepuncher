//! One clip streamed through a streaming session, as the app streams the microphone.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::protocol::{PhraseSelection, ServerEvent};
use crate::streaming::engine::StreamingEngine;

use super::{label_word, Clip, Outcome};

/// How the audio is handed to the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// As the microphone gives it: the delay of each report is measured.
    RealTime,
    /// As fast as the session takes it: only what is heard counts.
    Fast,
}

/// 20 ms: about how often the app hands the service new audio.
const CHUNK: usize = 320;
/// Silence after the clip, so its last utterance closes.
const TAIL_SECS: f32 = 1.0;

/// Streams `clip` through a new session of `engine` listening for every command.
pub fn run_clip(engine: &StreamingEngine, clip: &Clip, pace: Pace) -> Result<Outcome> {
    let mut session = engine.session_for(0)?;
    let phrases = voicepuncher_voice_commands::COMMANDS
        .iter()
        .map(|p| (*p).to_owned())
        .collect();
    session.select(&PhraseSelection { phrases, ..PhraseSelection::default() })?;
    let tail = vec![0.0f32; crate::conversions::saturating_usize(TAIL_SECS * 16_000.0)];
    let audio = [clip.audio.as_slice(), tail.as_slice()].concat();
    let began = Instant::now();
    let mut heard: Vec<(String, Instant)> = Vec::new();
    for (i, chunk) in audio.chunks(CHUNK).enumerate() {
        if pace == Pace::RealTime {
            wait_until(began + chunk_end(i, chunk.len()));
        }
        let mut events = session.accept(chunk);
        events.extend(session.poll());
        note(&mut heard, events);
    }
    note(&mut heard, session.sync(0));
    let delay_ms = match (pace, clip.word_end, heard.first()) {
        (Pace::RealTime, Some(end), Some((_, at))) => {
            let ended = began + Duration::from_secs_f32(end);
            Some(signed_ms(*at, ended))
        }
        _ => None,
    };
    Ok(Outcome {
        id: clip.id.clone(),
        label: clip.label.clone(),
        heard: heard.into_iter().map(|(w, _)| w).collect(),
        delay_ms,
    })
}

fn note(heard: &mut Vec<(String, Instant)>, events: Vec<ServerEvent>) {
    let at = Instant::now();
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
