//! Resampling of a continuous microphone stream to 16 kHz, shared by the dictation
//! recorder and the command spotting capture.

use anyhow::{anyhow, Result};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::core::domain::conversions::saturating_usize;
use crate::core::domain::AudioBuffer16k;

const CHUNK_FRAMES: usize = 1_024;
const SINC_LEN: usize = 256;
const F_CUTOFF: f32 = 0.95;
const OVERSAMPLING: usize = 128;

/// Resamples a continuous stream to 16 kHz, keeping the filter state between pushes, so a
/// stream pushed in pieces of any size comes out as one resampled whole.
pub struct StreamResampler {
    inner: Option<SincFixedIn<f32>>,
    pending: Vec<f32>,
    ratio: f64,
    frames_in: usize,
    frames_out: usize,
}

impl StreamResampler {
    /// A resampler from `native_rate` to 16 kHz; at 16 kHz the audio passes through.
    ///
    /// # Errors
    /// When the filter cannot be built for `native_rate`.
    pub fn new(native_rate: u32) -> Result<Self> {
        if native_rate == AudioBuffer16k::SAMPLE_RATE_HZ {
            return Ok(Self {
                inner: None,
                pending: Vec::new(),
                ratio: 1.0,
                frames_in: 0,
                frames_out: 0,
            });
        }
        let params = SincInterpolationParameters {
            sinc_len: SINC_LEN,
            f_cutoff: F_CUTOFF,
            oversampling_factor: OVERSAMPLING,
            interpolation: SincInterpolationType::Linear,
            window: WindowFunction::BlackmanHarris2,
        };
        let ratio = f64::from(AudioBuffer16k::SAMPLE_RATE_HZ) / f64::from(native_rate);
        let inner = SincFixedIn::<f32>::new(ratio, 1.0, params, CHUNK_FRAMES, 1)
            .map_err(|e| anyhow!("resampler construction failed: {e}"))?;
        Ok(Self { inner: Some(inner), pending: Vec::new(), ratio, frames_in: 0, frames_out: 0 })
    }

    /// The 16 kHz samples of every full filter block `samples` completes.
    pub fn push(&mut self, samples: &[f32]) -> Vec<f32> {
        let Some(inner) = self.inner.as_mut() else {
            return samples.to_vec();
        };
        self.frames_in += samples.len();
        self.pending.extend_from_slice(samples);
        let mut out = Vec::new();
        while self.pending.len() >= CHUNK_FRAMES {
            let chunk: Vec<f32> = self.pending.drain(..CHUNK_FRAMES).collect();
            match inner.process(&[chunk], None) {
                Ok(resampled) => out.extend_from_slice(&resampled[0]),
                Err(e) => log::warn!("resample failed: {e}"),
            }
        }
        self.frames_out += out.len();
        out
    }

    /// The 16 kHz samples still held back: the remainder of the last block and the filter
    /// delay, so the stream ends where the input did. The resampler then starts afresh.
    pub fn finish(&mut self) -> Vec<f32> {
        let Some(inner) = self.inner.as_mut() else {
            return Vec::new();
        };
        let rest: Vec<f32> = std::mem::take(&mut self.pending);
        let mut out = flushed(inner, Some(&rest));
        let expected = saturating_usize((self.frames_in as f64 * self.ratio).round());
        while self.frames_out + out.len() < expected {
            let more = flushed(inner, None);
            if more.is_empty() {
                break;
            }
            out.extend(more);
        }
        out.truncate(expected.saturating_sub(self.frames_out));
        inner.reset();
        self.frames_in = 0;
        self.frames_out = 0;
        out
    }
}

/// The output of the filter for `rest` padded with silence, or for silence alone.
fn flushed(inner: &mut SincFixedIn<f32>, rest: Option<&[f32]>) -> Vec<f32> {
    match inner.process_partial(rest.map(|r| [r]).as_ref().map(<[_; 1]>::as_slice), None) {
        Ok(mut resampled) => resampled.swap_remove(0),
        Err(e) => {
            log::warn!("resample flush failed: {e}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE_48K: u32 = 48_000;
    const RATIO_48K_TO_16K: usize = 3;
    /// The sinc window's half length at 16 kHz: the output tapers over it at the stream's end.
    const FILTER_EDGE_SAMPLES: usize = SINC_LEN / 2 / RATIO_48K_TO_16K + 1;

    fn through(rate: u32, pieces: &[&[f32]]) -> Vec<f32> {
        let mut resampler = StreamResampler::new(rate).unwrap();
        let mut out: Vec<f32> = pieces
            .iter()
            .flat_map(|piece| resampler.push(piece))
            .collect();
        out.extend(resampler.finish());
        out
    }

    #[test]
    fn should_pass_16_khz_audio_through_unchanged() {
        let out = through(AudioBuffer16k::SAMPLE_RATE_HZ, &[&[0.1, 0.2]]);
        assert_eq!(out, [0.1, 0.2]);
    }

    #[test]
    fn should_end_where_the_input_ends_without_trailing_silence() {
        // Given a second and a bit of a steady tone at 48 kHz
        let frames = RATE_48K as usize + 500;
        let input = vec![0.5f32; frames];
        // When
        let out = through(RATE_48K, &[&input]);
        // Then exactly the input's length at 16 kHz, the tone held up to the filter's edge
        assert_eq!(
            out.len(),
            frames / RATIO_48K_TO_16K + usize::from(frames % RATIO_48K_TO_16K != 0)
        );
        let before_edge = out[out.len() - FILTER_EDGE_SAMPLES];
        assert!(
            (before_edge - 0.5).abs() < 0.05,
            "sample before the edge {before_edge}"
        );
        assert!(
            *out.last().unwrap() > 0.1,
            "last sample {}",
            out.last().unwrap()
        );
    }

    #[test]
    fn should_place_the_output_where_the_input_was_in_time() {
        // Given silence, then a tone from the tenth of a second on
        let step_at = RATE_48K as usize / 10;
        let mut input = vec![0.0f32; RATE_48K as usize];
        input[step_at..].fill(0.5);
        // When
        let out = through(RATE_48K, &[&input]);
        // Then the tone starts at the tenth of a second of the output
        let first_loud = out.iter().position(|&s| s > 0.25).unwrap();
        let expected = step_at / RATIO_48K_TO_16K;
        assert!(
            first_loud.abs_diff(expected) <= 2,
            "tone at {first_loud}, expected {expected}"
        );
    }

    #[test]
    fn should_resample_pieces_of_any_size_as_one_whole() {
        // Given a sine at 48 kHz
        let input: Vec<f32> = (0..RATE_48K).map(|i| (i as f32 * 0.01).sin()).collect();
        // When pushed whole and pushed in odd pieces
        let whole = through(RATE_48K, &[&input]);
        let pieces: Vec<&[f32]> = input.chunks(333).collect();
        let in_pieces = through(RATE_48K, &pieces);
        // Then the same samples
        assert_eq!(whole, in_pieces);
    }

    #[test]
    fn should_resample_a_second_of_any_common_microphone_rate_to_about_a_second_of_16_khz() {
        for rate in [24_000, 44_100, 48_000, 96_000] {
            let input = vec![0.1f32; rate as usize];
            let out = through(rate, &[&input]);
            let expected = AudioBuffer16k::SAMPLE_RATE_HZ as usize;
            assert!(
                out.len().abs_diff(expected) <= 1,
                "{rate} Hz gave {} samples",
                out.len()
            );
        }
    }
}
