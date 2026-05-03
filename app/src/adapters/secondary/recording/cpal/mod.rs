use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::core::application::audio_recorder::AudioRecorder;
use crate::core::domain::AudioBuffer16k;

const I16_MAX_F32: f32 = i16::MAX as f32;

const RESAMPLER_CHUNK_SIZE: usize = 1024;
const RESAMPLER_SINC_LEN: usize = 256;
const RESAMPLER_F_CUTOFF: f32 = 0.95;
const RESAMPLER_OVERSAMPLING: usize = 128;

pub struct CpalAudioRecorder {
    debug_save_wav: Option<PathBuf>,
    max_recording_seconds: u64,
    active: Option<ActiveRecording>,
}

struct ActiveRecording {
    stream: Stream,
    native_sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
    buffer: Arc<Mutex<Vec<i16>>>,
    max_samples: usize,
}

impl CpalAudioRecorder {
    pub fn new(debug_save_wav: Option<PathBuf>, max_recording_seconds: u64) -> Self {
        Self {
            debug_save_wav,
            max_recording_seconds,
            active: None,
        }
    }
}

impl AudioRecorder for CpalAudioRecorder {
    fn start(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("no default input device available"))?;

        let supported_config = device.default_input_config()?;
        let native_sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let stream_config = StreamConfig {
            channels,
            sample_rate: native_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let max_samples = self.max_recording_seconds as usize * native_sample_rate as usize;

        let buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_write = Arc::clone(&buffer);

        let channels_usize = channels as usize;
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

        self.active = Some(ActiveRecording {
            stream,
            native_sample_rate,
            channels,
            buffer,
            max_samples,
        });

        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let recording = self
            .active
            .take()
            .ok_or_else(|| anyhow!("recorder was not started"))?;

        drop(recording.stream);

        let raw_samples = Arc::try_unwrap(recording.buffer)
            .map(|m| m.into_inner().unwrap())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone());

        if raw_samples.len() >= recording.max_samples {
            log::warn!(
                "Recording reached the {}-second limit and was cut off at that point. \
                 To allow longer recordings, increase [audio] max_recording_seconds in your config.",
                self.max_recording_seconds
            );
        }

        let resampled = if recording.native_sample_rate == AudioBuffer16k::SAMPLE_RATE_HZ {
            raw_samples
        } else {
            resample_to_16k(raw_samples, recording.native_sample_rate)?
        };

        if let Some(ref path) = self.debug_save_wav {
            save_wav(path, &resampled)?;
        }

        Ok(AudioBuffer16k::new(resampled))
    }
}

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * I16_MAX_F32) as i16
}

fn resample_to_16k(mono_i16: Vec<i16>, native_rate: u32) -> Result<Vec<i16>> {
    let ratio = AudioBuffer16k::SAMPLE_RATE_HZ as f64 / native_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: RESAMPLER_SINC_LEN,
        f_cutoff: RESAMPLER_F_CUTOFF,
        oversampling_factor: RESAMPLER_OVERSAMPLING,
        interpolation: SincInterpolationType::Linear,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(ratio, 1.0, params, RESAMPLER_CHUNK_SIZE, 1)
        .map_err(|e| anyhow!("resampler construction failed: {e}"))?;

    let input_f32: Vec<f32> = mono_i16.iter().map(|&s| s as f32 / I16_MAX_F32).collect();

    let total_frames = input_f32.len();
    let mut output_samples: Vec<f32> =
        Vec::with_capacity(((total_frames as f64) * ratio).ceil() as usize + RESAMPLER_CHUNK_SIZE);

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

        let output = resampler
            .process(&[padded], None)
            .map_err(|e| anyhow!("resampler process failed: {e}"))?;

        output_samples.extend_from_slice(&output[0]);
        pos += RESAMPLER_CHUNK_SIZE;
    }

    Ok(output_samples.into_iter().map(float_to_i16).collect())
}

fn save_wav(path: &PathBuf, samples: &[i16]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: AudioBuffer16k::SAMPLE_RATE_HZ,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
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
        let mut recorder = CpalAudioRecorder::new(None, 600);
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
        // When
        let result = resample_to_16k(input, 44_100).unwrap();
        // Then — output should be approximately 16000 samples (within 5%)
        let expected = 16_000usize;
        let tolerance = expected / 20;
        assert!(
            result.len().abs_diff(expected) <= tolerance,
            "expected ~{expected} samples, got {}",
            result.len()
        );
    }
}
