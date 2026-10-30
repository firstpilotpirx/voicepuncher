//! The streaming recognizer on the real models: the English and Russian streaming models behind
//! the voice gate hear a command and report it.

use std::sync::{Mutex, MutexGuard, OnceLock};

use voicepuncher_command_spotting::conversions::saturating_usize;
use voicepuncher_command_spotting::protocol::{PhraseSelection, ServerEvent};
use voicepuncher_command_spotting::streaming::engine::{StreamingCfg, StreamingEngine};
use voicepuncher_test_steps::required_env;

use crate::common;

const SAMPLE_RATE: usize = 16_000;
const PAUSE_SECS: f32 = 1.0;

const PHRASES: [&str; 6] = [
    "start dictation",
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
    "clear all text",
];

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE: OnceLock<Mutex<StreamingEngine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, StreamingEngine> {
    ENGINE
        .get_or_init(|| Mutex::new(load_engine()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn load_engine() -> StreamingEngine {
    let cfg = StreamingCfg {
        en_model_dir: required_env("VOICEPUNCHER_STREAMING_EN_MODEL_DIR").into(),
        ru_model_dir: required_env("VOICEPUNCHER_STREAMING_RU_MODEL_DIR").into(),
        vad_model: required_env("VOICEPUNCHER_KWS_VAD_MODEL").into(),
        // WHY: as the service decodes the streaming models by default.
        threads: 4,
        min_silence_secs: 0.3,
        command_history: None,
    };
    StreamingEngine::load(&cfg).expect("models load")
}

/// `audio` with a pause of silence before and after.
fn paused(audio: &[f32]) -> Vec<f32> {
    let pause = vec![0.0f32; saturating_usize(PAUSE_SECS * SAMPLE_RATE as f32)];
    [pause.as_slice(), audio, pause.as_slice()].concat()
}

/// `audio` as quiet as the user's quietest recorded commands: its loudest 10 ms at an RMS of
/// 0.005 (`say` speaks at about 0.24).
fn at_quiet_level(audio: &[f32]) -> Vec<f32> {
    let loudest = audio
        .chunks(SAMPLE_RATE / 100)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .fold(0.0_f32, f32::max);
    audio.iter().map(|s| s * 0.005 / loudest).collect()
}

/// The commands a session listening for every phrase reports in `audio`.
fn commands_in(engine: &StreamingEngine, audio: &[f32]) -> Vec<String> {
    let mut session = engine.session_for(1).expect("a session");
    let phrases = PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session
        .select(&PhraseSelection { phrases, ..PhraseSelection::default() })
        .expect("known phrases");
    let mut events: Vec<ServerEvent> = paused(audio)
        .chunks(SAMPLE_RATE / 10)
        .flat_map(|chunk| session.accept(chunk))
        .collect();
    // WHY: the recognizers decode on threads of their own; a sync waits for what is left.
    events.extend(session.sync(0));
    events
        .into_iter()
        .filter_map(|event| match event {
            ServerEvent::Command { commands, .. } => Some(commands),
            _ => None,
        })
        .flatten()
        .collect()
}

mod needs_streaming_models {
    use super::*;

    #[test]
    fn should_report_a_synthesized_press_enter() {
        let engine = engine();
        // When
        let commands = commands_in(&engine, &common::spoken_by(None, "press enter"));
        // Then
        assert_eq!(commands, ["press enter"]);
    }

    #[test]
    fn should_report_a_synthesized_doubled_enter_once() {
        let engine = engine();
        // When
        let commands = commands_in(&engine, &common::spoken_by(None, "enter enter"));
        // Then
        assert_eq!(commands, ["press enter"]);
    }

    #[test]
    fn should_open_the_gate_on_a_quiet_start() {
        let engine = engine();
        // When
        let quiet = at_quiet_level(&common::spoken_by(None, "start"));
        let commands = commands_in(&engine, &quiet);
        // Then
        assert_eq!(commands, ["start dictation"]);
    }

    #[test]
    fn should_report_nothing_in_a_sentence_without_a_command() {
        let engine = engine();
        // When
        let commands = commands_in(
            &engine,
            &common::spoken_by(None, "the weather is nice today"),
        );
        // Then
        assert_eq!(commands, Vec::<String>::new());
    }

    mod needs_voice_samples {
        use super::*;

        /// The user's quiet "restart" (session-2, labelled 27.23–27.52 s) with the margins the
        /// E2E scenarios use.
        fn users_quiet_restart() -> Vec<f32> {
            let root = required_env("VOICEPUNCHER_VOICE_SAMPLES");
            let audio = common::recorded_session(&root, "session-2");
            let at = |secs: f32| saturating_usize(secs * SAMPLE_RATE as f32);
            audio[at(26.93)..at(27.72)].to_vec()
        }

        #[test]
        fn should_report_the_users_quiet_restart() {
            let engine = engine();
            // When
            let commands = commands_in(&engine, &users_quiet_restart());
            // Then
            assert_eq!(commands, ["restart dictation"]);
        }
    }
}
