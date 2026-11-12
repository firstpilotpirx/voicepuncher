//! Voice control stand-ins of the scenarios that drive the pipeline by its input channel:
//! a spotter that hears nothing and a key presser that presses nothing.

use std::sync::mpsc::Sender;

use voicepuncher::test_doubles::prelude::*;

/// The command spotting service as the pipeline sees it; the scenario feeds the phrases it
/// reports straight into the pipeline's input channel.
pub struct QuietSpotter;

impl CommandSpotter for QuietSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {}
    voicepuncher::holds_no_heard_sound!();
}

/// Key presses the scenario never observes.
pub struct NoKeys;

impl KeyPresser for NoKeys {
    fn press_enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        Ok(())
    }

    fn paste(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Runs the pipeline on a thread of its own, with the quiet voice control stand-ins when the
/// scenario turns voice control on; the sender of the pipeline's input.
pub fn spawn_pipeline(mut pipeline: Pipeline, voice_control: bool) -> Sender<PipelineInput> {
    let (tx, rx) = std::sync::mpsc::channel();
    if voice_control {
        pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
    }
    std::thread::spawn(move || pipeline.run(rx));
    tx
}
