// Integration test for LlamaCppTextRefiner.
// Gated: set VOICEPUNCHER_LLM_MODEL=<path-to-gguf> to run real inference.
// Skips cleanly when env var is unset.

use std::path::Path;

use voicepuncher::adapters::secondary::textrefinement::llamacpp::LlamaCppTextRefiner;
use voicepuncher::core::application::text_refiner::TextRefiner;
use voicepuncher::core::domain::RawTranscript;

#[test]
fn should_fail_to_load_when_model_path_is_missing() {
    // Given
    let missing_path = Path::new("/nonexistent/model.gguf");

    // When
    let result = LlamaCppTextRefiner::load(
        missing_path,
        "You are a transcript cleaner.".to_owned(),
        128,
        0.0,
        512,
    );

    // Then
    assert!(result.is_err());
    let msg = result.err().unwrap().to_string().to_lowercase();
    assert!(
        msg.contains("not found") || msg.contains("nonexistent"),
        "expected error message to mention 'not found' or 'nonexistent', got: {msg}"
    );
}

#[test]
fn should_refine_transcript_or_skip_when_env_unset() {
    // Given
    let model_path = match std::env::var("VOICEPUNCHER_LLM_MODEL") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_LLM_MODEL not set, skipping");
            return;
        }
    };

    let mut refiner = LlamaCppTextRefiner::load(
        Path::new(&model_path),
        "Clean up the transcript. Remove filler words. Return only the cleaned text.".to_owned(),
        256,
        0.2,
        512,
    )
    .expect("model should load from VOICEPUNCHER_LLM_MODEL path");

    let input = "hello world uh i think the api is fine";
    let raw = RawTranscript::new(input.to_owned()).unwrap();

    // When
    let result = refiner.refine(&raw).expect("refine should succeed");

    // Then
    let output_len = result.as_str().len();
    let min_len = input.len() / 2;
    let max_len = input.len() * 2;
    assert!(
        !result.as_str().is_empty(),
        "refined text must not be empty"
    );
    assert!(
        output_len >= min_len && output_len <= max_len,
        "refined text length {output_len} is outside sanity band [{min_len}, {max_len}]"
    );
}
