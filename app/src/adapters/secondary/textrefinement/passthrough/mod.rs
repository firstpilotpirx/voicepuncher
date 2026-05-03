use anyhow::Result;

use crate::core::application::text_refiner::TextRefiner;
use crate::core::domain::{CleanedText, RawTranscript};

pub struct PassthroughTextRefiner;

impl TextRefiner for PassthroughTextRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        CleanedText::new(raw.as_str().to_owned()).map_err(|e| anyhow::anyhow!(e))
    }
}
