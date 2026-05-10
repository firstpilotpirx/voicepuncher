use anyhow::Result;

use crate::core::domain::{AudioBuffer16k, RawTranscript};

#[cfg_attr(test, mockall::automock)]
pub trait Transcriber: Send {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>>;
}
