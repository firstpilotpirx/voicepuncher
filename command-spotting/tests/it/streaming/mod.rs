//! The streaming recognizer on the real models: the English and Russian streaming models behind
//! the voice gate hear a command and report it.

use std::sync::{Mutex, MutexGuard, OnceLock};

use voicepuncher_command_spotting::conversions::saturating_usize;
use voicepuncher_command_spotting::protocol::{PhraseSelection, ServerEvent};
use voicepuncher_command_spotting::streaming::engine::{StreamingCfg, StreamingEngine};
use voicepuncher_test_steps::required_env;

use crate::common;

mod long_dictation;
mod quiet_start;

const SAMPLE_RATE: usize = 16_000;
const PAUSE_SECS: f32 = 1.0;

/// The phrases the app listens for in standby, when no dictation runs.
const STANDBY: [&str; 4] = [
    "start dictation",
    "press enter",
    "clear all text",
    "paste text",
];

const PHRASES: [&str; 7] = [
    "start dictation",
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
    "clear all text",
    "paste text",
];

/// ONNX Runtime aborts when models load or run on several test threads at once.
static ENGINE: OnceLock<Mutex<StreamingEngine>> = OnceLock::new();

fn engine() -> MutexGuard<'static, StreamingEngine> {
    ENGINE
        .get_or_init(|| Mutex::new(load_engine()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn load_engine() -> StreamingEngine {
    let cfg = StreamingCfg {
        en_model_dir: required_env("VOICEPUNCHER_STREAMING_EN_MODEL_DIR").into(),
        ru_model_dir: required_env("VOICEPUNCHER_STREAMING_RU_MODEL_DIR").into(),
        vad_model: required_env("VOICEPUNCHER_KWS_VAD_MODEL").into(),
        // WHY: as the service decodes the streaming models by default.
        threads: 1,
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

/// The events a session listening for `phrases` reports in `audio`, fed in 0.1 s chunks,
/// followed by the answer to a sync of `id`.
fn events_with_sync(
    engine: &StreamingEngine,
    phrases: &[&str],
    audio: &[f32],
    id: u64,
) -> Vec<ServerEvent> {
    let mut session = engine.session_for(1).expect("a session");
    let phrases = phrases.iter().map(|p| (*p).to_owned()).collect();
    session
        .select(&PhraseSelection { phrases, ..PhraseSelection::default() })
        .expect("known phrases");
    let mut events: Vec<ServerEvent> = audio
        .chunks(SAMPLE_RATE / 10)
        .flat_map(|chunk| session.accept(chunk))
        .collect();
    events.extend(session.sync(id));
    events
}

/// The commands a session listening for every phrase reports in `audio`.
fn commands_in(engine: &StreamingEngine, audio: &[f32]) -> Vec<String> {
    commands_heard(engine, &PHRASES, audio)
}

/// The commands a session listening for `phrases` reports in `audio`.
fn commands_heard(engine: &StreamingEngine, phrases: &[&str], audio: &[f32]) -> Vec<String> {
    // WHY: the recognizers decode on threads of their own; a sync waits for what is left.
    commands_of(events_with_sync(engine, phrases, &paused(audio), 0))
}

/// The commands reported in `events`, in order.
fn commands_of(events: Vec<ServerEvent>) -> Vec<String> {
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
    fn should_report_a_synthesized_paste() {
        let engine = engine();
        // When
        let commands = commands_in(&engine, &common::spoken_by(None, "paste"));
        // Then
        assert_eq!(commands, ["paste text"]);
    }

    #[test]
    fn should_report_a_command_said_right_before_a_sync_before_answering_it() {
        let engine = engine();
        let pause = vec![0.0f32; saturating_usize(PAUSE_SECS * SAMPLE_RATE as f32)];
        let audio = [pause.as_slice(), &common::spoken_by(None, "enter")].concat();
        // When the app asks for a sync right after the speech, before any pause after it
        let events = events_with_sync(&engine, &PHRASES, &audio, 7);
        // Then
        let at = |wanted: fn(&ServerEvent) -> bool| events.iter().position(wanted);
        let command = at(
            |e| matches!(e, ServerEvent::Command { commands, .. } if commands == &["press enter"]),
        )
        .expect("press enter");
        let synced = at(|e| matches!(e, ServerEvent::Synced { id: 7 })).expect("the sync");
        assert!(command < synced, "{events:?}");
    }

    #[test]
    fn should_start_nothing_for_speech_that_is_no_start() {
        let engine = engine();
        let speech = [
            (None, "we should start the meeting a bit later today"),
            (None, "the weather is nice and the coffee is hot"),
            (Some("Milena"), "давай встретимся завтра утром"),
        ];
        for (voice, words) in speech {
            // When
            let commands = commands_heard(&engine, &STANDBY, &common::spoken_by(voice, words));
            // Then
            assert!(
                !commands.contains(&"start dictation".to_owned()),
                "{words}: {commands:?}"
            );
        }
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
        fn should_report_the_users_quiet_restart_0_4_s_after_a_sentence() {
            let engine = engine();
            // Given a sentence, cut after its last sound, and 0.4 s of silence
            let sentence = common::spoken_by(None, "Coffee machine on the third floor is broken");
            let end = sentence
                .iter()
                .rposition(|s| s.abs() > 0.01)
                .map_or(0, |at| at + 1);
            let pause = vec![0.0; saturating_usize(0.4 * SAMPLE_RATE as f32)];
            // When
            let audio = [&sentence[..end], &pause, &users_quiet_restart()].concat();
            let commands = commands_in(&engine, &audio);
            // Then
            assert_eq!(commands, ["restart dictation"]);
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
