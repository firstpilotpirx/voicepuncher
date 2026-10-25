//! Speech synthesized by macOS `say`, kept on the machine: the speech cache the E2E harness
//! shares (`test-steps/src/speech_cache.rs`).

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
