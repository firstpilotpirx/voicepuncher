//! The command lines of the services.

/// `dev-tools`'s `whisper_args`, without the program.
pub fn whisper_args(model: &str, port: u16, threads: usize, vad: &str) -> Vec<String> {
    let port = port.to_string();
    let threads = threads.to_string();
    [
        "-m",
        model,
        "--host",
        "127.0.0.1",
        "--port",
        &port,
        "--inference-path",
        "/v1/audio/transcriptions",
        "-l",
        "auto",
        "-t",
        &threads,
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
    ]
    .map(String::from)
    .to_vec()
}

/// `dev-tools`'s command spotting service line, without the program.
pub fn command_spotting_args(
    port: u16,
    model_dir: &str,
    vad: &str,
    commands_url: &str,
) -> Vec<String> {
    let port = port.to_string();
    [
        "--port",
        &port,
        "--model-dir",
        model_dir,
        "--vad-model",
        vad,
        "--transcription-url",
        commands_url,
    ]
    .map(String::from)
    .to_vec()
}
