//! The live watcher: cuts the recording buffer into chunks as it grows and sends each to
//! the pipeline; the final chunk, after the stop, reaches the end of the recording.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use super::buffer::RecordingBuffer;
use crate::core::application::audio_recorder::LiveChunk;
use crate::core::application::chunking::{next_chunk_start, next_cut, ChunkingCfg};
use crate::core::domain::AudioBuffer16k;

const RATE: u32 = AudioBuffer16k::SAMPLE_RATE_HZ;
const POLL: Duration = Duration::from_millis(20);

/// A running watcher; dropping the sender makes it send the final chunk and end.
pub struct WatcherThread {
    pub handle: JoinHandle<()>,
    pub stop: Sender<()>,
}

struct Watcher {
    buffer: Arc<RecordingBuffer>,
    cfg: ChunkingCfg,
    chunks: Sender<LiveChunk>,
    stop: Receiver<()>,
    next_start: usize,
    stopping: bool,
}

/// Watches `buffer`, sending every chunk `cfg` cuts to `chunks`.
pub fn spawn(
    buffer: Arc<RecordingBuffer>,
    cfg: ChunkingCfg,
    chunks: Sender<LiveChunk>,
) -> Result<WatcherThread> {
    let (stop, stop_rx) = mpsc::channel();
    let watcher = Watcher { buffer, cfg, chunks, stop: stop_rx, next_start: 0, stopping: false };
    let handle = std::thread::Builder::new()
        .name("live-watcher".into())
        .spawn(move || watcher.run())
        .map_err(|e| anyhow!("failed to spawn live-watcher thread: {e}"))?;
    Ok(WatcherThread { handle, stop })
}

impl Watcher {
    fn run(mut self) {
        let chunk_samples = self.cfg.chunk_threshold_secs as usize * RATE as usize;
        let overlap = self.cfg.chunk_overlap_secs as usize * RATE as usize;
        loop {
            self.stopping |= !matches!(self.stop.try_recv(), Err(TryRecvError::Empty));
            let len = self.buffer.len();
            let available = len.saturating_sub(self.next_start);
            if !self.stopping && available <= chunk_samples {
                std::thread::sleep(POLL);
                continue;
            }
            if self.stopping && self.next_start >= len {
                return;
            }
            // After the stop, audio longer than a chunk threshold is still cut first, so the
            // final chunk fits the threshold too.
            let last = self.stopping && available <= chunk_samples;
            let Some(cut) = self.cut(len, last) else {
                std::thread::sleep(POLL);
                continue;
            };
            if !self.send(cut, last) || last {
                return;
            }
            self.next_start = next_chunk_start(self.next_start, cut, overlap);
        }
    }

    /// Where the next chunk ends: the end of the recording for the last one; none when the
    /// recording was cut before this chunk.
    fn cut(&self, len: usize, last: bool) -> Option<usize> {
        if last {
            return Some(len);
        }
        self.buffer.read(|samples| {
            (self.next_start < samples.len())
                .then(|| next_cut(samples, RATE, self.next_start, self.cfg))
        })
    }

    /// Sends the chunk up to `cut`; whether the pipeline still listens.
    fn send(&self, cut: usize, last: bool) -> bool {
        let started = Instant::now();
        let chunk = self.buffer.between(self.next_start, cut);
        let chunk_secs = chunk.len() as f32 / RATE as f32;
        let audio = AudioBuffer16k::new(chunk);
        let sent = self
            .chunks
            .send(LiveChunk { audio, enqueued_at: Instant::now() })
            .is_ok();
        if last {
            log::info!(
                "live-watcher final-chunk: chunk_secs={chunk_secs:.2} total={}ms",
                started.elapsed().as_millis()
            );
        }
        sent
    }
}
