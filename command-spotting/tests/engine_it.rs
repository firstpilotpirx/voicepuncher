//! The real keyword spotting model and voice activity detector on speech synthesized with
//! macOS `say`. Gated on the model paths the justfile exports.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use voicepuncher_command_spotting::engine::{Engine, EngineCfg, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::ServerEvent;

const PAUSE_SECS: f32 = 1.0;
/// As long as a user keeps listening on before saying a command.
const LONG_SILENCE_SECS: f32 = 25.0;
/// How late the reported phrase may start after its speech starts, and how far its end may be
/// from the end of its speech: the app's pause rule margins. An early start is allowed: in
/// noise the spotter may stretch a phrase back.
const LEAD_TOLERANCE_SECS: f32 = 0.3;
const TAIL_TOLERANCE_SECS: f32 = 0.5;
/// Silence streamed before the phrase set changes.
const LEAD_IN_SECS: f32 = 2.0;
const FRAME_SAMPLES: usize = 1_600;
const MIN_SILENCE_SECS: f32 = 0.4;
const ALL_PHRASES: [&str; 5] = [
    "start dictation",
    "stop dictation",
    "cancel dictation",
    "press enter",
    "clear all text",
];

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE: OnceLock<Option<Mutex<Engine>>> = OnceLock::new();

fn engine() -> Option<MutexGuard<'static, Engine>> {
    ENGINE
        .get_or_init(|| load_engine().map(Mutex::new))
        .as_ref()
        .map(|engine| engine.lock().unwrap_or_else(|e| e.into_inner()))
}

fn load_engine() -> Option<Engine> {
    let (Ok(model_dir), Ok(vad_model)) = (
        std::env::var("VOICEPUNCHER_KWS_MODEL_DIR"),
        std::env::var("VOICEPUNCHER_KWS_VAD_MODEL"),
    ) else {
        eprintln!(
            "GATED: VOICEPUNCHER_KWS_MODEL_DIR / VOICEPUNCHER_KWS_VAD_MODEL not set, skipping"
        );
        return None;
    };
    let cfg = EngineCfg {
        model_dir: PathBuf::from(model_dir),
        vad_model: PathBuf::from(vad_model),
        threads: 2,
        min_silence_secs: MIN_SILENCE_SECS,
        save_speech_dir: None,
    };
    Some(Engine::load(&cfg).expect("models load"))
}

/// `words` spoken by `say`, with a pause of silence before and after.
fn spoken(words: &str, dir: &Path) -> Vec<f32> {
    let wav = dir.join("speech.wav");
    let status = Command::new("say")
        .args([
            "-o",
            wav.to_str().unwrap(),
            "--data-format=LEI16@16000",
            words,
        ])
        .status()
        .expect("say runs");
    assert!(status.success());
    let pause = vec![0.0f32; (PAUSE_SECS * SAMPLE_RATE as f32) as usize];
    let mut reader = hound::WavReader::open(&wav).unwrap();
    let speech: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    pause
        .iter()
        .copied()
        .chain(speech)
        .chain(pause.iter().copied())
        .collect()
}

