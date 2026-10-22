//! The dictation recorder over cpal.
//!
//! The microphone's audio goes through a wait-free ring (`capture`) into a drain thread
//! (`drain`) that resamples it into one 16 kHz recording buffer (`buffer`); the live
//! watcher (`watcher`) cuts chunks out of that buffer.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Result};
use rtrb::Consumer;

use crate::core::application::audio_level_sink::AudioLevelSink;
use crate::core::application::audio_recorder::{AudioRecorder, LiveChunk};
use crate::core::application::chunking::ChunkingCfg;
use crate::core::application::PipelineInput;
use crate::core::domain::audio_level::AudioLevel;
use crate::core::domain::AudioBuffer16k;

pub mod buffer;
pub mod capture;
pub mod devices;
pub mod drain;
pub mod resampler;
pub mod watchdog;
pub mod watcher;

use buffer::{samples_before, RecordingBuffer};
use capture::{CaptureHealth, CapturedAudio, MicInput};

/// What a recording captures from: the microphone, or a test's stand-in stream.
pub struct Source {
    pub audio: Box<dyn CapturedAudio>,
    /// The rate of `audio`'s samples.
    pub rate: u32,
    pub health: Arc<CaptureHealth>,
    pub started_at: Instant,
    /// The open device, kept until the recording stops.
    pub input: Option<MicInput>,
}

impl Source {
    pub fn microphone(input: MicInput, audio: Consumer<f32>) -> Self {
        Self {
            audio: Box::new(audio),
            rate: input.rate,
            health: Arc::clone(input.health()),
            started_at: input.started_at,
            input: Some(input),
        }
    }
}

/// Opens the source for the configured input device, with the level register.
type OpenSource = Box<dyn FnMut(&str, Arc<AtomicU32>) -> Result<Source> + Send + Sync>;

pub struct CpalAudioRecorder {
    max_recording_seconds: u64,
    active: Option<ActiveRecording>,
    level_register: Arc<AtomicU32>,
    last_recording: Option<Vec<i16>>,
    /// The configuration's `input_device`: a device name or `default`.
    input_device: String,
    open: OpenSource,
    /// Where a failed microphone is reported, when the pipeline listens.
    failures: Option<Sender<PipelineInput>>,
}

struct ActiveRecording {
    input: Option<MicInput>,
    health: Arc<CaptureHealth>,
    rate: u32,
    buffer: Arc<RecordingBuffer>,
    drain: Option<drain::DrainThread>,
    watcher: Option<watcher::WatcherThread>,
    used_live: bool,
    started_at: Instant,
}

impl ActiveRecording {
    /// Closes the device and ends the drain, so the buffer holds the whole recording; the
    /// audio dropped by a full ring is logged once here.
    fn finish_capture(&mut self) {
        drop(self.input.take());
        let Some(drain) = self.drain.take() else {
            return;
        };
        drop(drain.stop);
        if drain.handle.join().is_err() {
            log::warn!("recording drain thread panicked");
        }
        let dropped = self.health.dropped_samples();
        if dropped > 0 {
            log::warn!(
                "recording: {} ms of audio dropped: the recorder fell behind the microphone",
                dropped * 1_000 / self.rate as usize
            );
        }
    }
}

impl CpalAudioRecorder {
    pub fn new(max_recording_seconds: u64, level_register: Arc<AtomicU32>) -> Self {
        Self {
            max_recording_seconds,
            active: None,
            level_register,
            last_recording: None,
            input_device: devices::DEFAULT_DEVICE.to_owned(),
            open: Box::new(|device, level| {
                MicInput::open(device, level).map(|(input, audio)| Source::microphone(input, audio))
            }),
            failures: None,
        }
    }

    /// A microphone that fails or goes silent mid-recording is reported to `failures`.
    pub fn with_failure_input(mut self, failures: Sender<PipelineInput>) -> Self {
        self.failures = Some(failures);
        self
    }

    /// Records from the input device `name` names (see [`devices::open`]).
    pub fn with_input_device(mut self, name: &str) -> Self {
        self.input_device = name.to_owned();
        self
    }

