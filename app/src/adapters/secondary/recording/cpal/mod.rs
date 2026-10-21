use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::core::application::audio_level_sink::AudioLevelSink;
use crate::core::application::audio_recorder::{AudioRecorder, LiveChunk};
use crate::core::application::chunking::{next_chunk_start, next_cut, ChunkingCfg};
use crate::core::domain::audio_level::AudioLevel;
use crate::core::domain::AudioBuffer16k;

pub mod devices;

const I16_MAX_F32: f32 = i16::MAX as f32;

const RESAMPLER_CHUNK_SIZE: usize = 1024;
const RESAMPLER_SINC_LEN: usize = 256;
const RESAMPLER_F_CUTOFF: f32 = 0.95;
const RESAMPLER_OVERSAMPLING: usize = 128;

pub struct CpalAudioRecorder {
    max_recording_seconds: u64,
    active: Option<ActiveRecording>,
    shared_buffer: Option<Arc<Mutex<Vec<i16>>>>,
    native_rate_out: Option<Arc<AtomicU32>>,
    level_register: Arc<AtomicU32>,
    last_recording: Option<NativeRecording>,
    /// The configuration's `input_device`: a device name or `default`.
    input_device: String,
}

/// What a stopped recording captured, at the device's own sample rate.
struct NativeRecording {
    samples: Vec<i16>,
    rate: u32,
}

impl NativeRecording {
    /// The recording resampled to 16 kHz, the audio speech recognition hears.
    fn at_16k(&self) -> Result<AudioBuffer16k> {
        if self.rate == AudioBuffer16k::SAMPLE_RATE_HZ {
            return Ok(AudioBuffer16k::new(self.samples.clone()));
        }
        let mut resampler = Resampler16k::new(self.rate)?;
        Ok(AudioBuffer16k::new(resampler.resample(&self.samples)?))
    }
}

struct ActiveRecording {
    stream: Stream,
    native_sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
    buffer: Arc<Mutex<Vec<i16>>>,
    max_samples: usize,
    live_stop_tx: Option<std::sync::mpsc::Sender<()>>,
    live_watcher: Option<std::thread::JoinHandle<()>>,
    used_live: bool,
    started_at: Instant,
    capturing: Arc<AtomicBool>,
}

impl CpalAudioRecorder {
    pub fn new(max_recording_seconds: u64, level_register: Arc<AtomicU32>) -> Self {
        Self {
            max_recording_seconds,
            active: None,
            shared_buffer: None,
            native_rate_out: None,
            level_register,
            last_recording: None,
            input_device: devices::DEFAULT_DEVICE.to_owned(),
        }
    }

    pub fn new_with_live_buffer(
        max_recording_seconds: u64,
        shared_buffer: Arc<Mutex<Vec<i16>>>,
        native_rate_out: Arc<AtomicU32>,
        level_register: Arc<AtomicU32>,
    ) -> Self {
        Self {
            max_recording_seconds,
            active: None,
            shared_buffer: Some(shared_buffer),
            native_rate_out: Some(native_rate_out),
            level_register,
            last_recording: None,
            input_device: devices::DEFAULT_DEVICE.to_owned(),
        }
    }

    /// Records from the input device `name` names (see [`devices::open`]).
    pub fn with_input_device(mut self, name: &str) -> Self {
        self.input_device = name.to_owned();
        self
    }
}

