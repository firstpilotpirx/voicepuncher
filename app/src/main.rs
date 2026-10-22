use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;

use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::adapters::secondary::logging::file::{
    install_panic_hook, launch_header, logs_dir, terminal_stderr, RotatingFileWriter, TeeWriter,
    KEPT_BACKUPS, LOG_FILE_NAME, MAX_LOG_BYTES,
};
use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::adapters::secondary::textpasting::stdio::StdioTextPaster;
use voicepuncher::adapters::secondary::visualfeedback::quiet::QuietVisualIndicator;
use voicepuncher::config::{Config, FileOutcome};
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use voicepuncher::core::domain::ModelNeeds;
use voicepuncher::runtime::services::SystemClock;
use voicepuncher::runtime::wiring;
use voicepuncher::runtime::CatalogModels;

mod menubar_app;

fn main() -> ExitCode {
    install_panic_hook();
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            // before the logger is up, the error output is the only place to say why
            if log::max_level() == log::LevelFilter::Off {
                eprintln!("{e:#}");
            } else {
                log::error!("{e:#}");
            }
            ExitCode::from(2)
        }
    }
}

/// The mode the command line picks: the download, one file, or the menu bar app.
fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(|| home().join(".voicepuncher/config.toml"));
    if cli.download_models {
        let code = download_models(&cli, &config_path)?;
        return Ok(ExitCode::from(u8::try_from(code).unwrap_or(2)));
    }
    let (config, config_file) = load_config(&cli, &config_path)?;
    let level = config
        .logging
        .level
        .parse()
        .unwrap_or(log::LevelFilter::Info);
    if cli.input_file.is_some() {
        init_stderr_logger(level);
    } else {
        init_file_logger(level);
    }
    log::info!("{}", launch_header(&config_path));
    if config_file == FileOutcome::Created {
        log::info!(
            "config: created {} with the defaults",
            config_path.display()
        );
    }
    match cli.input_file.clone() {
        Some(input_path) => run_file_mode(&input_path, &config, cli.archives_file_mode()),
        None => menubar_app::run(cli, config, config_path),
    }
}

/// The configuration file with this run's command line over it; a first launch writes the
/// defaults (the settings window changes them, nothing is asked).
fn load_config(cli: &Cli, config_path: &Path) -> anyhow::Result<(Config, FileOutcome)> {
    let config_file = voicepuncher::config::ensure_exists(config_path).context("config error")?;
    let mut config = voicepuncher::config::load(config_path).context("config error")?;
    cli.overlay(&mut config);
    voicepuncher::config::validate(&config).context("config error")?;
    Ok((config, config_file))
}

pub(crate) fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// `--download-models`: the models folder of the command line, else of the configuration
/// file when there is one; every catalog file but the text refinement model with `--no-llm`.
fn download_models(cli: &Cli, config_path: &Path) -> anyhow::Result<i32> {
    let mut config = if config_path.exists() {
        voicepuncher::config::load(config_path).context("config error")?
    } else {
        Config::default()
    };
    if let Some(dir) = &cli.models_dir {
        config.services.models_dir = dir.clone();
    }
    let needs = ModelNeeds { voice_control: true, refinement: !cli.no_llm };
    Ok(voicepuncher::runtime::models::download_headless(
        &CatalogModels::current(),
        config.services.resolved_models_dir(&home()),
        needs,
        Arc::new(SystemClock),
        &mut std::io::stderr(),
    ))
}

/// Transcribes one WAV file and prints the text; archives it only when a folder was named.
fn run_file_mode(input_path: &Path, config: &Config, archives: bool) -> anyhow::Result<ExitCode> {
    // the recording is read before any service is asked, so a bad file ends with 1
    let audio = match FileSystemAudioRecorder::load(input_path) {
        Ok(audio) => audio,
        Err(e) => {
            log::error!("file mode: {e:#}");
            return Ok(ExitCode::from(1));
        }
    };
    let transcriber = wiring::connect_transcriber(config)?;
    let refiner = wiring::connect_refiner(config)?;
    let mut pipeline = Pipeline::new(
        PipelineParts {
            recorder: Box::new(FileSystemAudioRecorder::new(input_path.to_path_buf())),
            transcriber: Box::new(transcriber),
            refiner,
            paster: Box::new(StdioTextPaster::new()),
            sound_player: Box::new(MacosSoundPlayer::new("", "", "")),
            visual_indicator: Box::new(QuietVisualIndicator),
        },
        PipelineConfig {
            min_press_ms: config.hotkey.min_press_ms,
            recording_mode: config.hotkey.recording_mode,
            chunking: wiring::chunking_cfg(&config.transcription),
            ..PipelineConfig::default()
        },
    );
    if !config.refinement.enabled {
        pipeline = pipeline.without_text_refinement_stage();
    }
    if archives {
        pipeline = pipeline.with_archive(Box::new(wiring::connect_archive(config)));
    }
    match pipeline.run_once(audio) {
        Ok(Some(text)) => eprintln!("▸ {}", text.as_str()),
        Ok(None) => {}
        Err(e) => {
            log::error!("file mode failed: {e:#}");
            return Ok(ExitCode::from(1));
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// File mode is a command line tool: its log goes to the error output only.
fn init_stderr_logger(level: log::LevelFilter) {
    env_logger::Builder::new()
        .filter_level(level)
        .format(log_line)
        .init();
}

/// Hotkey mode: the log file in the logs folder, plus the terminal when there is one; the
/// error output alone when the file cannot be opened.
fn init_file_logger(level: log::LevelFilter) {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let path = logs_dir(&home).join(LOG_FILE_NAME);
    let file = match RotatingFileWriter::open(&path, MAX_LOG_BYTES, KEPT_BACKUPS) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("log file {} cannot be opened: {e}", path.display());
            return init_stderr_logger(level);
        }
    };
    let tee = TeeWriter::new(file, terminal_stderr());
    env_logger::Builder::new()
        .filter_level(level)
        .format(log_line)
        .target(env_logger::Target::Pipe(Box::new(tee)))
        .init();
}

/// One log line with the thread that wrote it, so a session traces across the pipeline,
/// command spotting and transcription threads: `[time LEVEL thread target] message`.
fn log_line(buf: &mut env_logger::fmt::Formatter, record: &log::Record) -> std::io::Result<()> {
    use std::io::Write;
    let thread = std::thread::current();
    writeln!(
        buf,
        "[{} {:<5} {} {}] {}",
        buf.timestamp(),
        record.level(),
        thread.name().unwrap_or("-"),
        record.target(),
        record.args()
    )
}
