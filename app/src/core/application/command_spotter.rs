use std::time::Instant;

use anyhow::Result;

use crate::core::domain::{AudioBuffer16k, VoiceCommand};

/// Streams the microphone to the command spotting service while listening. Spotting
/// events reach the pipeline through the input channel the adapter was built with.
#[cfg_attr(test, mockall::automock)]
pub trait CommandSpotter: Send {
    /// Fails when the command spotting service does not answer.
    fn probe(&self) -> Result<()>;
    /// Opens the microphone and listens for the phrases of `commands`.
    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()>;
    /// Listens for the phrases of `commands` from now on.
    fn listen_for(&mut self, commands: &[VoiceCommand]);
    /// Closes the microphone.
    fn stop_listening(&mut self);
    /// The sound heard from `since` until now, as far as it is still held: the speech
    /// that followed a voice command starting a dictation before the recording began.
    fn heard_since(&self, since: Instant) -> AudioBuffer16k;
}
