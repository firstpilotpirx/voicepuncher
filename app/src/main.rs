use clap::Parser;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::adapters::primary::setup::cli::SetupWizard;
use voicepuncher::adapters::secondary::commandspotting::websocket::WebSocketCommandSpotter;
use voicepuncher::adapters::secondary::focus::macos::MacOsFocusProbe;
use voicepuncher::adapters::secondary::keypressing::macos::MacOsKeyPresser;
use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::adapters::secondary::recording::cpal::CpalAudioRecorder;
use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::adapters::secondary::textpasting::stdio::StdioTextPaster;
use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;
use voicepuncher::config::{Config, TranscriptionCfg};
use voicepuncher::core::application::hotkey_listener::HotkeyListener;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::preflight::Preflight;
use voicepuncher::core::application::visual_indicator::VisualIndicator;
use voicepuncher::core::application::CommandSpotter;
use voicepuncher::core::application::{ChunkingCfg, FocusWaitCfg, PipelineInput, TextRefiner};
use voicepuncher::core::domain::PttSignal;

struct NoopVisualIndicator;

impl VisualIndicator for NoopVisualIndicator {
    fn show_recording(&self) {}
    fn show_processing(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn hide(&self) {}
}

fn main() {
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
    cli.overlay(&mut config);
    voicepuncher::config::validate(&config).unwrap_or_else(|e| {
        eprintln!("config error: {e:#}");
        std::process::exit(2);
    });

    env_logger::Builder::new()
        .filter_level(
            config
                .logging
                .level
                .parse()
                .unwrap_or(log::LevelFilter::Info),
        )
        .init();

    if let Some(ref input_path) = input_file {
        let transcriber = connect_transcriber(&config);
        let refiner = connect_refiner(&config, no_llm);
        let audio = FileSystemAudioRecorder::load(input_path).unwrap_or_else(|e| {
            log::error!("file mode: {e:#}");
            std::process::exit(1);
        });
        let paster = StdioTextPaster::new();
        let sound_player = MacosSoundPlayer::new("", "", "");
        let mut pipeline = Pipeline::new(
            Box::new(FileSystemAudioRecorder::new(input_path.clone())),
            Box::new(transcriber),
            refiner,
            Box::new(paster),
            Box::new(sound_player),
            Box::new(NoopVisualIndicator),
            PipelineConfig {
                min_press_ms: config.hotkey.min_press_ms,
                recording_mode: config.hotkey.recording_mode,
                chunking: chunking_cfg(&config.transcription),
            },
        );
        pipeline.run_once(audio).unwrap_or_else(|e| {
            log::error!("file mode failed: {e:#}");
            std::process::exit(1);
        });
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
        config
            .audio
            .debug_save_wav
            .then(|| std::path::PathBuf::from("debug.wav")),
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
        &config.sounds.command_sound,
    );

    let mtm = MainThreadMarker::new().expect("main must run on the main thread");
    let indicator = MacosVisualIndicator::initialize(mtm, level_register);

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
    if let Some(spotter) = spotter {
        log::info!("voice control on: listening starts now; the hotkey records as push-to-talk");
        pipeline = pipeline.with_voice_control(Box::new(spotter), Box::new(MacOsKeyPresser));
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
