//! Long recordings transcribed in chunks: where to cut them and how to merge the chunk transcripts.

use crate::core::domain::AudioBuffer16k;

mod seam;
pub use seam::*;
mod cut;
pub use cut::*;

#[derive(Debug, Clone, Copy)]
pub struct ChunkingCfg {
    pub chunk_threshold_secs: u32,
    pub chunk_overlap_secs: u32,
    pub vad_lookback_secs: u32,
}

impl Default for ChunkingCfg {
    fn default() -> Self {
        Self { chunk_threshold_secs: 0, chunk_overlap_secs: 2, vad_lookback_secs: 5 }
    }
}

#[cfg(test)]
mod cut_tests;
#[cfg(test)]
mod seam_cases;
#[cfg(test)]
mod seam_tests;
