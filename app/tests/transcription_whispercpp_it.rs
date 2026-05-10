// Integration test for WhisperCppTranscriber.
// Gated: set VOICEPUNCHER_WHISPER_MODEL=<path-to-gguf> to run the real transcription test.
// Skips cleanly when the env var is unset.

use std::path::Path;

use voicepuncher::adapters::secondary::transcription::whispercpp::WhisperCppTranscriber;
use voicepuncher::config::WhisperCfg;
use voicepuncher::core::application::Transcriber;
use voicepuncher::core::domain::AudioBuffer16k;

fn default_cfg(language: &str) -> WhisperCfg {
    WhisperCfg {
        language: language.to_owned(),
        threads: 1,
        ..WhisperCfg::default()
    }
}

#[test]
fn should_skip_gracefully_when_model_env_var_unset() {
    // Given
    let model_path = match std::env::var("VOICEPUNCHER_WHISPER_MODEL") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_WHISPER_MODEL not set, skipping");
            return;
        }
    };

    // When
    let mut transcriber = WhisperCppTranscriber::load(Path::new(&model_path), default_cfg("auto"))
        .expect("model should load from VOICEPUNCHER_WHISPER_MODEL path");

    let audio = AudioBuffer16k::new(vec![0i16; 16_000]);
    let result = transcriber.transcribe(&audio);

    // Then — silence may yield an empty transcript error or an Ok; must not panic
    match result {
        Ok(_) => {}
        Err(e) => eprintln!("transcribe returned Err (acceptable for silence): {e}"),
    }
}

#[test]
fn should_fail_to_load_when_model_path_is_missing() {
    // Given
    let missing = Path::new("/nonexistent/model.bin");

    // When
    let result = WhisperCppTranscriber::load(missing, default_cfg("auto"));

    // Then
    assert!(result.is_err(), "expected Err for missing model path");
    let message = result.err().unwrap().to_string();
    assert!(
        message.contains("not found") || message.contains("nonexistent"),
        "unexpected error message: {message}"
    );
}
