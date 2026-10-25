//! Where the recording is cut into chunks.

use super::*;
use crate::core::domain::conversions::{saturating_i16, saturating_usize};

const RATE: u32 = 16_000;

fn cfg(threshold_secs: u32) -> ChunkingCfg {
    ChunkingCfg { chunk_threshold_secs: threshold_secs, ..ChunkingCfg::default() }
}

fn tone(secs: usize) -> Vec<i16> {
    vec![10_000i16; secs * RATE as usize]
}

fn secs(at: f32) -> usize {
    saturating_usize(at * RATE as f32)
}

/// Speech-like audio: 180 ms syllables with a 60 ms weak consonant between them, and a
/// 300 ms pause every 2 s before the lookback of the first cut (25 s at a 30 s threshold),
/// so a tenth of the first chunk is pause.
fn speech(total_secs: usize) -> Vec<i16> {
    let mut samples: Vec<i16> = (0..total_secs * RATE as usize)
        .map(|i| if i % 3_840 < 2_880 { 8_000 } else { 3_000 })
        .collect();
    for start in (1..25).step_by(2) {
        silence(&mut samples, start as f32, 0.3);
    }
    samples
}

fn silence(samples: &mut [i16], from_secs: f32, len_secs: f32) {
    samples[secs(from_secs)..secs(from_secs + len_secs)].fill(0);
}

/// Adds steady noise of about `rms` (of full scale) to every sample.
fn add_noise(samples: &mut [i16], rms: f32) {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    for s in samples.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let unit = (state >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0;
        let noise = unit * rms * 3f32.sqrt() * 32_768.0;
        *s =
            saturating_i16((f32::from(*s) + noise).clamp(f32::from(i16::MIN), f32::from(i16::MAX)));
    }
}

fn in_range(cut: usize, from_secs: f32, to_secs: f32) -> bool {
    (secs(from_secs)..secs(to_secs)).contains(&cut)
}

#[test]
fn should_cut_at_the_threshold_when_the_lookback_holds_no_pause() {
    let samples = tone(40);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert_eq!(cut, 30 * RATE as usize);
}

#[test]
fn should_cut_in_a_pause_rather_than_in_a_quiet_moment_of_a_word() {
    let mut samples = speech(40);
    silence(&mut samples, 26.0, 0.3);
    silence(&mut samples, 29.5, 0.04);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert!(
        in_range(cut, 26.0, 26.3),
        "cut at {:.2} s",
        cut as f32 / RATE as f32
    );
}

#[test]
fn should_cut_in_the_middle_of_the_longest_pause() {
    let mut samples = speech(40);
    silence(&mut samples, 25.5, 0.5);
    silence(&mut samples, 28.5, 0.2);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert_eq!(cut, secs(25.75), "cut at {:.2} s", cut as f32 / RATE as f32);
}

#[test]
fn should_take_a_stretch_of_the_minimum_pause_length_for_a_pause() {
    let mut samples = speech(40);
    silence(&mut samples, 27.0, 0.17);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert!(
        in_range(cut, 27.0, 27.17),
        "cut at {:.2} s",
        cut as f32 / RATE as f32
    );
}

#[test]
fn should_cut_in_the_latest_of_equally_long_pauses() {
    let mut samples = speech(40);
    silence(&mut samples, 26.0, 0.3);
    silence(&mut samples, 28.0, 0.3);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert_eq!(cut, secs(28.15), "cut at {:.2} s", cut as f32 / RATE as f32);
}

#[test]
fn should_find_a_pause_under_background_noise() {
    let mut samples = speech(40);
    silence(&mut samples, 26.0, 0.3);
    silence(&mut samples, 29.5, 0.04);
    add_noise(&mut samples, 0.03);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert!(
        in_range(cut, 26.0, 26.3),
        "cut at {:.2} s",
        cut as f32 / RATE as f32
    );
}

#[test]
fn should_not_take_quiet_speech_for_a_pause() {
    let mut samples: Vec<i16> = speech(40).iter().map(|&s| s / 20).collect();
    silence(&mut samples, 26.0, 0.3);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert!(
        in_range(cut, 26.0, 26.3),
        "cut at {:.2} s",
        cut as f32 / RATE as f32
    );
}

#[test]
fn should_not_take_a_short_dropout_for_a_pause() {
    let mut samples = speech(40);
    silence(&mut samples, 27.0, 0.12);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert_eq!(cut, 30 * RATE as usize);
}

#[test]
fn should_ignore_silence_before_the_lookback() {
    let mut samples = tone(40);
    samples[20 * RATE as usize..21 * RATE as usize].fill(0);
    let cut = next_cut(&samples, RATE, 0, cfg(30));
    assert_eq!(cut, 30 * RATE as usize);
}

#[test]
fn should_keep_a_recording_up_to_the_threshold_in_one_chunk() {
    let audio = AudioBuffer16k::new(tone(30));
    let chunks = split_into_chunks(&audio, cfg(30));
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].samples().len(), audio.samples().len());
}

#[test]
fn should_keep_a_recording_in_one_chunk_when_chunking_is_off() {
    let audio = AudioBuffer16k::new(tone(70));
    assert_eq!(split_into_chunks(&audio, cfg(0)).len(), 1);
}

#[test]
fn should_split_a_long_recording_into_overlapping_chunks() {
    let rate = RATE as usize;
    let audio = AudioBuffer16k::new(tone(70));
    let chunks = split_into_chunks(&audio, cfg(30));
    let lens: Vec<usize> = chunks.iter().map(|c| c.samples().len()).collect();
    // cuts at 30 s and 58 s; every chunk after the first starts 2 s before the cut
    assert_eq!(lens, vec![30 * rate, 30 * rate, 14 * rate]);
}

#[test]
fn should_keep_every_chunk_with_its_overlap_within_the_threshold() {
    // a pause early in the lookback, then none: the next cut falls at the hard boundary
    let mut samples = speech(100);
    silence(&mut samples, 26.0, 0.3);
    let chunks = split_into_chunks(&AudioBuffer16k::new(samples), cfg(30));
    let lens: Vec<f32> = chunks
        .iter()
        .map(|c| c.samples().len() as f32 / RATE as f32)
        .collect();
    assert!(chunks.len() > 3, "chunks of {lens:?} s");
    assert!(lens.iter().all(|&len| len <= 30.0), "chunks of {lens:?} s");
}

#[test]
fn should_start_the_next_chunk_the_overlap_before_the_cut() {
    assert_eq!(next_chunk_start(0, secs(26.0), secs(2.0)), secs(24.0));
}

#[test]
fn should_start_the_next_chunk_at_the_cut_when_the_overlap_reaches_the_chunk_start() {
    assert_eq!(
        next_chunk_start(secs(10.0), secs(11.0), secs(2.0)),
        secs(11.0)
    );
}

#[test]
fn should_split_at_a_silence_inside_the_lookback() {
    let rate = RATE as usize;
    let mut samples = tone(50);
    silence(&mut samples, 28.0, 0.3);
    let chunks = split_into_chunks(&AudioBuffer16k::new(samples), cfg(30));
    let lens: Vec<usize> = chunks.iter().map(|c| c.samples().len()).collect();
    // cut in the middle of the pause; the second chunk starts 2 s before the cut
    assert_eq!(lens, vec![secs(28.15), 50 * rate - secs(26.15)]);
}
