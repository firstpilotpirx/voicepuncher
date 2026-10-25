//! The shared engine, the synthesized speech and what the tests read from the events.

use super::*;
use voicepuncher_command_spotting::conversions::saturating_usize;

pub(super) const ALL_PHRASES: [&str; 6] = [
    "start dictation",
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
    "clear all text",
];

/// ONNX Runtime aborts when models load or run on several test threads at once.
pub(super) static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

pub fn engine() -> MutexGuard<'static, Engine> {
    ENGINE
        .get_or_init(|| Mutex::new(load_engine()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn load_engine() -> Engine {
    Engine::load(&engine_cfg(None)).expect("models load")
}

/// The engine config, confirming utterances with the speech recognition service at
/// `transcription_url` when given.
pub fn engine_cfg(transcription_url: Option<String>) -> EngineCfg {
    let (model_dir, vad_model) = kws_models();
    EngineCfg {
        model_dir,
        vad_model,
        threads: 2,
        min_silence_secs: MIN_SILENCE_SECS,
        save_speech_dir: None,
        keywords: None,
        transcription_url,
        max_command_secs: 3.0,
        confirmation_deadline: Duration::from_secs(3),
    }
}

/// `words` spoken by `say`, with a pause of silence before and after.
pub(super) fn spoken(words: &str) -> Vec<f32> {
    let pause = vec![0.0f32; saturating_usize(PAUSE_SECS * SAMPLE_RATE as f32)];
    pause
        .iter()
        .copied()
        .chain(common::spoken_by(None, words))
        .chain(pause.iter().copied())
        .collect()
}

/// A session of `engine` listening for `phrases`.
pub(super) fn listening<'e>(engine: &'e Engine, phrases: &[&str]) -> Session<'e> {
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    session
}

pub(super) fn events_for(engine: &Engine, words: &str) -> Vec<ServerEvent> {
    let mut session = listening(engine, &ALL_PHRASES);
    let audio = spoken(words);
    fed(&mut session, &audio)
}

/// Every phrase reported alone, in order.
pub(crate) fn spotted(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => Some(phrase.clone()),
            _ => None,
        })
        .collect()
}

/// Every event the session reports for `audio` fed frame by frame, its confirmations
/// included once they are answered.
pub(super) fn fed(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let mut events: Vec<ServerEvent> = audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    events.extend(session.settle());
    events
}

/// The speech segment and the phrase spotted in `events`.
pub(super) fn segment_and_phrase(events: &[ServerEvent]) -> ((f32, f32), (f32, f32)) {
    let (mut segment, mut phrase) = (None, None);
    for event in events {
        match event {
            ServerEvent::SpeechStart { t } => segment = Some((*t, f32::MAX)),
            ServerEvent::SpeechEnd { t } => segment = segment.map(|(s, _)| (s, *t)),
            ServerEvent::Phrase { start, end, .. } => phrase = Some((*start, *end)),
            _ => {}
        }
    }
    (
        segment.expect("speech was detected"),
        phrase.expect("the phrase was spotted"),
    )
}

pub(super) fn assert_phrase_fills_segment(events: &[ServerEvent]) {
    let (segment, phrase) = segment_and_phrase(events);
    assert!(
        phrase.0 - segment.0 < LEAD_TOLERANCE_SECS,
        "lead: segment {segment:?} phrase {phrase:?}"
    );
    assert!(
        (segment.1 - phrase.1).abs() < TAIL_TOLERANCE_SECS,
        "tail: segment {segment:?} phrase {phrase:?}"
    );
}

pub(super) fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; saturating_usize(secs * SAMPLE_RATE as f32)]
}

/// Each spotted phrase with the bounds of the speech segment around it.
/// A spotted phrase, the bounds of its speech segment and its own bounds, in seconds.
pub(super) type PhraseInSegment = (String, (f32, f32), (f32, f32));

pub(super) fn phrases_in_segments(events: &[ServerEvent]) -> Vec<PhraseInSegment> {
    let mut found = Vec::new();
    let mut segment_start = None;
    let mut last_segment: Option<(f32, f32)> = None;
    let mut waiting: Vec<(String, (f32, f32))> = Vec::new();
    for event in events {
        match event {
            ServerEvent::SpeechStart { t } => segment_start = Some(*t),
            // A phrase reported after its segment ended still belongs to it: the spotter
            // may decode it after the short pause that closed the segment.
            ServerEvent::Phrase { phrase, start, end, .. } => match (segment_start, last_segment) {
                (None, Some(segment)) if *start <= segment.1 => {
                    found.push((phrase.clone(), segment, (*start, *end)));
                }
                _ => waiting.push((phrase.clone(), (*start, *end))),
            },
            ServerEvent::SpeechEnd { t } => {
                let start = segment_start.take().expect("speech start before its end");
                last_segment = Some((start, *t));
                found.extend(
                    std::mem::take(&mut waiting)
                        .into_iter()
                        .map(|(p, span)| (p, (start, *t), span)),
                );
            }
            _ => {}
        }
    }
    found
}

/// Deterministic noise of about `rms` (xorshift), so every run hears the same background.
pub(super) fn noise(len: usize, rms: f32) -> Vec<f32> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let uniform = (state >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0;
            uniform * rms * 3f32.sqrt()
        })
        .collect()
}

pub(super) fn spotted_in(engine: &Engine, audio: &[f32]) -> Vec<ServerEvent> {
    let mut session = listening(engine, &ALL_PHRASES);
    fed(&mut session, audio)
}

/// About 20 dB quieter than `say`: a user sitting back from the built-in microphone.
pub(super) const QUIET_GAIN: f32 = 0.1;

/// About -40 dBFS: a fan or an air conditioner in the room.
pub(super) const ROOM_NOISE_RMS: f32 = 0.01;

/// The loudest 10 ms of a quiet voice: the user's quietest recorded commands reach 0.0045.
pub(super) const QUIET_VOICE_LEVEL: f32 = 0.005;

pub(super) const SHORT_FORMS: [&str; 6] = ["start", "stop", "cancel", "restart", "enter", "clear"];

/// `audio` scaled so its loudest 10 ms has an RMS of `level`.
pub(super) fn at_level(audio: Vec<f32>, level: f32) -> Vec<f32> {
    let loudest = audio
        .chunks(160)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .fold(0.0_f32, f32::max);
    audio.into_iter().map(|s| s * level / loudest).collect()
}

pub(super) fn speech_segments(events: &[ServerEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
        .count()
}
