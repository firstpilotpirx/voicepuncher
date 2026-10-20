//! Commands said without a pause around them, heard by the real keyword spotter, voice
//! activity detector and speech recognition service for commands, on speech synthesized
//! with macOS `say`. Gated on the model paths and the URL of the speech recognition service
//! for commands the justfile exports, so the engine confirms as the command spotting service
//! does.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

mod common;
mod scripted_hearing;

use scripted_hearing::{ScriptedHearing, ScriptedWord};
use voicepuncher_command_spotting::engine::{Engine, EngineCfg, Session, SAMPLE_RATE};
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
/// Quieter samples are silence.
const SILENCE_LEVEL: f32 = 0.01;
const VOICE: &str = "Eddy";
/// How long the engine waits for one confirmation: the command spotting service's default.
const CONFIRMATION_DEADLINE: Duration = Duration::from_secs(3);
/// A slow speech recognition service for commands, as under GPU load: within the deadline.
const SLOW_HEARING: Duration = Duration::from_secs(2);
/// Between two words of a scripted sentence: no pause that would end the utterance.
const WORD_GAP_SECS: f32 = 0.06;

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE: OnceLock<Option<Mutex<Engine>>> = OnceLock::new();

fn engine() -> Option<MutexGuard<'static, Engine>> {
    ENGINE
        .get_or_init(|| load_engine().map(Mutex::new))
        .as_ref()
        .map(|engine| engine.lock().unwrap_or_else(|e| e.into_inner()))
}

fn load_engine() -> Option<Engine> {
    let Ok(transcription_url) = std::env::var("VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL") else {
        eprintln!("GATED: VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL not set, skipping");
        return None;
    };
    let cfg = engine_cfg(Some(transcription_url))?;
    Some(Engine::load(&cfg).expect("models load"))
}

/// The engine config, confirming with the speech recognition service at `transcription_url`
/// when given; none without the models.
fn engine_cfg(transcription_url: Option<String>) -> Option<EngineCfg> {
    let (Ok(model_dir), Ok(vad_model)) = (
        std::env::var("VOICEPUNCHER_KWS_MODEL_DIR"),
        std::env::var("VOICEPUNCHER_KWS_VAD_MODEL"),
    ) else {
        eprintln!(
            "GATED: VOICEPUNCHER_KWS_MODEL_DIR / VOICEPUNCHER_KWS_VAD_MODEL not set, skipping"
        );
        return None;
    };
    Some(EngineCfg {
        model_dir: PathBuf::from(model_dir),
        vad_model: PathBuf::from(vad_model),
        threads: 2,
        min_silence_secs: MIN_SILENCE_SECS,
        save_speech_dir: None,
        keywords: None,
        transcription_url,
        max_command_secs: 3.0,
        confirmation_deadline: CONFIRMATION_DEADLINE,
    })
}

/// `words` spoken by `say`, without any pause.
fn speech(words: &str) -> Vec<f32> {
    common::spoken_by(Some(VOICE), words)
}

