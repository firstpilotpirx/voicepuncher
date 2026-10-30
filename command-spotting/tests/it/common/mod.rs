//! Speech synthesized by macOS `say`, kept on the machine: the speech cache the E2E harness
//! shares (`test-steps/src/speech_cache.rs`).

use std::path::Path;

use voicepuncher_test_steps::speech_cache::{synthesized, SayRequest};

/// `words` spoken by `say` in `voice` (the system's voice when `None`), 16 kHz mono, without
/// any pause.
pub fn spoken_by(voice: Option<&str>, words: &str) -> Vec<f32> {
    let request = SayRequest { voice, words_per_minute: None, text: words };
    synthesized(&request)
        .expect("say synthesizes the words")
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect()
}

/// The user's recorded session `session` of `voice_samples`, 16 kHz mono.
pub fn recorded_session(voice_samples: &str, session: &str) -> Vec<f32> {
    let path = Path::new(voice_samples).join(format!("{session}.wav"));
    hound::WavReader::open(path)
        .expect("the recording opens")
        .samples::<i16>()
        .map(|s| f32::from(s.expect("a sample")) / f32::from(i16::MAX))
        .collect()
}