    /// Records from what `open` gives instead of the microphone.
    pub fn with_source(
        mut self,
        open: impl FnMut(&str, Arc<AtomicU32>) -> Result<Source> + Send + Sync + 'static,
    ) -> Self {
        self.open = Box::new(open);
        self
    }

    fn active(&mut self) -> Result<&mut ActiveRecording> {
        self.active
            .as_mut()
            .ok_or_else(|| anyhow!("recorder was not started"))
    }
}

impl AudioRecorder for CpalAudioRecorder {
    fn start(&mut self) -> Result<()> {
        self.last_recording = None;
        let source = (self.open)(&self.input_device, Arc::clone(&self.level_register))?;
        let max_samples =
            self.max_recording_seconds as usize * AudioBuffer16k::SAMPLE_RATE_HZ as usize;
        let buffer = Arc::new(RecordingBuffer::new(max_samples));
        let drain = drain::spawn(
            source.audio,
            source.rate,
            Arc::clone(&buffer),
            Arc::clone(&source.health),
            self.failures.clone(),
        )?;
        self.active = Some(ActiveRecording {
            input: source.input,
            health: source.health,
            rate: source.rate,
            buffer,
            drain: Some(drain),
            watcher: None,
            used_live: false,
            started_at: source.started_at,
        });
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let mut recording = self
            .active
            .take()
            .ok_or_else(|| anyhow!("recorder was not started"))?;
        recording.finish_capture();
        if recording.buffer.is_full() {
            log::warn!(
                "Recording reached the {}-second limit and was cut off at that point. \
                 To allow longer recordings, increase [audio] max_recording_seconds in your config.",
                self.max_recording_seconds
            );
        }
        let samples = recording.buffer.take();
        let delivered = if recording.used_live {
            Vec::new()
        } else {
            samples.clone()
        };
        self.last_recording = Some(samples);
        Ok(AudioBuffer16k::new(delivered))
    }

    fn start_live_chunks(
        &mut self,
        cfg: ChunkingCfg,
    ) -> Result<Option<std::sync::mpsc::Receiver<LiveChunk>>> {
        if cfg.chunk_threshold_secs == 0 {
            return Ok(None);
        }
        let active = self.active()?;
        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<LiveChunk>();
        active.watcher = Some(watcher::spawn(Arc::clone(&active.buffer), cfg, chunk_tx)?);
        active.used_live = true;
        Ok(Some(chunk_rx))
    }

    fn flush_live_chunks(&mut self) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        active.finish_capture();
        let Some(watcher) = active.watcher.take() else {
            return;
        };
        drop(watcher.stop);
        if watcher.handle.join().is_err() {
            log::warn!("live-watcher thread panicked");
        }
    }

    fn begin_with(&mut self, lead_in: AudioBuffer16k) {
        if let Some(active) = self.active.as_mut() {
            // The recording now begins earlier: cuts count from there.
            active.started_at -= active.buffer.put_first(lead_in.samples());
        }
    }

    fn recording(&mut self) -> Option<AudioBuffer16k> {
        self.last_recording.clone().map(AudioBuffer16k::new)
    }

    fn truncate_at(&mut self, cut: Instant) {
        if let Some(active) = self.active.as_ref() {
            active.buffer.cut_at(samples_before(active.started_at, cut));
        }
    }
}

impl AudioLevelSink for CpalAudioRecorder {
    fn write_level(&self, level: AudioLevel) {
        self.level_register
            .store(level.to_bits(), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_err_when_stop_called_without_start() {
        let mut recorder = CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0)));
        let result = recorder.stop();
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("recorder was not started"));
    }

    #[test]
    fn should_store_level_bits_via_audio_level_sink() {
        let reg = Arc::new(AtomicU32::new(0));
        let recorder = CpalAudioRecorder::new(600, Arc::clone(&reg));
        recorder.write_level(AudioLevel::from_rms(0.5));
        assert_eq!(reg.load(Ordering::Relaxed), 0.5f32.to_bits());
    }
}