/// `say` speech from its first sound to its last.
fn sound_only(audio: Vec<f32>) -> Vec<f32> {
    let loud = |s: &f32| s.abs() > SILENCE_LEVEL;
    let first = audio.iter().position(loud).unwrap_or(0);
    let last = audio.iter().rposition(loud).map_or(first, |at| at + 1);
    audio[first..last].to_vec()
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
    let events: Vec<ServerEvent> = fed(&mut session, &stream);
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

/// Every event the session reports for `audio` fed frame by frame, its confirmations
/// included once they are answered.
fn fed(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let mut events: Vec<ServerEvent> = audio
        .chunks(FRAME_SAMPLES)
        .flat_map(|frame| session.accept(frame))
        .collect();
    events.extend(session.settle());
    events
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
    // `say` ends a phrase with about 0.4 s of its own silence: the breath is cut from the
    // last sound to the next.
    let audio: Vec<f32> = sound_only(speech("We meet on Friday"))
        .into_iter()
        .chain(silence(BREATH_SECS))
        .chain(sound_only(speech("stop dictation")))
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

/// ONNX Runtime aborts when models load or run on several test threads at once.
static SCRIPTED_RUNS: Mutex<()> = Mutex::new(());

/// `sentence` said word by word by `say`, between pauses, and where each word lies.
fn scripted(sentence: &str) -> (Vec<f32>, Vec<ScriptedWord>) {
    let mut stream = silence(PAUSE_SECS);
    let mut script = Vec::new();
    for word in sentence.split_whitespace() {
        let said = sound_only(speech(word));
        script.push(ScriptedWord {
            text: word.to_owned(),
            said: stream.len()..stream.len() + said.len(),
        });
        stream.extend(said);
        stream.extend(silence(WORD_GAP_SECS));
    }
    stream.extend(silence(PAUSE_SECS));
    (stream, script)
}

/// Every event for `sentence`, fed as fast as it is spoken, while the service listens for
/// `phrases` and each confirmation takes `delay`.
fn heard_in_time(phrases: &[&str], sentence: &str, delay: Duration) -> Option<Vec<ServerEvent>> {
    let _one_at_a_time = SCRIPTED_RUNS.lock().unwrap_or_else(|e| e.into_inner());
    let cfg = engine_cfg(None)?;
    let (stream, script) = scripted(sentence);
    let hearing = ScriptedHearing {
        stream: stream.clone(),
        script,
        delay,
    };
    let engine = Engine::load(&cfg)
        .expect("models load")
        .hearing(Arc::new(hearing));
    let mut session = engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let events = fed_as_spoken(&mut session, &stream);
    eprintln!("{events:?}");
    Some(events)
}

/// Every event the session reports for `audio` fed one frame per frame of time on a fixed
/// schedule, its confirmations included once they are answered.
fn fed_as_spoken(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let frame_time = Duration::from_secs_f32(FRAME_SAMPLES as f32 / SAMPLE_RATE as f32);
    let mut due = Instant::now();
    let mut events = Vec::new();
    for frame in audio.chunks(FRAME_SAMPLES) {
        events.extend(session.accept(frame));
        due += frame_time;
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    events.extend(session.settle());
    events
}

/// How many times `phrase` was carried out in a command said without a pause.
fn times_carried_out(events: &[ServerEvent], phrase: &str) -> usize {
    commands(events)
        .iter()
        .flat_map(|(names, _)| names.clone())
        .filter(|name| name == phrase)
        .count()
}

#[test]
fn should_restart_once_in_a_dictation_while_confirmations_come_back_slowly() {
    let Some(events) = heard_in_time(
        &DICTATING,
        "We moved the weekly meeting to Friday restart restart and the notes begin here with a few more words for everyone",
        SLOW_HEARING,
    ) else {
        return;
    };
    assert_eq!(
        times_carried_out(&events, "restart dictation"),
        1,
        "{events:?}"
    );
}

/// The commands carried out, in order, whether said in one chain or in several.
fn carried_out(events: &[ServerEvent]) -> Vec<String> {
    commands(events)
        .into_iter()
        .flat_map(|(names, _)| names)
        .collect()
}

/// The chain said without pauses in a dictation, and the commands it carries out in order.
const CHAIN_SENTENCE: &str =
    "We moved the weekly meeting stop enter start and the notes begin here";
const CHAIN: [&str; 3] = ["stop dictation", "press enter", "start dictation"];

/// Asserts the chain is carried out when each confirmation takes `hearing`. One test per speed,
/// so nextest runs them in processes of their own at the same time.
fn assert_chain_carried_out(hearing: Duration) {
    let listening: Vec<&str> = DICTATING.into_iter().chain(["start dictation"]).collect();
    let Some(events) = heard_in_time(&listening, CHAIN_SENTENCE, hearing) else {
        return;
    };
    assert_eq!(
        carried_out(&events),
        CHAIN,
        "hearing in {hearing:?}: {events:?}"
    );
}

#[test]
fn should_carry_out_the_chain_when_confirmations_come_back_at_once() {
    assert_chain_carried_out(Duration::ZERO);
}

#[test]
fn should_carry_out_the_same_chain_when_confirmations_come_back_slowly() {
    assert_chain_carried_out(SLOW_HEARING);
}
