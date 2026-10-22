//! The pipeline's unit tests: one builder with quiet doubles, assertions on what the user
//! sees (the pasted text, the overlay, the activity) and on the log line the manual run reads.

mod activity;
pub(super) mod doubles;
mod overlay;
mod processing;
mod recording;

use std::sync::{Arc, Mutex};

use crate::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use crate::core::application::pipeline_activity::{PipelineActivity, PipelineActivitySink};
use crate::core::application::PipelineInput;
use crate::core::domain::{AudioBuffer16k, PttSignal, RecordingMode};
use doubles::*;

fn one_second() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![0i16; 16_000])
}

/// Shorter than the shortest press.
fn too_short() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![0i16; 10])
}

/// A pipeline built from doubles: each one not given stays quiet and lets a dictation of
/// "hello" through, so a test names only the doubles it asserts on.
pub(super) struct PipelineUnderTest {
    recorder: Option<MockAudioRecorder>,
    transcriber: Option<MockTranscriber>,
    refiner: Option<MockTextRefiner>,
    paster: Option<MockTextPaster>,
    sounds: Option<MockSoundPlayer>,
    overlay: Option<MockVisualIndicator>,
    cfg: PipelineConfig,
    activity: Option<Box<dyn PipelineActivitySink>>,
    refinement_shown: bool,
}

impl PipelineUnderTest {
    pub(super) fn new() -> Self {
        Self {
            recorder: None,
            transcriber: None,
            refiner: None,
            paster: None,
            sounds: None,
            overlay: None,
            cfg: PipelineConfig { min_press_ms: 200, ..PipelineConfig::default() },
            activity: None,
            refinement_shown: true,
        }
    }

    pub(super) fn recorder(mut self, recorder: MockAudioRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub(super) fn transcriber(mut self, transcriber: MockTranscriber) -> Self {
        self.transcriber = Some(transcriber);
        self
    }

    pub(super) fn refiner(mut self, refiner: MockTextRefiner) -> Self {
        self.refiner = Some(refiner);
        self
    }

    pub(super) fn paster(mut self, paster: MockTextPaster) -> Self {
        self.paster = Some(paster);
        self
    }

    pub(super) fn sounds(mut self, sounds: MockSoundPlayer) -> Self {
        self.sounds = Some(sounds);
        self
    }

    pub(super) fn overlay(mut self, overlay: MockVisualIndicator) -> Self {
        self.overlay = Some(overlay);
        self
    }

    pub(super) fn cfg(mut self, cfg: PipelineConfig) -> Self {
        self.cfg = cfg;
        self
    }

    pub(super) fn mode(mut self, mode: RecordingMode) -> Self {
        self.cfg.recording_mode = mode;
        self
    }

    pub(super) fn activity_sink(mut self, sink: Box<dyn PipelineActivitySink>) -> Self {
        self.activity = Some(sink);
        self
    }

    pub(super) fn without_refinement_stage(mut self) -> Self {
        self.refinement_shown = false;
        self
    }

    pub(super) fn build(self) -> Pipeline {
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(self.recorder.unwrap_or_else(|| recording(one_second()))),
                transcriber: Box::new(self.transcriber.unwrap_or_else(|| transcribing("hello"))),
                refiner: Box::new(self.refiner.unwrap_or_else(echoing_refiner)),
                paster: Box::new(self.paster.unwrap_or_else(pasting)),
                sound_player: Box::new(self.sounds.unwrap_or_else(quiet_sounds)),
                visual_indicator: Box::new(self.overlay.unwrap_or_else(quiet_overlay)),
            },
            self.cfg,
        );
        if let Some(sink) = self.activity {
            pipeline = pipeline.with_activity_sink(sink);
        }
        if !self.refinement_shown {
            pipeline = pipeline.without_text_refinement_stage();
        }
        pipeline
    }
}

/// Presses the hotkey once: a start, or the stop of a Toggle-mode dictation.
fn press(pipeline: &mut Pipeline) {
    pipeline
        .handle(PipelineInput::Hotkey(PttSignal::Pressed))
        .unwrap();
}

fn signal(pipeline: &mut Pipeline, signal: PttSignal) {
    pipeline.handle(PipelineInput::Hotkey(signal)).unwrap();
}
