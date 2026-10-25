use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use anyhow::{anyhow, Result};

use crate::core::application::audio_recorder::{AudioRecorder, LiveChunk};
use crate::core::application::chunking::ChunkingCfg;
use crate::core::domain::AudioBuffer16k;

pub struct FileSystemAudioRecorder {
    path: PathBuf,
}

impl FileSystemAudioRecorder {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(path: &Path) -> Result<AudioBuffer16k> {
        let mut reader = hound::WavReader::open(path)
            .map_err(|e| anyhow!("failed to open WAV file {}: {e}", path.display()))?;
        let spec = reader.spec();
        if spec.sample_rate != AudioBuffer16k::SAMPLE_RATE_HZ {
            anyhow::bail!(
                "WAV sample rate is {} Hz; expected {} Hz",
                spec.sample_rate,
                AudioBuffer16k::SAMPLE_RATE_HZ
            );
        }
        if spec.channels != 1 {
            anyhow::bail!(
                "WAV must be mono (1 channel); got {} channels",
                spec.channels
            );
        }
        if spec.bits_per_sample != 16 {
            anyhow::bail!("WAV must be 16-bit PCM; got {}-bit", spec.bits_per_sample);
        }
        let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
        let samples = samples.map_err(|e| anyhow!("failed to read WAV samples: {e}"))?;
        Ok(AudioBuffer16k::new(samples))
    }
}

impl AudioRecorder for FileSystemAudioRecorder {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Self::load(&self.path)
    }

    fn start_live_chunks(
        &mut self,
        _cfg: ChunkingCfg,
    ) -> Result<Option<mpsc::Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}

    fn recording(&mut self) -> Option<AudioBuffer16k> {
        Self::load(&self.path).ok()
    }
    fn truncate_at(&mut self, _cut: Instant) {}
    fn begin_with(&mut self, _lead_in: AudioBuffer16k) {}
}
