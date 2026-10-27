//! The real assembler: the macOS adapters and the local services of the configuration.

use std::sync::atomic::AtomicU32;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use crate::adapters::secondary::archiving::filesystem::{ArchiveSetup, FileSystemDictationArchive};
use crate::adapters::secondary::commandspotting::websocket::WebSocketCommandSpotter;
use crate::adapters::secondary::focus::macos::MacOsFocusProbe;
use crate::adapters::secondary::keypressing::macos::MacOsKeyPresser;
use crate::adapters::secondary::recording::cpal::tap::MicTap;
use crate::adapters::secondary::recording::cpal::CpalAudioRecorder;
use crate::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use crate::adapters::secondary::textpasting::macos::MacOsTextPaster;
use crate::adapters::secondary::transcription::openai::{longest_wait, OpenAiTranscriber};
use crate::config::{Config, TranscriptionCfg};
use crate::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use crate::core::application::{ChunkingCfg, CommandSpotter, FocusWaitCfg, PipelineInput};

use super::{PipelineAssembler, SharedVisualIndicator};

/// How often the pipeline asks which element has focus while it waits for a text input.
const FOCUS_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// What a stopped dictation may wait beyond its last chunk, for the chunks before it.
const STOP_DRAIN: Duration = Duration::from_secs(2);

/// Builds every pipeline over the process's one overlay and one level register.
pub struct WiringAssembler {
    overlay: SharedVisualIndicator,
    level_register: Arc<AtomicU32>,
}

impl WiringAssembler {
    /// The microphone recorder of the configuration, reporting a failed device to `inputs`.
    fn recorder(&self, config: &Config, inputs: Sender<PipelineInput>) -> CpalAudioRecorder {
        let level = Arc::clone(&self.level_register);
        CpalAudioRecorder::new(config.audio.max_recording_seconds, level)
            .with_input_device(&config.audio.input_device)
            .with_failure_input(inputs)
    }

    pub fn new(overlay: SharedVisualIndicator, level_register: Arc<AtomicU32>) -> Self {
        Self { overlay, level_register }
    }
}

impl PipelineAssembler for WiringAssembler {
    fn assemble(&mut self, config: &Config, inputs: Sender<PipelineInput>) -> Result<Pipeline> {
        // WHY: one microphone: a dictation records what voice control already captures.
        let tap = MicTap::default();
        let recorder = self.recorder(config, inputs.clone()).with_tap(tap.clone());
        let spotter = config
            .voice_control
            .enabled
            .then(|| connect_command_spotter(config, inputs, tap))
            .transpose()?;
        let transcriber = connect_transcriber(config)?;
        let paster = MacOsTextPaster::new(config.paste.clipboard_restore_delay_ms);
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(recorder),
                transcriber: Box::new(transcriber),
                paster: Box::new(paster),
                sound_player: Box::new(sound_player(config)),
                visual_indicator: Box::new(self.overlay.clone()),
            },
            pipeline_config(config),
        )
        .with_stop_wait(stop_wait(&config.transcription))
        .with_focus_wait(
            Box::new(MacOsFocusProbe::new()),
            FocusWaitCfg {
                timeout: Duration::from_secs(config.paste.focus_wait_seconds),
                poll_interval: FOCUS_POLL_INTERVAL,
            },
        );
        if config.archive.enabled {
            pipeline = pipeline.with_archive(Box::new(connect_archive(config)));
        } else {
            log::info!("dictation archive: off");
        }
        if let Some(spotter) = spotter {
            log::info!(
                "voice control on: listening starts now; the hotkey records as push-to-talk"
            );
            pipeline =
                pipeline.with_voice_control(Box::new(spotter), Box::new(MacOsKeyPresser::new()));
        }
        Ok(pipeline)
    }
}

/// The pipeline's settings from the configuration: the hotkey's and the chunking.
fn pipeline_config(config: &Config) -> PipelineConfig {
    PipelineConfig {
        min_press_ms: config.hotkey.min_press_ms,
        recording_mode: config.hotkey.recording_mode,
        chunking: chunking_cfg(&config.transcription),
        ..PipelineConfig::default()
    }
}

fn sound_player(config: &Config) -> MacosSoundPlayer {
    MacosSoundPlayer::new(
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
    )
}

/// The dictation archive in the configured folder, with the model every entry records.
pub fn connect_archive(config: &Config) -> FileSystemDictationArchive {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dir = config.archive.resolved_dir(&home);
    log::info!(
        "dictation archive: {} (limit {} MB)",
        dir.display(),
        config.archive.max_size_mb
    );
    let setup = ArchiveSetup {
        dictation_model: config.transcription.model.clone(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    FileSystemDictationArchive::new(dir, config.archive.max_size_bytes(), setup)
}

/// The command spotting adapter, once its service answers.
fn connect_command_spotter(
    config: &Config,
    inputs: Sender<PipelineInput>,
    tap: MicTap,
) -> Result<WebSocketCommandSpotter> {
    let spotter = WebSocketCommandSpotter::new(config.voice_control.url.clone(), inputs)
        .with_input_device(&config.audio.input_device)
        .with_tap(tap)
        .confirming(config.voice_control.confirm_commands);
    spotter.probe()?;
    Ok(spotter)
}

/// How long a stopped dictation waits for its transcript: its last chunk with every retry,
/// plus the drain of the chunks before it.
fn stop_wait(cfg: &TranscriptionCfg) -> Duration {
    longest_wait(Duration::from_secs(cfg.timeout_secs)) + STOP_DRAIN
}

pub fn connect_transcriber(config: &Config) -> Result<OpenAiTranscriber> {
    OpenAiTranscriber::connect(config.transcription.clone())
}

pub fn chunking_cfg(cfg: &TranscriptionCfg) -> ChunkingCfg {
    ChunkingCfg {
        chunk_threshold_secs: cfg.chunk_threshold_secs,
        chunk_overlap_secs: cfg.chunk_overlap_secs,
        vad_lookback_secs: cfg.vad_lookback_secs,
    }
}
