//! Commands said without a pause around them, heard by the real keyword spotter, voice
//! activity detector and speech recognition service, on speech synthesized with macOS
//! `say`. Gated on the model paths and the speech recognition service URL the justfile
//! exports.

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use voicepuncher_command_spotting::engine::{Engine, EngineCfg, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::{Placement, ServerEvent};

const STANDBY: [&str; 3] = ["start dictation", "press enter", "clear all text"];
const DICTATING: [&str; 4] = [
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
];
const FRAME_SAMPLES: usize = 1_600;
/// Silence before the speech and after it, long enough to end the utterance.
const PAUSE_SECS: f32 = 1.0;
/// A short breath between a sentence and a command: longer than the pause that ends an
/// utterance.
const BREATH_SECS: f32 = 0.35;
const MIN_SILENCE_SECS: f32 = 0.3;
const VOICE: &str = "Eddy";

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE: OnceLock<Option<Mutex<Engine>>> = OnceLock::new();

fn engine() -> Option<MutexGuard<'static, Engine>> {
    ENGINE
        .get_or_init(|| load_engine().map(Mutex::new))
        .as_ref()
        .map(|engine| engine.lock().unwrap_or_else(|e| e.into_inner()))
}

fn load_engine() -> Option<Engine> {
    let (Ok(model_dir), Ok(vad_model), Ok(transcription_url)) = (
        std::env::var("VOICEPUNCHER_KWS_MODEL_DIR"),
        std::env::var("VOICEPUNCHER_KWS_VAD_MODEL"),
        std::env::var("VOICEPUNCHER_TRANSCRIPTION_URL"),
    ) else {
        eprintln!(
            "GATED: VOICEPUNCHER_KWS_MODEL_DIR / VOICEPUNCHER_KWS_VAD_MODEL / VOICEPUNCHER_TRANSCRIPTION_URL not set, skipping"
        );
        return None;
    };
    let cfg = EngineCfg {
        model_dir: PathBuf::from(model_dir),
        vad_model: PathBuf::from(vad_model),
        threads: 2,
        min_silence_secs: MIN_SILENCE_SECS,
        save_speech_dir: None,
        keywords: None,
        transcription_url: Some(transcription_url),
        max_command_secs: 3.0,
    };
    Some(Engine::load(&cfg).expect("models load"))
}

/// `words` spoken by `say`, without any pause.
fn speech(words: &str) -> Vec<f32> {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("speech.wav");
    let status = Command::new("say")
        .args(["-v", VOICE, "-o"])
        .arg(&wav)
        .args(["--data-format=LEI16@16000", words])
        .status()
        .expect("say runs");
    assert!(status.success());
    hound::WavReader::open(&wav)
        .unwrap()
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect()
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; (secs * SAMPLE_RATE as f32) as usize]
}

/// Every event the service reports for `audio`, with a pause before and after it, while
/// it listens for `phrases`.
fn heard(engine: &Engine, phrases: &[&str], audio: &[f32]) -> Vec<ServerEvent> {
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let stream: Vec<f32> = silence(PAUSE_SECS)
        .into_iter()
        .chain(audio.iter().copied())
        .chain(silence(PAUSE_SECS))
        .collect();
    let events: Vec<ServerEvent> = stream
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    eprintln!("{events:?}");
    events
}

fn commands(events: &[ServerEvent]) -> Vec<(Vec<String>, Placement)> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Command {
                commands,
                placement,
                ..
            } => Some((commands.clone(), *placement)),
            _ => None,
        })
        .collect()
}

fn phrases(events: &[ServerEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Phrase { phrase, .. } => Some(phrase.clone()),
            _ => None,
        })
        .collect()
}

fn command(names: &[&str], placement: Placement) -> (Vec<String>, Placement) {
    (names.iter().map(|n| (*n).to_owned()).collect(), placement)
}

#[test]
fn should_hear_a_stop_right_after_the_last_dictated_word() {
    let Some(engine) = engine() else { return };
    let events = heard(
        &engine,
        &DICTATING,
        &speech("We meet on Friday at ten stop"),
    );
    assert_eq!(
        commands(&events),
        [command(&["stop dictation"], Placement::Tail)]
    );
}

#[test]
fn should_hear_enter_right_after_other_speech() {
    let Some(engine) = engine() else { return };
    let events = heard(&engine, &STANDBY, &speech("Okay, press enter"));
    // A phrase for the whole utterance would fail the app's pause rule here: only a
    // command placed at the end counts.
    assert_eq!(
        commands(&events),
        [command(&["press enter"], Placement::Tail)],
        "{events:?}"
    );
}

#[test]
fn should_hear_start_opening_an_utterance_and_place_it_before_the_speech() {
    let Some(engine) = engine() else { return };
    let events = heard(
        &engine,
        &STANDBY,
        &speech("Start, the meeting moved to Friday at ten"),
    );
    assert_eq!(
        commands(&events),
        [command(&["start dictation"], Placement::Head)]
    );
    let end = events
        .iter()
        .find_map(|e| match e {
            ServerEvent::Command { end, .. } => Some(*end),
            _ => None,
        })
        .unwrap();
    assert!(end < PAUSE_SECS + 1.2, "start ends at {end}");
}

#[test]
fn should_hear_a_doubled_stop_in_the_middle_of_speech() {
    let Some(engine) = engine() else { return };
    let events = heard(
        &engine,
        &DICTATING,
        &speech("We move the meeting to Friday stop stop and some more words here"),
    );
    assert_eq!(
        commands(&events),
        [command(&["stop dictation"], Placement::Anywhere)]
    );
}

#[test]
fn should_hear_a_chain_of_commands() {
    let Some(engine) = engine() else { return };
    let events = heard(&engine, &DICTATING, &speech("That is all, stop enter"));
    assert_eq!(
        commands(&events),
        [command(
            &["stop dictation", "press enter"],
            Placement::Anywhere
        )]
    );
}

#[test]
fn should_hear_a_stop_after_a_short_breath_as_an_utterance_of_its_own() {
    let Some(engine) = engine() else { return };
    let audio: Vec<f32> = speech("We meet on Friday")
        .into_iter()
        .chain(silence(BREATH_SECS))
        .chain(speech("stop dictation"))
        .collect();
    let events = heard(&engine, &DICTATING, &audio);
    let starts = events
        .iter()
        .filter(|e| matches!(e, ServerEvent::SpeechStart { .. }))
        .count();
    assert_eq!(starts, 2, "two utterances expected: {events:?}");
    assert!(
        phrases(&events).contains(&"stop dictation".to_owned()),
        "{events:?}"
    );
}

#[test]
fn should_hear_no_command_in_a_sentence_that_only_holds_a_command_word() {
    let Some(engine) = engine() else { return };
    let events = heard(
        &engine,
        &DICTATING,
        &speech("We should stop the server tonight"),
    );
    assert!(
        commands(&events).is_empty() && phrases(&events).is_empty(),
        "{events:?}"
    );
    let events = heard(&engine, &STANDBY, &speech("Enter the building on the left"));
    assert!(
        commands(&events).is_empty() && phrases(&events).is_empty(),
        "{events:?}"
    );
}
