//! Microphone capture for the command spotting service: mono `f32` at 16 kHz, in chunks.

use std::sync::mpsc::Sender;
use std::time::Instant;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Stream, StreamConfig};

use crate::adapters::secondary::recording::cpal::devices;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

pub const SAMPLE_RATE_HZ: u32 = 16_000;
const RESAMPLER_CHUNK: usize = 1_024;
const RESAMPLER_SINC_LEN: usize = 256;
const RESAMPLER_F_CUTOFF: f32 = 0.95;
const RESAMPLER_OVERSAMPLING: usize = 128;

pub struct MicCapture {
    _stream: Stream,
    pub started_at: Instant,
}

impl MicCapture {
    /// Opens the input device `input_device` names (see [`devices::open`]); every
    /// captured chunk goes to `chunks`, mono at the device's own rate, returned beside it.
    pub fn open(chunks: Sender<Vec<f32>>, input_device: &str) -> Result<(Self, u32)> {
        let device = devices::open(input_device)?;
        let supported = device.default_input_config()?;
        let channels = supported.channels() as usize;
        let rate = supported.sample_rate();
        let config = StreamConfig {
            channels: supported.channels(),
            sample_rate: rate,
            buffer_size: cpal::BufferSize::Default,
        };
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = chunks.send(to_mono(data, channels));
            },
            |err| log::error!("command spotting capture error: {err}"),
            None,
        )?;
        stream.play()?;
        let capture = Self { _stream: stream, started_at: Instant::now() };
        Ok((capture, rate))
    }
}

/// Interleaved frames of `channels` samples, averaged into one channel.
fn to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    interleaved
        .chunks_exact(channels.max(1))
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Resamples a continuous stream to 16 kHz, keeping the filter state between chunks.
pub struct StreamResampler {
    inner: Option<SincFixedIn<f32>>,
    pending: Vec<f32>,
}

impl StreamResampler {
    pub fn new(native_rate: u32) -> Result<Self> {
        if native_rate == SAMPLE_RATE_HZ {
            return Ok(Self { inner: None, pending: Vec::new() });
        }
        let params = SincInterpolationParameters {
            sinc_len: RESAMPLER_SINC_LEN,
            f_cutoff: RESAMPLER_F_CUTOFF,
            oversampling_factor: RESAMPLER_OVERSAMPLING,
            interpolation: SincInterpolationType::Linear,
            window: WindowFunction::BlackmanHarris2,
        };
        let ratio = f64::from(SAMPLE_RATE_HZ) / f64::from(native_rate);
        let inner = SincFixedIn::<f32>::new(ratio, 1.0, params, RESAMPLER_CHUNK, 1)
            .map_err(|e| anyhow!("resampler construction failed: {e}"))?;
        Ok(Self { inner: Some(inner), pending: Vec::new() })
    }

    /// The 16 kHz samples of every full resampler chunk `samples` completes.
    pub fn push(&mut self, samples: &[f32]) -> Vec<f32> {
        let Some(inner) = self.inner.as_mut() else {
            return samples.to_vec();
        };
        self.pending.extend_from_slice(samples);
        let mut out = Vec::new();
        while self.pending.len() >= RESAMPLER_CHUNK {
            let chunk: Vec<f32> = self.pending.drain(..RESAMPLER_CHUNK).collect();
            match inner.process(&[chunk], None) {
                Ok(resampled) => out.extend_from_slice(&resampled[0]),
                Err(e) => log::warn!("command spotting resample failed: {e}"),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pass_16_khz_audio_through_unchanged() {
        let mut resampler = StreamResampler::new(SAMPLE_RATE_HZ).unwrap();
        assert_eq!(resampler.push(&[0.1, 0.2]), [0.1, 0.2]);
    }

    fn resampled_second(native_rate: u32) -> usize {
        let mut resampler = StreamResampler::new(native_rate).unwrap();
        let buffer = 512;
        let buffers = native_rate as usize / buffer + 1;
        (0..buffers)
            .map(|_| resampler.push(&vec![0.1; buffer]).len())
            .sum()
    }

    #[test]
    fn should_resample_a_second_of_any_common_microphone_rate_to_about_a_second_of_16_khz() {
        for rate in [24_000, 44_100, 48_000, 96_000] {
            let out = resampled_second(rate);
            let expected = SAMPLE_RATE_HZ as usize;
            assert!(
                out.abs_diff(expected) <= expected / 10,
                "{rate} Hz gave {out} samples"
            );
        }
    }

    #[test]
    fn should_average_stereo_frames_into_mono() {
        assert_eq!(to_mono(&[0.2, 0.4, -0.5, 0.5], 2), [0.3, 0.0]);
    }

    #[test]
    fn should_keep_mono_frames_as_they_are() {
        assert_eq!(to_mono(&[0.1, 0.2], 1), [0.1, 0.2]);
    }

    #[test]
    fn should_resample_a_second_of_48_khz_to_about_a_second_of_16_khz() {
        // Given
        let mut resampler = StreamResampler::new(48_000).unwrap();
        // When
        let out: Vec<f32> = (0..48)
            .flat_map(|_| resampler.push(&[0.0; 1_000]))
            .collect();
        // Then
        let expected = SAMPLE_RATE_HZ as usize;
        assert!(
            out.len().abs_diff(expected) <= expected / 20,
            "got {}",
            out.len()
        );
    }
}
