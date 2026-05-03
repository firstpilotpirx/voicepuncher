use anyhow::Result;

use crate::core::domain::{CleanedText, RawTranscript};

#[cfg_attr(test, mockall::automock)]
pub trait TextRefiner: Send {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText>;
}
