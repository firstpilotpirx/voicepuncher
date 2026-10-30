//! The command lines the services start with, without the program. The app's catalog starts its services with
//! the same arguments (`app/src/adapters/secondary/localservices/catalog`), and its test `command_lines` reads the
//! lists below from this file: each stays one literal list, one argument per element.

/// The `whisper-server` arguments; both speech recognition services run them, each with its own model.
///
/// Greedy, non-speech tokens suppressed, no text context: text decoded in one 30 s window of a request does not
/// steer the next (requests never share text: whisper-server clears it before every request). No text context
/// drops a request's prompt too, so a dictation request with a prompt sets its own context (`max_context` 224,
/// `carry_initial_prompt`). No temperature fallback is ignored by whisper-server 1.9.5: dictation keeps the
/// server's fallback (retries at 0.2, 0.4, … 1.0), and the command spotting service's confirmations turn it off
/// per request; the flag stays for a whisper-server that honours it. Timestamps stay on, so whisper resumes after
/// the last decoded segment instead of skipping the rest of a 30 s window when the text ends early at a pause.
/// VAD removes only silences longer than 2 s, so silence yields no text ("Thank you.") while the short pauses
/// between words stay. No language probabilities: with them, whisper-server 1.9.5 crashes on a `verbose_json`
/// request whose audio VAD leaves empty, as the command spotting service's confirmation of an utterance without
/// speech.
pub fn whisper_args(model: &str, port: &str, threads: &str, vad: &str) -> Vec<String> {
    owned(&[
        "-m",
        model,
        "--host",
        "127.0.0.1",
        "--port",
        port,
        "--inference-path",
        "/v1/audio/transcriptions",
        "-l",
        "auto",
        "-t",
        threads,
        "-bo",
        "1",
        "-nf",
        "-sns",
        "-mc",
        "0",
        "--vad",
        "-vm",
        vad,
        "-vsd",
        "2000",
        "-vp",
        "500",
        "-nlp",
    ])
}

/// The command spotting service's arguments; `--save-speech-dir` follows when `VOICEPUNCHER_KWS_SAVE_SPEECH_DIR`
/// is set.
pub fn command_spotting_args(
    port: &str,
    model_dir: &str,
    vad: &str,
    commands_url: &str,
) -> Vec<String> {
    owned(&[
        "--port",
        port,
        "--model-dir",
        model_dir,
        "--vad-model",
        vad,
        "--transcription-url",
        commands_url,
    ])
}

/// The command spotting service's arguments with the streaming recognizers
/// (`VOICEPUNCHER_COMMAND_RECOGNIZER=streaming`): no speech recognition service for commands.
pub fn streaming_args(
    port: &str,
    en_model_dir: &str,
    ru_model_dir: &str,
    vad: &str,
) -> Vec<String> {
    owned(&[
        "--port",
        port,
        "--recognizer",
        "streaming",
        "--en-model-dir",
        en_model_dir,
        "--ru-model-dir",
        ru_model_dir,
        "--vad-model",
        vad,
    ])
}

fn owned(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_string()).collect()
}
