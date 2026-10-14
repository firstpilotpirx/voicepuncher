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
    fn truncate_at(&mut self, _cut: Instant) {}
    /// Starts recording with `lead_in`, audio heard just before, as its beginning.
    fn start_with_lead_in(&mut self, _lead_in: AudioBuffer16k) -> Result<()> {
        self.start()
    }
}