impl AudioRecorder for CpalAudioRecorder {
    fn start(&mut self) -> Result<()> {
        self.last_recording = None;
        let device = devices::open(&self.input_device)?;

        let supported_config = device.default_input_config()?;
        let native_sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let stream_config = StreamConfig {
            channels,
            sample_rate: native_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let max_samples = self.max_recording_seconds as usize * native_sample_rate as usize;

        if let Some(ref rate_arc) = self.native_rate_out {
            rate_arc.store(native_sample_rate, Ordering::Relaxed);
        }

        let buffer: Arc<Mutex<Vec<i16>>> = if let Some(ref shared) = self.shared_buffer {
            shared.lock().unwrap().clear();
            Arc::clone(shared)
        } else {
            Arc::new(Mutex::new(Vec::new()))
        };

        let buffer_write = Arc::clone(&buffer);
        let capturing = Arc::new(AtomicBool::new(true));
        let capturing_write = Arc::clone(&capturing);
        let channels_usize = channels as usize;
        let level_reg = Arc::clone(&self.level_register);
        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono_frames: Vec<i16> = if channels_usize == 1 {
                    data.iter().map(|&s| float_to_i16(s)).collect()
                } else {
                    data.chunks_exact(channels_usize)
                        .map(|frame| {
                            let avg = frame.iter().copied().sum::<f32>() / channels_usize as f32;
                            float_to_i16(avg)
                        })
                        .collect()
                };
                let mean_sq: f32 = mono_frames
                    .iter()
                    .map(|&s| {
                        let f = s as f32 / 32_768.0;
                        f * f
                    })
                    .sum::<f32>()
                    / mono_frames.len().max(1) as f32;
                let rms = mean_sq.sqrt();
                level_reg.store(f32::to_bits(rms.clamp(0.0, 1.0)), Ordering::Relaxed);
                if !capturing_write.load(Ordering::Relaxed) {
                    return;
                }
                if let Ok(mut buf) = buffer_write.lock() {
                    let remaining = max_samples.saturating_sub(buf.len());
                    let to_push = mono_frames.len().min(remaining);
                    buf.extend_from_slice(&mono_frames[..to_push]);
                }
            },
            |err| {
                log::error!("cpal input stream error: {err}");
            },
            None,
        )?;

        stream.play()?;
        let started_at = Instant::now();

        self.active = Some(ActiveRecording {
            stream,
            native_sample_rate,
            channels,
            buffer,
            max_samples,
            live_stop_tx: None,
            live_watcher: None,
            used_live: false,
            started_at,
            capturing,
        });

        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let recording = self
            .active
            .take()
            .ok_or_else(|| anyhow!("recorder was not started"))?;

        drop(recording.stream);

        let native = NativeRecording {
            samples: take_samples(recording.buffer),
            rate: recording.native_sample_rate,
        };
        if recording.used_live {
            self.last_recording = Some(native);
            return Ok(AudioBuffer16k::new(Vec::new()));
        }

        if native.samples.len() >= recording.max_samples {
            log::warn!(
                "Recording reached the {}-second limit and was cut off at that point. \
                 To allow longer recordings, increase [audio] max_recording_seconds in your config.",
                self.max_recording_seconds
            );
        }

        let resampled = native.at_16k()?;
        self.last_recording = Some(native);
        Ok(resampled)
    }

    fn start_live_chunks(
        &mut self,
        cfg: ChunkingCfg,
    ) -> Result<Option<std::sync::mpsc::Receiver<LiveChunk>>> {
        if cfg.chunk_threshold_secs == 0 {
            return Ok(None);
        }
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| anyhow!("recorder was not started"))?;

        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<LiveChunk>();
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();

        let buffer = Arc::clone(&active.buffer);
        let native_sample_rate = active.native_sample_rate;

        let handle = std::thread::Builder::new()
            .name("live-watcher".into())
            .spawn(move || {
                live_watcher_thread(buffer, native_sample_rate, cfg, chunk_tx, stop_rx);
            })
            .map_err(|e| anyhow!("failed to spawn live-watcher thread: {e}"))?;

        active.live_stop_tx = Some(stop_tx);
        active.live_watcher = Some(handle);
        active.used_live = true;

        Ok(Some(chunk_rx))
    }

    fn flush_live_chunks(&mut self) {
        if let Some(active) = self.active.as_mut() {
            drop(active.live_stop_tx.take());
            if let Some(handle) = active.live_watcher.take() {
                if handle.join().is_err() {
                    log::warn!("live-watcher thread panicked");
                }
            }
        }
    }

    fn begin_with(&mut self, lead_in: AudioBuffer16k) {
        if let Some(active) = self.active.as_mut() {
            if let Ok(mut buffer) = active.buffer.lock() {
                let lead_in_time = put_first(&mut buffer, &lead_in, active.native_sample_rate);
                // The recording now begins earlier: cuts count from there.
                active.started_at -= lead_in_time;
            }
        }
    }

    fn recording(&mut self) -> Option<AudioBuffer16k> {
        self.last_recording
            .as_ref()?
            .at_16k()
            .inspect_err(|e| log::warn!("last recording could not be resampled: {e:#}"))
            .ok()
    }

    fn truncate_at(&mut self, cut: Instant) {
        if let Some(active) = self.active.as_ref() {
            active.capturing.store(false, Ordering::Relaxed);
            let kept = samples_before(active.started_at, cut, active.native_sample_rate);
            truncate_buffer(&active.buffer, kept);
        }
    }
}

