use clap::Parser;

use voicepuncher::adapters::primary::rdev_hotkey_listener::RdevHotkeyListener;
use voicepuncher::adapters::primary::setup_wizard::SetupWizard;
use voicepuncher::adapters::secondary::cpal_audio_recorder::CpalAudioRecorder;
use voicepuncher::adapters::secondary::file_system_audio_recorder::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::llamacpp_text_refiner::LlamaCppTextRefiner;
use voicepuncher::adapters::secondary::macos_permission_probe::MacOsPermissionProbe;
use voicepuncher::adapters::secondary::macos_sound_player::MacosSoundPlayer;
use voicepuncher::adapters::secondary::macos_text_paster::MacOsTextPaster;
use voicepuncher::adapters::secondary::stdio_text_paster::StdioTextPaster;
use voicepuncher::adapters::secondary::whispercpp_transcriber::WhisperCppTranscriber;
use voicepuncher::cli::Cli;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::ports::hotkey_listener::HotkeyListener;
use voicepuncher::core::application::preflight::Preflight;
use voicepuncher::core::domain::PttSignal;
use std::sync::{Arc, atomic::AtomicBool};

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

    let mut config = voicepuncher::config::load(&config_path)
        .unwrap_or_else(|_| voicepuncher::config::Config::default());

    let input_file = cli.input_file.clone();
    cli.overlay(&mut config);

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
        let transcriber = WhisperCppTranscriber::load(
            &config.whisper.model_path,
            &config.whisper.language,
            config.whisper.threads,
        )
        .unwrap_or_else(|e| {
            log::error!("failed to load whisper model: {e:#}");
            std::process::exit(2);
        });
        let refiner = LlamaCppTextRefiner::load(
            &config.llm.model_path,
            config.llm.system_prompt.clone(),
            config.llm.max_tokens as i32,
            config.llm.temperature,
            config.llm.context_size,
        )
        .unwrap_or_else(|e| {
            log::error!("failed to load llama model: {e:#}");
            std::process::exit(2);
        });
        let audio = FileSystemAudioRecorder::load(input_path).unwrap_or_else(|e| {
            log::error!("file mode: {e:#}");
            std::process::exit(1);
        });
        let paster = StdioTextPaster::new();
        let sound_player = MacosSoundPlayer::new("", "");
        let mut pipeline = Pipeline::new(
            Box::new(FileSystemAudioRecorder::new(input_path.clone())),
            Box::new(transcriber),
            Box::new(refiner),
            Box::new(paster),
            Box::new(sound_player),
            PipelineConfig {
                min_press_ms: config.hotkey.min_press_ms,
                recording_mode: config.hotkey.recording_mode,
            },
            Arc::new(AtomicBool::new(false)),
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

    let transcriber = WhisperCppTranscriber::load(
        &config.whisper.model_path,
        &config.whisper.language,
        config.whisper.threads,
    )
    .unwrap_or_else(|e| {
        log::error!("failed to load whisper model: {e:#}");
        std::process::exit(2);
    });

    let refiner = LlamaCppTextRefiner::load(
        &config.llm.model_path,
        config.llm.system_prompt.clone(),
        config.llm.max_tokens as i32,
        config.llm.temperature,
        config.llm.context_size,
    )
    .unwrap_or_else(|e| {
        log::error!("failed to load llama model: {e:#}");
        std::process::exit(2);
    });

    let paster = MacOsTextPaster::new(config.paste.clipboard_restore_delay_ms);

    let recorder = CpalAudioRecorder::new(
        config
            .audio
            .debug_save_wav
            .then(|| std::path::PathBuf::from("debug.wav")),
    );

    let (tx, rx) = std::sync::mpsc::channel::<PttSignal>();

    let processing = Arc::new(AtomicBool::new(false));

    let listener = RdevHotkeyListener::new();
    let _handle = Box::new(listener)
        .spawn(config.hotkey.key, tx, Arc::clone(&processing))
        .unwrap_or_else(|e| {
            log::error!("failed to spawn hotkey listener: {e:#}");
            std::process::exit(2);
        });

    let sound_player = MacosSoundPlayer::new(
        &config.sounds.start_sound,
        &config.sounds.stop_sound,
    );

    let mut pipeline = Pipeline::new(
        Box::new(recorder),
        Box::new(transcriber),
        Box::new(refiner),
        Box::new(paster),
        Box::new(sound_player),
        PipelineConfig {
            min_press_ms: config.hotkey.min_press_ms,
            recording_mode: config.hotkey.recording_mode,
        },
        Arc::clone(&processing),
    );

    pipeline.run(rx).unwrap_or_else(|e| {
        log::error!("fatal: {e:#}");
        std::process::exit(1);
    });
}
