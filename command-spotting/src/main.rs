//! voicepuncher-command-spotting: the command spotting service of voicepuncher. Listens on
//! a loopback WebSocket; the app streams microphone audio in and gets spotted voice control
//! phrases and speech segment bounds back.

use std::net::{IpAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::Parser;

use voicepuncher_command_spotting::engine::{Engine, EngineCfg};
use voicepuncher_command_spotting::server;

#[derive(Parser, Debug)]
#[command(name = "voicepuncher-command-spotting", version, about)]
struct Args {
    /// Address to listen on; must be on this machine.
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    #[arg(long, default_value_t = 8082)]
    port: u16,
    /// Directory of the sherpa-onnx keyword spotting model (zipformer gigaspeech).
    #[arg(long)]
    model_dir: PathBuf,
    /// Silero voice activity detector model for sherpa-onnx (silero_vad.onnx).
    #[arg(long)]
    vad_model: PathBuf,
    #[arg(long, default_value_t = 2)]
    threads: i32,
    /// Silence in seconds that ends an utterance: the pause around a command.
    #[arg(long, default_value_t = 0.4)]
    min_silence_secs: f32,
    /// Save every speech segment heard as a WAV file in this directory (for tuning).
    #[arg(long)]
    save_speech_dir: Option<PathBuf>,
    /// Keyword lines to use in place of the built-in keywords.txt (for tuning).
    #[arg(long)]
    keywords_file: Option<PathBuf>,
    /// Speech recognition service that confirms short utterances (OpenAI Audio
    /// Transcriptions API on this machine); without it only the keywords count.
    #[arg(long)]
    transcription_url: Option<String>,
    /// Longest utterance, in seconds, sent to the speech recognition service.
    #[arg(long, default_value_t = 3.0)]
    max_command_secs: f32,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    if !args.host.is_loopback() {
        bail!(
            "--host {} is not on this machine; audio never leaves it",
            args.host
        );
    }
    let engine = Engine::load(&EngineCfg {
        model_dir: args.model_dir,
        vad_model: args.vad_model,
        threads: args.threads,
        min_silence_secs: args.min_silence_secs,
        save_speech_dir: args.save_speech_dir,
        keywords: args
            .keywords_file
            .map(std::fs::read_to_string)
            .transpose()
            .context("read --keywords-file")?,
        transcription_url: args.transcription_url,
        max_command_secs: args.max_command_secs,
    })?;
    let listener = TcpListener::bind((args.host, args.port))
        .with_context(|| format!("listen on {}:{}", args.host, args.port))?;
    log::info!(
        "command spotting service ready at ws://{}:{}",
        args.host,
        args.port
    );
    server::serve(listener, Arc::new(engine))
}
