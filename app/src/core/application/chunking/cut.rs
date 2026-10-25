//! Cutting a recording into chunks in its pauses.

use super::{AudioBuffer16k, ChunkingCfg};
use crate::core::domain::conversions::saturating_usize;

/// Length of the frames whose loudness the pause search measures.
pub(super) const QUIET_FRAME_MS: usize = 20;

/// Shortest quiet stretch that counts as a pause: longer than a microphone dropout (120 ms),
/// shorter than most gaps between phrases (150–250 ms).
pub(super) const MIN_PAUSE_MS: usize = 160;

/// Percentile of the frame loudness of a chunk that stands for its noise floor: a chunk of
/// speech pauses at least this share of the time.
pub(super) const NOISE_FLOOR_PERCENTILE: f32 = 0.1;

/// A frame is quiet up to this multiple of the noise floor.
pub(super) const NOISE_FLOOR_FACTOR: f32 = 2.0;

/// ... but never above this share of the median frame, so speech without any pause, whose
/// noise floor is itself speech, never counts as quiet.
pub(super) const SPEECH_LEVEL_SHARE: f32 = 0.5;

/// A frame this quiet is quiet whatever the noise floor: digital silence makes the floor 0.
pub(super) const MIN_QUIET_RMS: f32 = 0.002;

/// Where the chunk that starts at `chunk_start` ends.
///
/// The middle of the longest pause (the latest of equally long ones) within the lookback before the
/// hard boundary (`chunk_threshold_secs` after `chunk_start`, so the chunk, its overlap with the
/// previous one included, fits the threshold), or the hard boundary when there is none.
///
/// A pause is a stretch of at least [`MIN_PAUSE_MS`] of frames quieter than the chunk's noise floor
/// allows; it is searched after the chunk's overlap only.
pub fn next_cut(samples: &[i16], sample_rate: u32, chunk_start: usize, cfg: ChunkingCfg) -> usize {
    let rate = sample_rate as usize;
    let hard_boundary = (chunk_start + cfg.chunk_threshold_secs as usize * rate).min(samples.len());
    let lookback_start = hard_boundary
        .saturating_sub(cfg.vad_lookback_secs as usize * rate)
        .max(chunk_start + cfg.chunk_overlap_secs as usize * rate);
    let frame = rate * QUIET_FRAME_MS / 1000;
    if frame == 0 || hard_boundary <= lookback_start {
        return hard_boundary;
    }
    let loudness: Vec<f32> = samples[chunk_start..hard_boundary]
        .chunks(frame)
        .map(rms)
        .collect();
    let level = quiet_level(&loudness);
    let first = (lookback_start - chunk_start).div_ceil(frame);
    match longest_pause(&loudness[first.min(loudness.len())..], level) {
        Some((start, end)) => chunk_start + (first + start + first + end) * frame / 2,
        None => hard_boundary,
    }
}

/// The longest run of frames quieter than `level`, of at least [`MIN_PAUSE_MS`], as frame
/// indexes `(first, past the last)`; the later of two equal runs.
fn longest_pause(loudness: &[f32], level: f32) -> Option<(usize, usize)> {
    let min_frames = MIN_PAUSE_MS.div_ceil(QUIET_FRAME_MS);
    let mut longest: Option<(usize, usize)> = None;
    let mut run_start = None;
    for i in 0..=loudness.len() {
        let quiet = i < loudness.len() && loudness[i] < level;
        match (quiet, run_start) {
            (true, None) => run_start = Some(i),
            (false, Some(start)) => {
                let len = i - start;
                if len >= min_frames && longest.is_none_or(|(s, e)| len >= e - s) {
                    longest = Some((start, i));
                }
                run_start = None;
            }
            _ => {}
        }
    }
    longest
}

/// Loudness below which a frame of this chunk is quiet, from its noise floor.
pub(super) fn quiet_level(loudness: &[f32]) -> f32 {
    let mut sorted = loudness.to_vec();
    sorted.sort_by(f32::total_cmp);
    let at = |share: f32| sorted[saturating_usize((sorted.len() - 1) as f32 * share)];
    (NOISE_FLOOR_FACTOR * at(NOISE_FLOOR_PERCENTILE))
        .min(SPEECH_LEVEL_SHARE * at(0.5))
        .max(MIN_QUIET_RMS)
}

/// Root mean square of `samples`, as a share of full scale.
pub(super) fn rms(samples: &[i16]) -> f32 {
    let mean_square = samples
        .iter()
        .map(|&s| (f32::from(s) / 32_768.0).powi(2))
        .sum::<f32>()
        / samples.len() as f32;
    mean_square.sqrt()
}

/// Splits a whole recording the way a live dictation is split.
///
/// While more than a chunk threshold of audio follows the start of the chunk, cut it with
/// [`next_cut`] and start the next chunk the overlap before the cut.
pub fn split_into_chunks(audio: &AudioBuffer16k, cfg: ChunkingCfg) -> Vec<AudioBuffer16k> {
    let samples = audio.samples();
    let rate = AudioBuffer16k::SAMPLE_RATE_HZ as usize;
    let chunk_len = cfg.chunk_threshold_secs as usize * rate;
    if chunk_len == 0 || samples.len() <= chunk_len {
        return vec![audio.clone()];
    }
    let overlap = cfg.chunk_overlap_secs as usize * rate;
    let mut chunks = Vec::new();
    let mut start = 0;
    while samples.len() - start > chunk_len {
        let cut = next_cut(samples, AudioBuffer16k::SAMPLE_RATE_HZ, start, cfg);
        chunks.push(AudioBuffer16k::new(samples[start..cut].to_vec()));
        start = next_chunk_start(start, cut, overlap);
    }
    chunks.push(AudioBuffer16k::new(samples[start..].to_vec()));
    chunks
}

/// Where the chunk after a `cut` starts: the overlap before the cut, or at the cut when the
/// overlap would reach back to the start of the chunk before.
pub fn next_chunk_start(chunk_start: usize, cut: usize, overlap: usize) -> usize {
    match cut.saturating_sub(overlap) {
        start if start > chunk_start => start,
        _ => cut,
    }
}
