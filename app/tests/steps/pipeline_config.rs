//! The pipeline configuration a scenario's application configuration asks for.

use voicepuncher::config::Config;
use voicepuncher::core::application::pipeline::PipelineConfig;

/// The pipeline configuration of `config`: its hotkey settings, dictations transcribed in one go.
pub fn pipeline_config(config: &Config) -> PipelineConfig {
    PipelineConfig {
        min_press_ms: config.hotkey.min_press_ms,
        recording_mode: config.hotkey.recording_mode,
        ..PipelineConfig::default()
    }
}
