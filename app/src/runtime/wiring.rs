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
use crate::adapters::secondary::recording::cpal::CpalAudioRecorder;
use crate::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use crate::adapters::secondary::textpasting::macos::MacOsTextPaster;
use crate::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use crate::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use crate::adapters::secondary::transcription::openai::OpenAiTranscriber;
use crate::config::{Config, TranscriptionCfg};
use crate::core::application::pipeline::{Pipeline, PipelineConfig};
use crate::core::application::{
    ChunkingCfg, CommandSpotter, FocusWaitCfg, PipelineInput, TextRefiner,
};

use super::{PipelineAssembler, SharedVisualIndicator};

/// How often the pipeline asks which element has focus while it waits for a text input.
const FOCUS_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Builds every pipeline over the process's one overlay and one level register.
pub struct WiringAssembler {
    overlay: SharedVisualIndicator,
    level_register: Arc<AtomicU32>,
}

impl WiringAssembler {
    pub fn new(overlay: SharedVisualIndicator, level_register: Arc<AtomicU32>) -> Self {
        Self {
            overlay,
            level_register,
        }
    }
}

impl PipelineAssembler for WiringAssembler {
    fn assemble(&mut self, config: &Config, inputs: Sender<PipelineInput>) -> Result<Pipeline> {
        let spotter = config
            .voice_control
            .enabled
            .then(|| connect_command_spotter(config, inputs))
            .transpose()?;
        let transcriber = connect_transcriber(config)?;
        let refiner = connect_refiner(config)?;
        let mut pipeline = Pipeline::new(
            Box::new(
                CpalAudioRecorder::new(
                    config.audio.max_recording_seconds,
                    Arc::clone(&self.level_register),
                )
                .with_input_device(&config.audio.input_device),
            ),
            Box::new(transcriber),
            refiner,
            Box::new(MacOsTextPaster::new(
                config.paste.clipboard_restore_delay_ms,
            )),
            Box::new(sound_player(config)),
            Box::new(self.overlay.clone()),
            PipelineConfig {
                min_press_ms: config.hotkey.min_press_ms,
                recording_mode: config.hotkey.recording_mode,
                chunking: chunking_cfg(&config.transcription),
            },
        )
        .with_focus_wait(
            Box::new(MacOsFocusProbe::new()),
            FocusWaitCfg {
                timeout: Duration::from_secs(config.paste.focus_wait_seconds),
                poll_interval: FOCUS_POLL_INTERVAL,
            },
        );
        if !config.refinement.enabled {
            pipeline = pipeline.without_text_refinement_stage();
        }
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

/// The dictation archive in the configured folder, with the models every entry records.
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
        refinement_model: config
            .refinement
            .enabled
            .then(|| config.refinement.model.clone()),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    FileSystemDictationArchive::new(dir, config.archive.max_size_bytes(), setup)
}

/// The command spotting adapter, once its service answers.
fn connect_command_spotter(
    config: &Config,
    inputs: Sender<PipelineInput>,
) -> Result<WebSocketCommandSpotter> {
    let spotter = WebSocketCommandSpotter::new(config.voice_control.url.clone(), inputs)
        .with_input_device(&config.audio.input_device);
    spotter.probe()?;
    Ok(spotter)
}

pub fn connect_transcriber(config: &Config) -> Result<OpenAiTranscriber> {
    OpenAiTranscriber::connect(config.transcription.clone())
}

/// The text refiner `[refinement] enabled` asks for, read anew at every assembly so a
/// reload switches refinement on or off.
pub fn connect_refiner(config: &Config) -> Result<Box<dyn TextRefiner>> {
    if !config.refinement.enabled {
        log::info!("text refinement off (turn it on with --llm), pasting raw transcripts");
        return Ok(Box::new(PassthroughTextRefiner));
    }
    Ok(Box::new(OpenAiTextRefiner::connect(
        config.refinement.clone(),
    )?))
}

pub fn chunking_cfg(cfg: &TranscriptionCfg) -> ChunkingCfg {
    ChunkingCfg {
        chunk_threshold_secs: cfg.chunk_threshold_secs,
        chunk_overlap_secs: cfg.chunk_overlap_secs,
        vad_lookback_secs: cfg.vad_lookback_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_paste_raw_transcripts_without_a_refinement_service_when_refinement_is_off() {
        // Given
        let mut config = Config::default();
        config.refinement.enabled = false;
        config.refinement.base_url = "http://127.0.0.1:1/v1".into();
        // When
        let refiner = connect_refiner(&config);
        // Then
        assert!(refiner.is_ok());
    }

    #[test]
    fn should_need_the_refinement_service_when_refinement_is_on() {
        // Given
        let mut config = Config::default();
        config.refinement.enabled = true;
        config.refinement.base_url = "http://127.0.0.1:1/v1".into();
        // When
        let refiner = connect_refiner(&config);
        // Then
        assert!(refiner.is_err());
    }
}