/// The samples `buffer` holds, without copying them when nothing else shares it.
fn take_samples(buffer: Arc<Mutex<Vec<i16>>>) -> Vec<i16> {
    Arc::try_unwrap(buffer)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or_else(|arc| arc.lock().unwrap().clone())
}

/// Puts `lead_in` before the samples `buffer` holds at `rate`; how long the lead-in lasts.
fn put_first(buffer: &mut Vec<i16>, lead_in: &AudioBuffer16k, rate: u32) -> Duration {
    let samples = upsampled(lead_in.samples(), rate);
    let lead_in_time = Duration::from_secs_f64(samples.len() as f64 / f64::from(rate));
    buffer.splice(0..0, samples);
    lead_in_time
}

/// 16 kHz `samples` at `rate`, linearly interpolated: the lead-in goes back to 16 kHz for
/// the speech recognition service, so the in-between samples need not be exact.
fn upsampled(samples: &[i16], rate: u32) -> Vec<i16> {
    if rate == AudioBuffer16k::SAMPLE_RATE_HZ || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = f64::from(AudioBuffer16k::SAMPLE_RATE_HZ) / f64::from(rate);
    let len = (samples.len() as f64 / ratio) as usize;
    (0..len)
        .map(|i| {
            let at = i as f64 * ratio;
            let below = at.floor() as usize;
            let above = (below + 1).min(samples.len() - 1);
            let weight = at - below as f64;
            let value =
                f64::from(samples[below]) * (1.0 - weight) + f64::from(samples[above]) * weight;
            value.round() as i16
        })
        .collect()
}

/// Number of samples captured at `rate` between `started` and `cut`.
fn samples_before(started: Instant, cut: Instant, rate: u32) -> usize {
    (cut.saturating_duration_since(started).as_secs_f64() * f64::from(rate)) as usize
}

/// The samples from `start` to `end`, bounded by what the buffer holds now: a recording cut
/// by `truncate_at` may be shorter than when its length was read.
fn samples_between(buffer: &[i16], start: usize, end: usize) -> Vec<i16> {
    let end = end.min(buffer.len());
    buffer.get(start.min(end)..end).unwrap_or_default().to_vec()
}

fn truncate_buffer(buffer: &Mutex<Vec<i16>>, len: usize) {
    if let Ok(mut samples) = buffer.lock() {
        samples.truncate(len);
    }
}

impl AudioLevelSink for CpalAudioRecorder {
    fn write_level(&self, level: AudioLevel) {
        self.level_register
            .store(level.to_bits(), Ordering::Relaxed);
    }
}

struct Resampler16k {
    inner: SincFixedIn<f32>,
    ratio: f64,
}

