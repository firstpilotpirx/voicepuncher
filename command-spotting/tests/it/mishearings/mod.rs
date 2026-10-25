//! Each mishearing of `voice-commands` that came from the user's own recording, heard again
//! by the real speech recognition service for commands and read by the matcher as the
//! command it stands for. The recordings stay on the machine
//! (`~/.voicepuncher/voice-samples/<session>.wav`).

use std::path::PathBuf;
use std::time::Duration;

use voicepuncher_command_spotting::confirm::UtteranceTranscriber;
use voicepuncher_command_spotting::conversions::saturating_usize;
use voicepuncher_command_spotting::matcher::PhraseMatcher;
use voicepuncher_voice_commands::{Recording, DICTATING, MISHEARINGS, STANDBY};

const SAMPLE_RATE: u32 = 16_000;
/// Heard before and after the second the command starts at: its first and last sounds.
const BEFORE_SECS: f32 = 0.3;
const AFTER_SECS: f32 = 0.5;
const DEADLINE: Duration = Duration::from_secs(20);

/// The recorded stretch of `recording`, from `voice_samples`.
fn recorded(voice_samples: &str, recording: &Recording) -> Vec<f32> {
    let path = PathBuf::from(voice_samples).join(format!("{}.wav", recording.session));
    let audio: Vec<f32> = hound::WavReader::open(path)
        .unwrap()
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / f32::from(i16::MAX))
        .collect();
    let at = |secs: f32| saturating_usize(secs * SAMPLE_RATE as f32).min(audio.len());
    audio[at(recording.at_secs - BEFORE_SECS)..at(recording.at_secs + AFTER_SECS)].to_vec()
}

/// The matcher of the state that accepts `phrase`.
fn matcher_accepting(phrase: &str) -> PhraseMatcher {
    let state = if DICTATING.contains(&phrase) {
        DICTATING
    } else {
        STANDBY
    };
    PhraseMatcher::new(&state.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>())
}

mod needs_voice_samples {
    mod needs_command_stt {
        use super::super::*;
        use voicepuncher_test_steps::required_env;

        #[test]
        fn should_read_each_recorded_mishearing_as_its_command() {
            let voice_samples = required_env("VOICEPUNCHER_VOICE_SAMPLES");
            let url = required_env("VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL");
            let transcriber = UtteranceTranscriber::new(&url).unwrap();
            let recorded_rows: Vec<_> = MISHEARINGS
                .iter()
                .filter_map(|row| row.recording.map(|r| (row, r)))
                .collect();
            assert!(!recorded_rows.is_empty(), "no mishearing names a recording");
            for (row, recording) in recorded_rows {
                // Given the recorded command
                let audio = recorded(&voice_samples, &recording);
                // When
                let heard = transcriber
                    .transcribe(&audio, SAMPLE_RATE, DEADLINE)
                    .unwrap();
                // Then
                let matcher = matcher_accepting(row.means[0]);
                let read = matcher
                    .phrase_in(&heard.text)
                    .or_else(|| matcher.near_miss(&heard.text));
                assert_eq!(
                    read.as_deref(),
                    Some(row.means[0]),
                    "{recording:?} heard as {:?}",
                    heard.text
                );
            }
        }
    }
}
