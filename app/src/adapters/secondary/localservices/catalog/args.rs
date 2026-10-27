//! The command lines of the services.

use std::path::Path;

use crate::config::ArchiveCfg;

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
    history: Option<&CommandHistoryArgs>,
) -> Vec<String> {
    let port = port.to_string();
    let mut args: Vec<String> = [
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
    .to_vec();
    if let Some(history) = history {
        args.extend([
            "--command-history-dir".to_owned(),
            history.dir.clone(),
            "--command-history-max-mb".to_owned(),
            history.max_size_mb.to_string(),
        ]);
    }
    args
}

/// Where a command spotting service the app starts keeps the command history, and how much.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandHistoryArgs {
    pub dir: String,
    pub max_size_mb: u64,
}

impl CommandHistoryArgs {
    /// The command history `archive` keeps under `home`; none when it keeps no commands.
    pub fn of(archive: &ArchiveCfg, home: &Path) -> Option<Self> {
        let dir = archive.command_history_dir(home)?;
        Some(Self { dir: dir.display().to_string(), max_size_mb: archive.max_size_mb })
    }
}