impl Resampler16k {
    fn new(native_rate: u32) -> Result<Self> {
        let ratio = AudioBuffer16k::SAMPLE_RATE_HZ as f64 / native_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: RESAMPLER_SINC_LEN,
            f_cutoff: RESAMPLER_F_CUTOFF,
            oversampling_factor: RESAMPLER_OVERSAMPLING,
            interpolation: SincInterpolationType::Linear,
            window: WindowFunction::BlackmanHarris2,
        };
        let inner = SincFixedIn::<f32>::new(ratio, 1.0, params, RESAMPLER_CHUNK_SIZE, 1)
            .map_err(|e| anyhow!("resampler construction failed: {e}"))?;
        Ok(Self { inner, ratio })
    }

    fn resample(&mut self, mono_i16: &[i16]) -> Result<Vec<i16>> {
        self.inner.reset();

        let input_f32: Vec<f32> = mono_i16.iter().map(|&s| s as f32 / I16_MAX_F32).collect();
        let total_frames = input_f32.len();
        let mut output_samples: Vec<f32> = Vec::with_capacity(
            ((total_frames as f64) * self.ratio).ceil() as usize + RESAMPLER_CHUNK_SIZE,
        );

        let mut pos = 0usize;
        while pos < total_frames {
            let end = (pos + RESAMPLER_CHUNK_SIZE).min(total_frames);
            let chunk = &input_f32[pos..end];

            let padded: Vec<f32> = if chunk.len() < RESAMPLER_CHUNK_SIZE {
                let mut p = chunk.to_vec();
                p.resize(RESAMPLER_CHUNK_SIZE, 0.0);
                p
            } else {
                chunk.to_vec()
            };

            let output = self
                .inner
                .process(&[padded], None)
                .map_err(|e| anyhow!("resampler process failed: {e}"))?;

            output_samples.extend_from_slice(&output[0]);
            pos += RESAMPLER_CHUNK_SIZE;
        }

        Ok(output_samples.into_iter().map(float_to_i16).collect())
    }
}

fn live_watcher_thread(
    buffer: Arc<Mutex<Vec<i16>>>,
    native_sample_rate: u32,
    cfg: ChunkingCfg,
    chunk_tx: std::sync::mpsc::Sender<LiveChunk>,
    stop_rx: std::sync::mpsc::Receiver<()>,
) {
    let chunk_samples = cfg.chunk_threshold_secs as usize * native_sample_rate as usize;
    let overlap_samples = cfg.chunk_overlap_secs as usize * native_sample_rate as usize;

    let mut resampler: Option<Resampler16k> =
        if native_sample_rate == AudioBuffer16k::SAMPLE_RATE_HZ {
            None
        } else {
            match Resampler16k::new(native_sample_rate) {
                Ok(r) => Some(r),
                Err(e) => {
                    log::error!("live-watcher: resampler init failed: {e:#}");
                    return;
                }
            }
        };

    let mut next_start: usize = 0;
    let mut stopping = false;

    loop {
        stopping |= match stop_rx.try_recv() {
            Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => true,
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
        };

        let current_len = buffer.lock().unwrap().len();
        let available = current_len.saturating_sub(next_start);

        if !stopping && available <= chunk_samples {
            std::thread::sleep(std::time::Duration::from_millis(20));
            continue;
        }

        if stopping && next_start >= current_len {
            break;
        }
        // After the stop, audio longer than a chunk threshold is still cut first, so the final
        // chunk fits the threshold too.
        let should_stop = stopping && available <= chunk_samples;

        let t_detect = Instant::now();

        let (chunk, cut_point) = if should_stop {
            let chunk = samples_between(&buffer.lock().unwrap(), next_start, current_len);
            let cut = current_len;
            (chunk, cut)
        } else {
            let buf_guard = buffer.lock().unwrap();
            if next_start >= buf_guard.len() {
                // The recording was cut before this chunk: nothing is left to send.
                drop(buf_guard);
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            }
            let cut = next_cut(&buf_guard, native_sample_rate, next_start, cfg);
            let chunk = samples_between(&buf_guard, next_start, cut);
            (chunk, cut)
        };

        let t_after_slice = Instant::now();
        let chunk_secs = chunk.len() as f32 / native_sample_rate as f32;

        let resampled_result: Result<AudioBuffer16k> = match resampler.as_mut() {
            None => Ok(AudioBuffer16k::new(chunk)),
            Some(r) => r.resample(&chunk).map(AudioBuffer16k::new),
        };

        match resampled_result {
            Ok(audio) => {
                let t_after_resample = Instant::now();
                let enqueued_at = Instant::now();
                let send_failed = chunk_tx.send(LiveChunk { audio, enqueued_at }).is_err();
                let t_after_send = Instant::now();

                if should_stop {
                    log::info!(
                        "live-watcher final-chunk: chunk_secs={:.2} slice={}ms resample={}ms send={}ms total={}ms",
                        chunk_secs,
                        (t_after_slice - t_detect).as_millis(),
                        (t_after_resample - t_after_slice).as_millis(),
                        (t_after_send - t_after_resample).as_millis(),
                        (t_after_send - t_detect).as_millis(),
                    );
                }

                if send_failed {
                    break;
                }
            }
            Err(e) => {
                log::warn!("live-watcher: resample failed: {e:#}");
            }
        }

        next_start = next_chunk_start(next_start, cut_point, overlap_samples);

        if should_stop {
            break;
        }
    }
}

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * I16_MAX_F32) as i16
}

