//! What a pipeline is built from: its six ports and its configuration.

use std::time::Duration;

use crate::core::application::chunking::ChunkingCfg;
use crate::core::application::{
    AudioRecorder, SoundPlayer, TextPaster, TextRefiner, Transcriber, VisualIndicator,
};
use crate::core::domain::RecordingMode;

/// The six ports a pipeline drives.
pub struct PipelineParts {
    pub recorder: Box<dyn AudioRecorder>,
    pub transcriber: Box<dyn Transcriber>,
    pub refiner: Box<dyn TextRefiner>,
    pub paster: Box<dyn TextPaster>,
    pub sound_player: Box<dyn SoundPlayer>,
    pub visual_indicator: Box<dyn VisualIndicator>,
}

/// The wait for a stopped dictation's transcript without a configured one: three attempts
/// at the default 60 s request timeout, their pauses and the drain.
const DEFAULT_STOP_WAIT: Duration = Duration::from_secs(184);

#[derive(Debug, Clone, Copy)]
pub struct PipelineConfig {
    pub min_press_ms: u64,
    pub recording_mode: RecordingMode,
    pub chunking: ChunkingCfg,
    /// How long a stopped dictation waits for its live worker's result.
    pub stop_wait: Duration,
    /// Whether the overlay shows the text refinement stage: off when refinement is off.
    pub shows_refinement: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            min_press_ms: 0,
            recording_mode: RecordingMode::default(),
            chunking: ChunkingCfg::default(),
            stop_wait: DEFAULT_STOP_WAIT,
            shows_refinement: true,
        }
    }
}
