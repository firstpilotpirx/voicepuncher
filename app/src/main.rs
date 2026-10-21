use clap::Parser;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::adapters::primary::setup::cli::SetupWizard;
use voicepuncher::adapters::secondary::archiving::filesystem::{
    ArchiveSetup, FileSystemDictationArchive,
};
use voicepuncher::adapters::secondary::commandspotting::websocket::WebSocketCommandSpotter;
use voicepuncher::adapters::secondary::focus::macos::MacOsFocusProbe;
use voicepuncher::adapters::secondary::keypressing::macos::MacOsKeyPresser;
use voicepuncher::adapters::secondary::logging::file::{
    install_panic_hook, launch_header, logs_dir, terminal_stderr, RotatingFileWriter, TeeWriter,
    KEPT_BACKUPS, LOG_FILE_NAME, MAX_LOG_BYTES,
};
use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::adapters::secondary::recording::cpal::CpalAudioRecorder;
use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::adapters::secondary::textpasting::stdio::StdioTextPaster;
use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::adapters::secondary::visualfeedback::macos::{
    MacosPointerSign, MacosVisualIndicator,
};
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::PointerSignIndicator;
use voicepuncher::config::{Config, TranscriptionCfg};
use voicepuncher::core::application::hotkey_listener::HotkeyListener;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::preflight::Preflight;
use voicepuncher::core::application::visual_indicator::VisualIndicator;
use voicepuncher::core::application::CommandSpotter;
use voicepuncher::core::application::{ChunkingCfg, FocusWaitCfg, PipelineInput, TextRefiner};
use voicepuncher::core::domain::{FailureNotice, PttSignal};

struct NoopVisualIndicator;

impl VisualIndicator for NoopVisualIndicator {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
}

fn main() {
    install_panic_hook();
    let cli = Cli::parse();

    let config_path = cli.config.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".voicepuncher/config.toml")
    });

    SetupWizard::run_if_needed(&config_path, cli.setup).unwrap_or_else(|e| {
        eprintln!("setup failed: {e:#}");
        std::process::exit(2);
    });

    let mut config = voicepuncher::config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("config error: {e:#}");
        std::process::exit(2);
    });

    let input_file = cli.input_file.clone();
    let no_llm = cli.no_llm;
    let archives_file_mode = cli.archives_file_mode();
    cli.overlay(&mut config);
    voicepuncher::config::validate(&config).unwrap_or_else(|e| {
        eprintln!("config error: {e:#}");
        std::process::exit(2);
    });

    let level = config
        .logging
        .level
        .parse()
        .unwrap_or(log::LevelFilter::Info);
    if input_file.is_some() {
        init_stderr_logger(level);
    } else {
        init_file_logger(level);
    }
    log::info!("{}", launch_header(&config_path));
    if config.audio.debug_save_wav {
        log::warn!(
            "[audio] debug_save_wav is retired: the dictation archive ([archive]) now keeps \
             every recording; remove the key from the config"
        );
    }

    if let Some(ref input_path) = input_file {
        run_file_mode(input_path, &config, no_llm, archives_file_mode);
        return;
    }

    let (inputs_tx, inputs_rx) = std::sync::mpsc::channel::<PipelineInput>();
    let spotter = config
        .voice_control
        .enabled
        .then(|| connect_command_spotter(&config, inputs_tx.clone()));

    let probe = MacOsPermissionProbe::new();
    Preflight::run(&probe).unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    });

    let transcriber = connect_transcriber(&config);

    let refiner = connect_refiner(&config, no_llm);

    let paster = MacOsTextPaster::new(config.paste.clipboard_restore_delay_ms);

    let level_register = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    let recorder = CpalAudioRecorder::new(
        config.audio.max_recording_seconds,
        std::sync::Arc::clone(&level_register),
    );

    let (hotkey_tx, hotkey_rx) = std::sync::mpsc::channel::<PttSignal>();

    let listener = RdevHotkeyListener::new();
    let _handle = Box::new(listener)
        .spawn(config.hotkey.key, hotkey_tx)
        .unwrap_or_else(|e| {
            log::error!("failed to spawn hotkey listener: {e:#}");
            std::process::exit(2);
        });
    forward_hotkey_signals(hotkey_rx, inputs_tx.clone());

    let sound_player = MacosSoundPlayer::new(
        &config.sounds.start_sound,
        &config.sounds.stop_sound,
        &config.sounds.cancel_sound,
    )
    .with_voice_control_sounds(
        &config.sounds.listening_on_sound,
        &config.sounds.listening_off_sound,
    )
    .with_command_sounds(
        &config.sounds.enter_sound,
        &config.sounds.clear_sound,
        &config.sounds.paste_sound,
        &config.sounds.restart_sound,
    );

    let mtm = MainThreadMarker::new().expect("main must run on the main thread");
    let indicator = PointerSignIndicator::new(
        MacosVisualIndicator::initialize(mtm, level_register),
        MacosPointerSign::initialize(mtm),
    );

    let mut pipeline = Pipeline::new(
        Box::new(recorder),
        Box::new(transcriber),
        refiner,
        Box::new(paster),
        Box::new(sound_player),
        Box::new(indicator),
        PipelineConfig {
            min_press_ms: config.hotkey.min_press_ms,
            recording_mode: config.hotkey.recording_mode,
            chunking: chunking_cfg(&config.transcription),
        },
    )
    .with_focus_wait(
        Box::new(MacOsFocusProbe::new()),
        FocusWaitCfg {
            timeout: std::time::Duration::from_secs(config.paste.focus_wait_seconds),
            poll_interval: std::time::Duration::from_millis(100),
        },
    );
    if no_llm {
        pipeline = pipeline.without_text_refinement_stage();
    }
    if config.archive.enabled {
        pipeline = pipeline.with_archive(Box::new(connect_archive(&config, no_llm)));
    } else {
        log::info!("dictation archive: off");
    }
    if let Some(spotter) = spotter {
        log::info!("voice control on: listening starts now; the hotkey records as push-to-talk");
        pipeline = pipeline.with_voice_control(Box::new(spotter), Box::new(MacOsKeyPresser::new()));
    }

    let pipeline_handle = std::thread::Builder::new()
        .name("pipeline".into())
        .spawn(move || {
            pipeline.run(inputs_rx).unwrap_or_else(|e| {
                log::error!("fatal: {e:#}");
                std::process::exit(1);
            });
        })
        .expect("failed to spawn pipeline thread");

    let app = NSApplication::sharedApplication(mtm);
    app.run();

    pipeline_handle.join().ok();
}