#[cfg(test)]
mod lead_in_tests {
    use super::*;

    #[test]
    fn should_upsample_a_second_of_16_khz_to_a_second_at_the_device_rate() {
        // Given
        let second: Vec<i16> = (0..16_000).map(|i| (i % 100) as i16).collect();
        // When
        let at_48k = upsampled(&second, 48_000);
        // Then
        assert_eq!(at_48k.len(), 48_000);
        assert_eq!(at_48k[0], 0);
        assert_eq!(at_48k[3], second[1]);
    }

    #[test]
    fn should_keep_16_khz_audio_as_it_is() {
        assert_eq!(upsampled(&[1, 2, 3], 16_000), [1, 2, 3]);
    }

    #[test]
    fn should_put_the_lead_in_before_the_recorded_samples() {
        // Given a quarter second heard, and what the recorder captured itself
        let lead_in = AudioBuffer16k::new(vec![7; 4_000]);
        let mut buffer = vec![1, 2, 3];
        // When
        let lead_in_time = put_first(&mut buffer, &lead_in, 48_000);
        // Then
        assert_eq!(lead_in_time, Duration::from_millis(250));
        assert_eq!(buffer.len(), 12_000 + 3);
        assert_eq!(buffer[..12_000], [7; 12_000]);
        assert_eq!(buffer[12_000..], [1, 2, 3]);
    }
}

#[cfg(test)]
mod last_recording_tests {
    use super::*;

    const DEVICE_RATE_HZ: u32 = 48_000;
    const RESAMPLED_LENGTH_TOLERANCE: usize = 20;

    #[test]
    fn should_give_the_last_recording_at_16_khz_with_its_lead_in() {
        // Given half a second heard before the start and a second recorded at 48 kHz
        let lead_in = AudioBuffer16k::new(vec![0; 8_000]);
        let mut samples = vec![0i16; DEVICE_RATE_HZ as usize];
        put_first(&mut samples, &lead_in, DEVICE_RATE_HZ);
        let recording = NativeRecording {
            samples,
            rate: DEVICE_RATE_HZ,
        };
        // When
        let audio = recording.at_16k().unwrap();
        // Then a second and a half at 16 kHz
        let expected = 24_000usize;
        assert!(
            audio.samples().len().abs_diff(expected) <= expected / RESAMPLED_LENGTH_TOLERANCE,
            "expected ~{expected} samples, got {}",
            audio.samples().len()
        );
    }

    #[test]
    fn should_give_a_16_khz_recording_as_it_is() {
        // Given
        let recording = NativeRecording {
            samples: vec![1, 2, 3],
            rate: AudioBuffer16k::SAMPLE_RATE_HZ,
        };
        // When
        let audio = recording.at_16k().unwrap();
        // Then
        assert_eq!(audio.samples(), [1, 2, 3]);
    }
}

