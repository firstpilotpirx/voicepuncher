use std::time::Instant;

use anyhow::Result;

use crate::core::application::chunking::ChunkingCfg;
use crate::core::domain::AudioBuffer16k;

pub struct LiveChunk {
    pub audio: AudioBuffer16k,
    pub enqueued_at: Instant,
}

#[cfg_attr(test, mockall::automock)]
pub trait AudioRecorder: Send {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<AudioBuffer16k>;
    fn start_live_chunks(
        &mut self,
        cfg: ChunkingCfg,
    ) -> Result<Option<std::sync::mpsc::Receiver<LiveChunk>>>;
    fn flush_live_chunks(&mut self);
    /// Drops the audio captured from `cut` on and records nothing more, so a later
    /// `flush_live_chunks` and `stop` deliver only the audio before `cut`.
    fn truncate_at(&mut self, cut: Instant);
    /// Puts `lead_in`, sound heard right up to the moment the recording started, at its
    /// beginning: the recording then begins where the lead-in does.
    fn begin_with(&mut self, lead_in: AudioBuffer16k);
    /// The whole last recording at 16 kHz, lead-in included and nothing after a cut; none
    /// when the recorder keeps none.
    fn recording(&mut self) -> Option<AudioBuffer16k>;
}
