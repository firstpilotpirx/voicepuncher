//! Microphone capture shared by the dictation recorder and the command spotting stream.
//!
//! The input callback averages frames to mono and pushes them into a wait-free ring,
//! allocating and locking nothing, so audio is never dropped for a held-up reader.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};

use super::devices;

/// How much audio the ring holds before its reader must have taken it.
pub const RING_SECONDS: usize = 2;

/// What the input callback reports, read from any thread.
pub struct CaptureHealth {
    /// The latest RMS level, as `f32` bits (the overlay's register).
    level: Arc<AtomicU32>,
    dropped: AtomicUsize,
    last_callback_ms: AtomicU64,
    had_callback: AtomicBool,
    failed: AtomicBool,
    epoch: Instant,
}

impl CaptureHealth {
    pub fn new(level: Arc<AtomicU32>) -> Self {
        Self {
            level,
            dropped: AtomicUsize::new(0),
            last_callback_ms: AtomicU64::new(0),
            had_callback: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            epoch: Instant::now(),
        }
    }

    /// Samples the callback dropped because the ring was full.
    pub fn dropped_samples(&self) -> usize {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Whether the device reported an error.
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    /// Whether the device delivered any audio yet.
    pub fn had_callback(&self) -> bool {
        self.had_callback.load(Ordering::Relaxed)
    }

    /// The time since the last input callback (since the open, before the first).
    pub fn since_last_callback(&self) -> Duration {
        let last = Duration::from_millis(self.last_callback_ms.load(Ordering::Relaxed));
        self.epoch.elapsed().saturating_sub(last)
    }

    /// The error callback's report.
    pub fn note_error(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }

    fn note_frames(&self, sum_sq: f32, kept: usize, dropped: usize) {
        let rms = (sum_sq / kept.max(1) as f32).sqrt();
        self.level
            .store(f32::to_bits(rms.clamp(0.0, 1.0)), Ordering::Relaxed);
        self.dropped.fetch_add(dropped, Ordering::Relaxed);
        let now = u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.last_callback_ms.store(now, Ordering::Relaxed);
        self.had_callback.store(true, Ordering::Relaxed);
    }
}

/// An open input device, capturing until dropped.
pub struct MicInput {
    _stream: Stream,
    /// The device's own sample rate: the rate of the ring's samples.
    pub rate: u32,
    pub started_at: Instant,
    health: Arc<CaptureHealth>,
}

impl MicInput {
    /// Opens the input device `input_device` names (see [`devices::open`]).
    ///
    /// Its audio, mono at its own rate, comes out of the returned ring consumer; its level
    /// goes to `level` as `f32` bits.
    pub fn open(input_device: &str, level: Arc<AtomicU32>) -> Result<(Self, Consumer<f32>)> {
        let device = devices::open(input_device)?;
        let supported = device.default_input_config()?;
        let rate = supported.sample_rate();
        let config = StreamConfig {
            channels: supported.channels(),
            sample_rate: rate,
            buffer_size: cpal::BufferSize::Default,
        };
        let (mut ring, consumer) = RingBuffer::new(RING_SECONDS * rate as usize);
        let health = Arc::new(CaptureHealth::new(level));
        let (frames_health, error_health) = (Arc::clone(&health), Arc::clone(&health));
        let channels = usize::from(supported.channels());
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                on_frames(data, channels, &mut ring, &frames_health);
            },
            move |err| {
                log::error!("microphone input error: {err}");
                error_health.note_error();
            },
            None,
        )?;
        stream.play()?;
        let input = Self { _stream: stream, rate, started_at: Instant::now(), health };
        Ok((input, consumer))
    }

    pub fn health(&self) -> &Arc<CaptureHealth> {
        &self.health
    }
}