#[cfg(test)]
mod tests {
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
    fn should_return_err_when_stop_called_without_start() {
        // Given
        let mut recorder = CpalAudioRecorder::new(600, Arc::new(AtomicU32::new(0)));
        // When
        let result = recorder.stop();
        // Then
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("recorder was not started"));
    }

    #[test]
    fn should_produce_correct_sample_count_when_resampling_from_44100_to_16k() {
        // Given — 1 second of 44100 Hz silence
        let input: Vec<i16> = vec![0i16; 44_100];
        let mut resampler = Resampler16k::new(44_100).unwrap();
        // When
        let result = resampler.resample(&input).unwrap();
        // Then — output should be approximately 16000 samples (within 5%)
        let expected = 16_000usize;
        let tolerance = expected / 20;
        assert!(
            result.len().abs_diff(expected) <= tolerance,
            "expected ~{expected} samples, got {}",
            result.len()
        );
    }

    #[test]
    fn should_produce_consistent_results_across_repeated_resamples() {
        // Given — same buffer resampled twice through the same resampler
        let input: Vec<i16> = (0..44_100).map(|i| ((i % 1000) as i16) * 10).collect();
        let mut resampler = Resampler16k::new(44_100).unwrap();
        // When
        let first = resampler.resample(&input).unwrap();
        let second = resampler.resample(&input).unwrap();
        // Then — reset() between calls means identical inputs produce identical outputs
        assert_eq!(
            first.len(),
            second.len(),
            "output length should be stable across repeated resamples"
        );
        assert_eq!(
            first, second,
            "samples should be byte-identical when reset() is called between buffers"
        );
    }

    #[test]
    fn should_keep_one_second_of_samples_for_a_cut_one_second_after_the_start() {
        // Given
        let started = Instant::now();
        let cut = started + std::time::Duration::from_secs(1);
        // When
        let kept = samples_before(started, cut, 16_000);
        // Then
        assert_eq!(kept, 16_000);
    }

    #[test]
    fn should_keep_no_samples_for_a_cut_before_the_start() {
        // Given
        let cut = Instant::now();
        let started = cut + std::time::Duration::from_millis(500);
        // When
        let kept = samples_before(started, cut, 48_000);
        // Then
        assert_eq!(kept, 0);
    }

    #[test]
    fn should_keep_every_sample_captured_before_a_late_cut() {
        // Given
        let buffer = Arc::new(Mutex::new(vec![1i16; 8_000]));
        let started = Instant::now();
        // When
        truncate_buffer(
            &buffer,
            samples_before(started, started + std::time::Duration::from_secs(2), 16_000),
        );
        // Then
        assert_eq!(buffer.lock().unwrap().len(), 8_000);
    }

    #[test]
    fn should_take_only_the_samples_left_when_the_buffer_was_cut_after_its_length_was_read() {
        // Given the watcher read 16 000 samples, then the recording was cut to 10 000
        let buffer = Arc::new(Mutex::new(vec![1i16; 16_000]));
        let read_len = buffer.lock().unwrap().len();
        truncate_buffer(&buffer, 10_000);
        // When
        let chunk = samples_between(&buffer.lock().unwrap(), 4_000, read_len);
        // Then
        assert_eq!(chunk.len(), 6_000);
    }

    #[test]
    fn should_take_nothing_when_the_cut_is_before_the_chunk_start() {
        // Given
        let buffer = vec![1i16; 3_000];
        // When
        let chunk = samples_between(&buffer, 4_000, 16_000);
        // Then
        assert!(chunk.is_empty());
    }

    #[test]
    fn should_store_level_bits_via_audio_level_sink() {
        // Given
        let reg = Arc::new(AtomicU32::new(0));
        let recorder = CpalAudioRecorder::new(600, Arc::clone(&reg));
        // When
        recorder.write_level(AudioLevel::from_rms(0.5));
        // Then
        assert_eq!(reg.load(Ordering::Relaxed), 0.5f32.to_bits());
    }
}
