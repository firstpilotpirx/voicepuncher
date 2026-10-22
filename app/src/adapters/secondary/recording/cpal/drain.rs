//! The drain thread: the captured audio from the ring into the 16 kHz recording buffer.
//!
//! It pops the ring every few milliseconds, resamples and appends; at the stop it flushes
//! the resampler's tail, so the recording ends where the microphone's audio ends.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Result};

use super::buffer::RecordingBuffer;
use super::capture::{CaptureHealth, CapturedAudio};
use super::resampler::StreamResampler;
use super::watchdog::Watchdog;
use crate::core::application::PipelineInput;

/// How often the ring is popped.
pub const DRAIN_PERIOD: Duration = Duration::from_millis(5);

const I16_MAX_F32: f32 = i16::MAX as f32;

/// A running drain thread; dropping the sender stops it after its final flush.
pub struct DrainThread {
    pub handle: JoinHandle<()>,
    pub stop: Sender<()>,
}

struct Drain {
    audio: Box<dyn CapturedAudio>,
    resampler: StreamResampler,
    buffer: Arc<RecordingBuffer>,
    stop: Receiver<()>,
    watchdog: Watchdog,
}

/// Drains `audio`, mono at `rate`, into `buffer` until stopped; a device `health` shows
/// failed or silent is reported to `failures` once.
pub fn spawn(
    audio: Box<dyn CapturedAudio>,
    rate: u32,
    buffer: Arc<RecordingBuffer>,
    health: Arc<CaptureHealth>,
    failures: Option<Sender<PipelineInput>>,
) -> Result<DrainThread> {
    let (stop, stop_rx) = mpsc::channel();
    let drain = Drain {
        audio,
        resampler: StreamResampler::new(rate)?,
        buffer,
        stop: stop_rx,
        watchdog: Watchdog::new(health, failures),
    };
    let handle = std::thread::Builder::new()
        .name("recording-drain".into())
        .spawn(move || drain.run())
        .map_err(|e| anyhow!("failed to spawn the recording drain thread: {e}"))?;
    Ok(DrainThread { handle, stop })
}

impl Drain {
    fn run(mut self) {
        loop {
            let stopping = !matches!(self.stop.try_recv(), Err(TryRecvError::Empty));
            let captured = self.audio.take();
            let resampled = self.resampler.push(&captured);
            self.append(&resampled);
            self.watchdog.check();
            if stopping {
                let tail = self.resampler.finish();
                self.append(&tail);
                return;
            }
            std::thread::sleep(DRAIN_PERIOD);
        }
    }

    fn append(&self, samples: &[f32]) {
        let samples: Vec<i16> = samples.iter().copied().map(float_to_i16).collect();
        self.buffer.append(&samples);
    }
}

pub fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * I16_MAX_F32) as i16
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;

    use super::*;

    #[test]
    fn should_convert_zero_float_to_zero_i16() {
        assert_eq!(float_to_i16(0.0), 0);
    }

    #[test]
    fn should_convert_positive_one_to_max_i16() {
        assert_eq!(float_to_i16(1.0), 32_767);
    }

    #[test]
    fn should_convert_negative_one_to_min_i16() {
        assert_eq!(float_to_i16(-1.0), -32_767);
    }

    #[test]
    fn should_clamp_above_one_to_max_i16() {
        assert_eq!(float_to_i16(2.0), 32_767);
    }

    #[test]
    fn should_clamp_below_minus_one_to_min_i16() {
        assert_eq!(float_to_i16(-2.0), -32_767);
    }

    #[test]
    fn should_drain_16_khz_audio_as_it_is_and_end_where_it_ends() {
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>();
        let buffer = Arc::new(RecordingBuffer::new(100_000));
        let health = Arc::new(CaptureHealth::new(Arc::new(AtomicU32::new(0))));
        let drain = spawn(
            Box::new(audio_rx),
            16_000,
            Arc::clone(&buffer),
            health,
            None,
        )
        .unwrap();
        audio_tx.send(vec![0.5; 100]).unwrap();
        drop(drain.stop);
        drain.handle.join().unwrap();
        assert_eq!(buffer.take(), [float_to_i16(0.5); 100]);
    }
}
