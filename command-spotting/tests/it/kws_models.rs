//! The command spotting models the justfile finds: a test that loads them lives in a `needs_kws_models` module,
//! which the recipes leave out, and count, when the models are missing.

use std::path::PathBuf;

use voicepuncher_test_steps::required_env;

/// The keyword spotting model directory and the voice activity detection model.
pub fn kws_models() -> (PathBuf, PathBuf) {
    (
        required_env("VOICEPUNCHER_KWS_MODEL_DIR").into(),
        required_env("VOICEPUNCHER_KWS_VAD_MODEL").into(),
    )
}
