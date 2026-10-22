//! Voice control stand-ins of the scenarios that drive the pipeline by its input channel:
//! a spotter that hears nothing and a key presser that presses nothing.

use anyhow::Result;
use voicepuncher::core::application::{CommandSpotter, KeyPresser};
use voicepuncher::core::domain::VoiceCommand;

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
