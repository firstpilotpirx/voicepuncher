use clap::Parser;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::adapters::primary::setup::cli::SetupWizard;
use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::adapters::secondary::recording::cpal::CpalAudioRecorder;
use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::adapters::secondary::textpasting::stdio::StdioTextPaster;
use voicepuncher::adapters::secondary::textrefinement::llamacpp::LlamaCppTextRefiner;
use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::adapters::secondary::transcription::whispercpp::WhisperCppTranscriber;
use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;
use voicepuncher::core::application::hotkey_listener::HotkeyListener;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::preflight::Preflight;
use voicepuncher::core::application::visual_indicator::VisualIndicator;
use voicepuncher::core::application::ChunkingCfg;
use voicepuncher::core::domain::PttSignal;

struct NoopVisualIndicator;

impl VisualIndicator for NoopVisualIndicator {
    fn show_recording(&self) {}
    fn show_processing(&self) {}
    fn hide(&self) {}
}

fn main() {
    whisper_rs::install_logging_hooks();
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

    let mut config = voicepuncher::config::load(&config_path)
        .unwrap_or_else(|_| voicepuncher::config::Config::default());

    let input_file = cli.input_file.clone();
    let no_llm = cli.no_llm;
    cli.overlay(&mut config);

    env_logger::Builder::new()
        .filter_level(
            config
                .logging
                .level
                .parse()
                .unwrap_or(log::LevelFilter::Info),
        )
        .filter_module("whisper_rs", log::LevelFilter::Warn)
        .filter_module("ggml", log::LevelFilter::Warn)
        .init();

    if let Some(ref input_path) = input_file {
        let transcriber =
            WhisperCppTranscriber::load(&config.whisper.model_path, config.whisper.clone())
                .unwrap_or_else(|e| {
                    log::error!("failed to load whisper model: {e:#}");
                    std::process::exit(2);
                });
        let refiner: Box<dyn voicepuncher::core::application::TextRefiner> = if no_llm {
            log::info!("LLM disabled (--no-llm), using passthrough");
            Box::new(PassthroughTextRefiner)
        } else {
            Box::new(
                LlamaCppTextRefiner::load(
                    &config.llm.model_path,
                    config.llm.system_prompt.clone(),
                    config.llm.max_tokens as i32,
                    config.llm.temperature,
                    config.llm.context_size,
                )
                .unwrap_or_else(|e| {
                    log::error!("failed to load llama model: {e:#}");
                    std::process::exit(2);
                }),
            )
        };
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
                chunking: ChunkingCfg {
                    chunk_threshold_secs: config.whisper.chunk_threshold_secs,
                    chunk_overlap_secs: config.whisper.chunk_overlap_secs,
                    vad_rms_threshold: config.whisper.vad_rms_threshold,
                    vad_lookback_secs: config.whisper.vad_lookback_secs,
                },
            },
        );
        pipeline.run_once(audio).unwrap_or_else(|e| {
            log::error!("file mode failed: {e:#}");
            std::process::exit(1);
        });
        return;
    }

    let probe = MacOsPermissionProbe::new();
    Preflight::run(&probe).unwrap_or_else(|e| {
        log::error!("{e:#}");
        std::process::exit(2);
    });

    let transcriber =
        WhisperCppTranscriber::load(&config.whisper.model_path, config.whisper.clone())
            .unwrap_or_else(|e| {
                log::error!("failed to load whisper model: {e:#}");
                std::process::exit(2);
            });

    let refiner: Box<dyn voicepuncher::core::application::TextRefiner> = if no_llm {
        log::info!("LLM disabled (--no-llm), using passthrough");
        Box::new(PassthroughTextRefiner)
    } else {
        Box::new(
            LlamaCppTextRefiner::load(
                &config.llm.model_path,
                config.llm.system_prompt.clone(),
                config.llm.max_tokens as i32,
                config.llm.temperature,
                config.llm.context_size,
            )
            .unwrap_or_else(|e| {
                log::error!("failed to load llama model: {e:#}");
                std::process::exit(2);
            }),
        )
    };

    let paster = MacOsTextPaster::new(config.paste.clipboard_restore_delay_ms);

    let recorder = CpalAudioRecorder::new(
        config
            .audio
            .debug_save_wav
            .then(|| std::path::PathBuf::from("debug.wav")),
        config.audio.max_recording_seconds,
    );

    let (tx, rx) = std::sync::mpsc::channel::<PttSignal>();

    let listener = RdevHotkeyListener::new();
    let _handle = Box::new(listener)
        .spawn(config.hotkey.key, tx)
        .unwrap_or_else(|e| {
            log::error!("failed to spawn hotkey listener: {e:#}");
            std::process::exit(2);
        });

    let sound_player = MacosSoundPlayer::new(
        &config.sounds.start_sound,
        &config.sounds.stop_sound,
        &config.sounds.cancel_sound,
    );

    let mtm = MainThreadMarker::new().expect("main must run on the main thread");
    let indicator = MacosVisualIndicator::initialize(mtm);

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
            chunking: ChunkingCfg {
                chunk_threshold_secs: config.whisper.chunk_threshold_secs,
                chunk_overlap_secs: config.whisper.chunk_overlap_secs,
                vad_rms_threshold: config.whisper.vad_rms_threshold,
                vad_lookback_secs: config.whisper.vad_lookback_secs,
            },
        },
    );

    let pipeline_handle = std::thread::Builder::new()
        .name("pipeline".into())
        .spawn(move || {
            pipeline.run(rx).unwrap_or_else(|e| {
                log::error!("fatal: {e:#}");
                std::process::exit(1);
            });
        })
        .expect("failed to spawn pipeline thread");

    let app = NSApplication::sharedApplication(mtm);
    app.run();

    pipeline_handle.join().ok();
}