/// The input callback: interleaved frames of `channels` samples averaged to mono and
/// pushed into `ring`, as many as fit; the rest are counted as dropped.
///
/// Runs on the audio thread: it allocates nothing and locks nothing.
pub fn on_frames(data: &[f32], channels: usize, ring: &mut Producer<f32>, health: &CaptureHealth) {
    let channels = channels.max(1);
    let frames = data.len() / channels;
    let kept = frames.min(ring.slots());
    let mut sum_sq = 0.0_f32;
    if let Ok(chunk) = ring.write_chunk_uninit(kept) {
        chunk.fill_from_iter(data.chunks_exact(channels).map(|frame| {
            let mono = frame.iter().sum::<f32>() / channels as f32;
            sum_sq += mono * mono;
            mono
        }));
    }
    health.note_frames(sum_sq, kept, frames - kept);
}

/// What hands the captured audio to the thread that uses it.
pub trait CapturedAudio: Send + 'static {
    /// Every sample delivered since the last take, in order.
    fn take(&mut self) -> Vec<f32>;
}

impl CapturedAudio for Consumer<f32> {
    fn take(&mut self) -> Vec<f32> {
        let Ok(chunk) = self.read_chunk(self.slots()) else {
            return Vec::new();
        };
        let (first, second) = chunk.as_slices();
        let taken: Vec<f32> = first.iter().chain(second).copied().collect();
        chunk.commit_all();
        taken
    }
}

/// A test double's audio, sent chunk by chunk through a channel.
impl CapturedAudio for Receiver<Vec<f32>> {
    fn take(&mut self) -> Vec<f32> {
        self.try_iter().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health() -> (CaptureHealth, Arc<AtomicU32>) {
        let level = Arc::new(AtomicU32::new(0));
        (CaptureHealth::new(Arc::clone(&level)), level)
    }

    #[test]
    fn should_average_stereo_frames_into_mono() {
        let (mut ring, mut consumer) = RingBuffer::new(4);
        let (health, _) = health();
        on_frames(&[0.2, 0.4, -0.5, 0.5], 2, &mut ring, &health);
        assert_eq!(consumer.take(), [0.3, 0.0]);
    }

    #[test]
    fn should_keep_mono_frames_as_they_are() {
        let (mut ring, mut consumer) = RingBuffer::new(4);
        let (health, _) = health();
        on_frames(&[0.1, 0.2], 1, &mut ring, &health);
        assert_eq!(consumer.take(), [0.1, 0.2]);
    }

    #[test]
    fn should_store_the_level_of_the_frames() {
        let (mut ring, _consumer) = RingBuffer::new(4);
        let (health, level) = health();
        on_frames(&[0.5, -0.5, 0.5, -0.5], 1, &mut ring, &health);
        assert_eq!(f32::from_bits(level.load(Ordering::Relaxed)), 0.5);
    }

    #[test]
    fn should_count_the_samples_dropped_when_the_ring_is_full() {
        let (mut ring, mut consumer) = RingBuffer::new(3);
        let (health, _) = health();
        on_frames(&[1.0, 2.0, 3.0, 4.0, 5.0], 1, &mut ring, &health);
        assert_eq!(consumer.take(), [1.0, 2.0, 3.0]);
        assert_eq!(health.dropped_samples(), 2);
    }

    #[test]
    fn should_note_when_the_callback_last_ran() {
        let (mut ring, _consumer) = RingBuffer::new(4);
        let (health, _) = health();
        on_frames(&[0.0], 1, &mut ring, &health);
        assert!(health.since_last_callback() < Duration::from_millis(100));
    }

    #[test]
    fn should_set_the_failed_flag_on_an_error() {
        let (health, _) = health();
        assert!(!health.failed());
        health.note_error();
        assert!(health.failed());
    }

    #[test]
    fn should_take_the_ring_in_order_across_its_wrap() {
        let (mut ring, mut consumer) = RingBuffer::new(4);
        let (health, _) = health();
        on_frames(&[1.0, 2.0, 3.0], 1, &mut ring, &health);
        consumer.take();
        on_frames(&[4.0, 5.0, 6.0], 1, &mut ring, &health);
        assert_eq!(consumer.take(), [4.0, 5.0, 6.0]);
    }
}
