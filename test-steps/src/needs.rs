//! The needs of the tests: a service, a model, recordings or the desktop.
//!
//! A test that needs one lives in a module named after the need
//! (`needs_stt`, `needs_kws_models`, …); the gates of `dev-tools` leave it out, and count it, when the need is
//! missing. A test that runs anyway fails here, naming what it needs.

/// The value of `variable`, which the recipe that runs the test sets.
///
/// Panics naming the variable and that recipe when it is unset or empty.
pub fn required_env(variable: &str) -> String {
    match std::env::var(variable) {
        Ok(value) if !value.is_empty() => value,
        _ => panic!(
            "{variable} is not set: this test needs it, and only {} runs it (the gates of `dev-tools` leave it \
             out otherwise)",
            recipe_setting(variable)
        ),
    }
}

/// The recipe that sets `variable` for the tests that need it.
fn recipe_setting(variable: &str) -> &'static str {
    match variable.trim_start_matches("VOICEPUNCHER_") {
        "KWS_MODEL_DIR" | "KWS_VAD_MODEL" => "`just check` after `just models`",
        "VOICE_SAMPLES" => "`just e2e-smoke` with recordings in ~/.voicepuncher/voice-samples",
        "BENCH_CORPUS" => "`just check` after `just corpus`",
        "DESKTOP_IT" => "`just it-real`",
        "REAL_KEYBOARD_IT" => "`just test-real-keyboard`",
        "HANG_PROBE" => "`VOICEPUNCHER_HANG_PROBE=1 just test`",
        "MACHINE_LOCK" => "`just test` with agent-kit's machine lock",
        _ => "`just check` (its model stages start the services)",
    }
}

#[cfg(test)]
mod tests;