fn events_for(engine: &Engine, words: &str) -> Vec<ServerEvent> {
    let dir = std::env::temp_dir().join(format!("kws-it-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let audio = spoken(words, &dir);
    audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect()
}

fn spotted(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => Some(phrase.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn should_spot_each_phrase_spoken_on_its_own_inside_its_speech_segment() {
    let Some(engine) = engine() else { return };
    for phrase in ALL_PHRASES {
        let events = events_for(&engine, phrase);
        eprintln!("{phrase}: {events:?}");
        assert_eq!(spotted(&events), [phrase], "events: {events:?}");
        let starts = events
            .iter()
            .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
            .count();
        assert_eq!(starts, 1, "one speech segment expected: {events:?}");
    }
}

/// The speech segment and the phrase spotted in `events`.
fn segment_and_phrase(events: &[ServerEvent]) -> ((f32, f32), (f32, f32)) {
    let (mut segment, mut phrase) = (None, None);
    for event in events {
        match event {
            ServerEvent::SpeechStart { t } => segment = Some((*t, f32::MAX)),
            ServerEvent::SpeechEnd { t } => segment = segment.map(|(s, _)| (s, *t)),
            ServerEvent::Phrase { start, end, .. } => phrase = Some((*start, *end)),
        }
    }
    (
        segment.expect("speech was detected"),
        phrase.expect("the phrase was spotted"),
    )
}

fn assert_phrase_fills_segment(events: &[ServerEvent]) {
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

#[test]
fn should_report_the_phrase_within_the_bounds_of_its_speech() {
    let Some(engine) = engine() else { return };
    assert_phrase_fills_segment(&events_for(&engine, "press enter"));
}

#[test]
fn should_report_the_phrase_within_its_speech_after_a_long_silence() {
    let Some(engine) = engine() else { return };
    // Given
    let dir = std::env::temp_dir().join(format!("kws-it-long-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let mut audio = vec![0.0f32; (LONG_SILENCE_SECS * SAMPLE_RATE as f32) as usize];
    audio.extend(spoken("start dictation", &dir));
    // When
    let events: Vec<ServerEvent> = audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    // Then
    assert_phrase_fills_segment(&events);
}

#[test]
fn should_count_event_times_from_the_first_sample_across_a_phrase_change() {
    let Some(engine) = engine() else { return };
    // Given
    let dir = std::env::temp_dir().join(format!("kws-it-clock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut session = engine.session().unwrap();
    session.listen_for(&["stop dictation".to_owned()]).unwrap();
    let lead_in = vec![0.0f32; (LEAD_IN_SECS * SAMPLE_RATE as f32) as usize];
    for frame in lead_in.chunks(FRAME_SAMPLES) {
        session.accept(frame);
    }
    // When
    session.listen_for(&["press enter".to_owned()]).unwrap();
    let events: Vec<ServerEvent> = spoken("press enter", &dir)
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    // Then
    let speech_start = events
        .iter()
        .find_map(|e| match e {
            ServerEvent::SpeechStart { t } => Some(*t),
            _ => None,
        })
        .expect("speech was detected");
    let expected = LEAD_IN_SECS + PAUSE_SECS;
    assert!(
        (speech_start - expected).abs() < 0.3,
        "speech at {speech_start}, expected about {expected}: {events:?}"
    );
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; (secs * SAMPLE_RATE as f32) as usize]
}

/// Each spotted phrase with the bounds of the speech segment around it.
/// A spotted phrase, the bounds of its speech segment and its own bounds, in seconds.
type PhraseInSegment = (String, (f32, f32), (f32, f32));

fn phrases_in_segments(events: &[ServerEvent]) -> Vec<PhraseInSegment> {
    let mut found = Vec::new();
    let mut segment_start = None;
    let mut waiting: Vec<(String, (f32, f32))> = Vec::new();
    for event in events {
        match event {
            ServerEvent::SpeechStart { t } => segment_start = Some(*t),
            ServerEvent::Phrase { phrase, start, end } => {
                waiting.push((phrase.clone(), (*start, *end)))
            }
            ServerEvent::SpeechEnd { t } => {
                let start = segment_start.take().expect("speech start before its end");
                found.extend(waiting.drain(..).map(|(p, span)| (p, (start, *t), span)));
            }
        }
    }
    found
}

#[test]
fn should_spot_commands_said_one_after_another_in_one_long_stream() {
    let Some(engine) = engine() else { return };
    // Given
    let dir = std::env::temp_dir().join(format!("kws-it-many-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let mut audio = silence(5.0);
    audio.extend(spoken("start dictation", &dir));
    audio.extend(silence(10.0));
    audio.extend(spoken("press enter", &dir));
    audio.extend(silence(20.0));
    audio.extend(spoken("clear all text", &dir));
    // When
    let events: Vec<ServerEvent> = audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    // Then
    let found = phrases_in_segments(&events);
    let names: Vec<&str> = found.iter().map(|(p, _, _)| p.as_str()).collect();
    assert_eq!(
        names,
        ["start dictation", "press enter", "clear all text"],
        "{events:?}"
    );
    for (phrase, segment, span) in &found {
        assert!(
            span.0 - segment.0 < LEAD_TOLERANCE_SECS
                && (segment.1 - span.1).abs() < TAIL_TOLERANCE_SECS,
            "{phrase}: segment {segment:?} phrase {span:?}"
        );
    }
}

#[test]
fn should_place_the_phrase_alike_whether_the_audio_comes_in_small_or_large_messages() {
    let Some(engine) = engine() else { return };
    // Given
    let dir = std::env::temp_dir().join(format!("kws-it-msg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let audio = spoken("press enter", &dir);
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    // When the whole utterance arrives in one message
    let mut session = engine.session().unwrap();
    session.listen_for(&phrases).unwrap();
    let events = session.accept(&audio);
    // Then
    assert_phrase_fills_segment(&events);
}

/// Deterministic noise of about `rms` (xorshift), so every run hears the same background.
fn noise(len: usize, rms: f32) -> Vec<f32> {
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

fn spotted_in(engine: &Engine, audio: &[f32]) -> Vec<ServerEvent> {
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = ALL_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect()
}

/// About 20 dB quieter than `say`: a user sitting back from the built-in microphone.
const QUIET_GAIN: f32 = 0.1;
/// About -40 dBFS: a fan or an air conditioner in the room.
const ROOM_NOISE_RMS: f32 = 0.01;

#[test]
fn should_spot_quiet_speech() {
    let Some(engine) = engine() else { return };
    let dir = std::env::temp_dir().join(format!("kws-it-quiet-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for phrase in ALL_PHRASES {
        // Given
        let audio: Vec<f32> = spoken(phrase, &dir)
            .iter()
            .map(|s| s * QUIET_GAIN)
            .collect();
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(spotted(&events), [phrase], "quiet {phrase}: {events:?}");
        assert_phrase_fills_segment(&events);
    }
}

#[test]
fn should_spot_speech_over_room_noise() {
    let Some(engine) = engine() else { return };
    let dir = std::env::temp_dir().join(format!("kws-it-noise-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for phrase in ALL_PHRASES {
        // Given
        let mut audio = silence(LONG_SILENCE_SECS);
        audio.extend(spoken(phrase, &dir));
        let room = noise(audio.len(), ROOM_NOISE_RMS);
        let heard: Vec<f32> = audio.iter().zip(&room).map(|(s, n)| s + n).collect();
        // When
        let events = spotted_in(&engine, &heard);
        // Then
        assert_eq!(spotted(&events), [phrase], "noisy {phrase}: {events:?}");
        assert_phrase_fills_segment(&events);
    }
}