/// Transcribes one WAV file and prints the text; archives it only when a folder was named.
fn run_file_mode(input_path: &std::path::Path, config: &Config, no_llm: bool, archives: bool) {
    let transcriber = connect_transcriber(config);
    let refiner = connect_refiner(config, no_llm);
    let audio = FileSystemAudioRecorder::load(input_path).unwrap_or_else(|e| {
        log::error!("file mode: {e:#}");
        std::process::exit(1);
    });
    let mut pipeline = Pipeline::new(
        Box::new(FileSystemAudioRecorder::new(input_path.to_path_buf())),
        Box::new(transcriber),
        refiner,
        Box::new(StdioTextPaster::new()),
        Box::new(MacosSoundPlayer::new("", "", "")),
        Box::new(NoopVisualIndicator),
        PipelineConfig {
            min_press_ms: config.hotkey.min_press_ms,
            recording_mode: config.hotkey.recording_mode,
            chunking: chunking_cfg(&config.transcription),
        },
    );
    if no_llm {
        pipeline = pipeline.without_text_refinement_stage();
    }
    if archives {
        pipeline = pipeline.with_archive(Box::new(connect_archive(config, no_llm)));
    }
    pipeline.run_once(audio).unwrap_or_else(|e| {
        log::error!("file mode failed: {e:#}");
        std::process::exit(1);
    });
}

/// The dictation archive in the configured folder, with the models every entry records.
fn connect_archive(config: &Config, no_llm: bool) -> FileSystemDictationArchive {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dir = config.archive.resolved_dir(&home);
    log::info!(
        "dictation archive: {} (limit {} MB)",
        dir.display(),
        config.archive.max_size_mb
    );
    let setup = ArchiveSetup {
        dictation_model: config.transcription.model.clone(),
        refinement_model: (!no_llm).then(|| config.refinement.model.clone()),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    FileSystemDictationArchive::new(dir, config.archive.max_size_bytes(), setup)
}

/// Feeds hotkey signals into the pipeline's ordered input stream.
fn forward_hotkey_signals(
    hotkey_rx: std::sync::mpsc::Receiver<PttSignal>,
    inputs_tx: std::sync::mpsc::Sender<PipelineInput>,
) {
    std::thread::Builder::new()
        .name("hotkey-forwarder".into())
        .spawn(move || {
            for signal in hotkey_rx {
                if inputs_tx.send(PipelineInput::Hotkey(signal)).is_err() {
                    break;
                }
            }
        })
        .expect("failed to spawn hotkey forwarder thread");
}

/// The command spotting adapter, once its service answers; exits like the other services.
fn connect_command_spotter(
    config: &Config,
    inputs_tx: std::sync::mpsc::Sender<PipelineInput>,
) -> WebSocketCommandSpotter {
    let spotter = WebSocketCommandSpotter::new(config.voice_control.url.clone(), inputs_tx);
    spotter.probe().unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    });
    spotter
}

fn connect_transcriber(config: &Config) -> OpenAiTranscriber {
    OpenAiTranscriber::connect(config.transcription.clone()).unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    })
}

fn connect_refiner(config: &Config, no_llm: bool) -> Box<dyn TextRefiner> {
    if no_llm {
        log::info!("text refinement disabled (--no-llm), pasting raw transcripts");
        return Box::new(PassthroughTextRefiner);
    }
    let refiner = OpenAiTextRefiner::connect(config.refinement.clone()).unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    });
    Box::new(refiner)
}

fn chunking_cfg(cfg: &TranscriptionCfg) -> ChunkingCfg {
    ChunkingCfg {
        chunk_threshold_secs: cfg.chunk_threshold_secs,
        chunk_overlap_secs: cfg.chunk_overlap_secs,
        vad_lookback_secs: cfg.vad_lookback_secs,
    }
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
