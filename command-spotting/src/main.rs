//! voicepuncher-command-spotting: the command spotting service of voicepuncher. Listens on
//! a loopback WebSocket; the app streams microphone audio in and gets the voice commands
//! heard and speech segment bounds back.

use std::net::{IpAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::Parser;

use voicepuncher_command_spotting::history::HistoryCfg;
use voicepuncher_command_spotting::server;
use voicepuncher_command_spotting::streaming::engine::{StreamingCfg, StreamingEngine};

/// The streaming models decode on their own interactive thread alone, as ONNX Runtime's extra
/// threads run at the default service class and wait on a busy machine.
const DEFAULT_THREADS: i32 = 1;

#[derive(Parser, Debug)]
#[command(name = "voicepuncher-command-spotting", version, about)]
struct Args {
    /// Address to listen on; must be on this machine.
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    #[arg(long, default_value_t = 8082)]
    port: u16,
    /// Directory of the English streaming model (Nemotron).
    #[arg(long)]
    en_model_dir: PathBuf,
    /// Directory of the Russian streaming model (zipformer vosk).
    #[arg(long)]
    ru_model_dir: PathBuf,
    /// Silero voice activity detector model for sherpa-onnx (`silero_vad.onnx`).
    #[arg(long)]
    vad_model: PathBuf,
    /// Threads each model decodes on.
    #[arg(long, default_value_t = DEFAULT_THREADS)]
    threads: i32,
    /// Silence in seconds that ends an utterance: the pause around a command.
    #[arg(long, default_value_t = 0.3)]
    min_silence_secs: f32,
    #[command(flatten)]
    history: HistoryArgs,
}

#[derive(clap::Args, Debug)]
struct HistoryArgs {
    /// Keep every utterance with a command heard in this directory (the command history);
    /// without it nothing is kept.
    #[arg(long)]
    command_history_dir: Option<PathBuf>,
    /// Size limit of the command history in megabytes: the oldest entries go first.
    #[arg(long, default_value_t = 2048)]
    command_history_max_mb: u64,
}

impl HistoryArgs {
    fn cfg(self) -> Option<HistoryCfg> {
        let max_size_bytes = self.command_history_max_mb.saturating_mul(1024 * 1024);
        self.command_history_dir
            .map(|dir| HistoryCfg { dir, max_size_bytes })
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(voicepuncher_log_line::log_line)
        .init();
    let args = Args::parse();
    if !args.host.is_loopback() {
        bail!(
            "--host {} is not on this machine; audio never leaves it",
            args.host
        );
    }
    let (host, port) = (args.host, args.port);
    let engine = StreamingEngine::load(&StreamingCfg {
        en_model_dir: args.en_model_dir,
        ru_model_dir: args.ru_model_dir,
        vad_model: args.vad_model,
        threads: args.threads,
        min_silence_secs: args.min_silence_secs,
        command_history: args.history.cfg(),
    })?;
    let listener =
        TcpListener::bind((host, port)).with_context(|| format!("listen on {host}:{port}"))?;
    log::info!("command spotting service ready at ws://{host}:{port}");
    server::serve(&listener, &Arc::new(engine))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The service's arguments with its models, followed by `more`.
    fn args(more: &[&str]) -> Result<Args, clap::Error> {
        let base = ["service", "--en-model-dir", "en", "--ru-model-dir", "ru"];
        Args::try_parse_from(base.iter().chain(&["--vad-model", "v"]).chain(more))
    }

    #[test]
    fn should_end_an_utterance_after_0_3_s_of_silence_by_default() {
        // When
        let args = args(&[]).unwrap();
        // Then
        assert!(
            (args.min_silence_secs - 0.3).abs() < 1e-6,
            "{} is not {}",
            args.min_silence_secs,
            0.3
        );
    }

    #[test]
    fn should_keep_no_command_history_without_its_directory() {
        // When
        let args = args(&[]).unwrap();
        // Then
        assert_eq!(args.history.cfg(), None);
    }

    #[test]
    fn should_take_the_streaming_models() {
        // When
        let args = args(&[]).unwrap();
        // Then
        assert_eq!(args.en_model_dir, PathBuf::from("en"));
        assert_eq!(args.ru_model_dir, PathBuf::from("ru"));
    }

    #[test]
    fn should_refuse_to_start_without_the_russian_model() {
        // When
        let args = Args::try_parse_from(["service", "--en-model-dir", "en", "--vad-model", "v"]);
        // Then
        assert!(args.is_err(), "{args:?}");
    }

    #[test]
    fn should_decode_the_streaming_models_on_1_thread_by_default() {
        // When
        let args = args(&[]).unwrap();
        // Then
        assert_eq!(args.threads, 1);
    }

    #[test]
    fn should_take_the_threads_asked_for() {
        // When
        let args = args(&["--threads", "6"]).unwrap();
        // Then
        assert_eq!(args.threads, 6);
    }

    #[test]
    fn should_refuse_the_retired_keyword_spotter_arguments() {
        // When
        let args = args(&["--recognizer", "keyword-spotting"]);
        // Then
        assert!(args.is_err(), "{args:?}");
    }
}
