//! Finds the quiet pause that ends an utterance while the voice activity detector still holds
//! it as speech: after a quiet word, its speech probability stays high for up to half a second.

use std::collections::VecDeque;

/// 10 ms at 16 kHz: the stretch whose loudness is measured.
pub(super) const FRAME_SAMPLES: usize = 160;
/// A frame at most this share of the loudest frame of its utterance is quiet: the "s" and
/// "t" ending a word in a loud voice still reach 5 % of its vowels.
const QUIET_FRACTION: f32 = 0.03;
/// A frame at most this many times the room's noise floor is quiet: a quiet voice's pause
/// is room noise, not a share of its speech.
const FLOOR_FACTOR: f32 = 3.0;
/// The most of the loudest frame a noise floor makes quiet: in a noisy room the end of a word
/// still stands out of the noise.
const FLOOR_CAP_FRACTION: f32 = 0.3;
/// How far back the noise floor is the quietest frame: 3 s.
const FLOOR_FRAMES: usize = 300;
/// Frames at the edges of a pause that hold the end of one word or the start of the next:
/// a pause of the minimum silence has this many fewer whole quiet frames.
const EDGE_FRAMES: usize = 2;
/// Quieter than this, an utterance has had no speech yet to measure a pause against.
const NO_SPEECH_RMS: f32 = 1e-4;

/// The loudness of the utterance heard now, and how long it has been quiet.
pub(super) struct PauseWatch {
    min_quiet_frames: usize,
    loudest: f32,
    /// Loudness of the last frames heard, for the noise floor.
    recent: VecDeque<f32>,
    quiet_frames: usize,
    quiet_from: usize,
    pending: Vec<f32>,
    /// Sample position of the first sample in `pending`.
    position: usize,
}

impl PauseWatch {
    /// Watches for a quiet run of `min_silence_samples`.
    pub(super) fn new(min_silence_samples: usize) -> Self {
        Self {
            min_quiet_frames: (min_silence_samples / FRAME_SAMPLES)
                .saturating_sub(EDGE_FRAMES)
                .max(1),
            loudest: 0.0,
            recent: VecDeque::with_capacity(FLOOR_FRAMES),
            quiet_frames: 0,
            quiet_from: 0,
            pending: Vec::new(),
            position: 0,
        }
    }

    /// Hears `samples`, the stream's next ones; `speaking` tells whether an utterance runs.
    /// Gives the sample position where a quiet run began once it is long enough to end the
    /// utterance.
    pub(super) fn hear(&mut self, samples: &[f32], speaking: bool) -> Option<usize> {
        self.pending.extend_from_slice(samples);
        let mut pause = None;
        let frames = self.pending.len() / FRAME_SAMPLES;
        for n in 0..frames {
            let frame = &self.pending[n * FRAME_SAMPLES..(n + 1) * FRAME_SAMPLES];
            let at = self.position + n * FRAME_SAMPLES;
            let loudness = rms(frame);
            if self.recent.len() == FLOOR_FRAMES {
                self.recent.pop_front();
            }
            self.recent.push_back(loudness);
            if !speaking {
                self.forget();
                continue;
            }
            if self.is_quiet(loudness) {
                if self.quiet_frames == 0 {
                    self.quiet_from = at;
                }
                self.quiet_frames += 1;
                if self.quiet_frames >= self.min_quiet_frames && pause.is_none() {
                    pause = Some(self.quiet_from);
                }
            } else {
                self.loudest = self.loudest.max(loudness);
                self.quiet_frames = 0;
            }
        }
        self.pending.drain(..frames * FRAME_SAMPLES);
        self.position += frames * FRAME_SAMPLES;
        pause
    }

    /// The loudness at or below which a frame of the utterance heard now is quiet.
    pub(super) fn quiet_level(&self) -> f32 {
        let floor = self.recent.iter().copied().fold(f32::INFINITY, f32::min);
        let near_floor = (floor * FLOOR_FACTOR).min(self.loudest * FLOOR_CAP_FRACTION);
        (self.loudest * QUIET_FRACTION).max(near_floor)
    }

    /// Starts over for the next utterance.
    pub(super) fn forget(&mut self) {
        self.loudest = 0.0;
        self.quiet_frames = 0;
    }

    fn is_quiet(&self, loudness: f32) -> bool {
        self.loudest > NO_SPEECH_RMS && loudness <= self.quiet_level()
    }
}

/// Where the sound after a segment's end first rises above `quiet_level`, as an offset into
/// `after_end`; the whole of it when it stays quiet.
pub(super) fn first_sound(after_end: &[f32], quiet_level: f32) -> usize {
    after_end
        .chunks(FRAME_SAMPLES)
        .position(|frame| rms(frame) > quiet_level)
        .map_or(after_end.len(), |n| n * FRAME_SAMPLES)
}

fn rms(frame: &[f32]) -> f32 {
    (frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.3 s at 16 kHz.
    const MIN_SILENCE_SAMPLES: usize = 4_800;

    fn tone(frames: usize, level: f32) -> Vec<f32> {
        vec![level; frames * FRAME_SAMPLES]
    }

    #[test]
    fn should_end_an_utterance_where_a_quiet_run_of_its_minimum_silence_began() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        watch.hear(&tone(20, 0.01), true);
        // When
        let pause = watch.hear(&tone(30, 0.001), true);
        // Then
        assert_eq!(pause, Some(20 * FRAME_SAMPLES));
    }

    #[test]
    fn should_not_end_an_utterance_before_its_quiet_run_is_long_enough() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        watch.hear(&tone(20, 0.01), true);
        // When
        let pause = watch.hear(&tone(27, 0.001), true);
        // Then
        assert_eq!(pause, None);
    }

    #[test]
    fn should_count_a_frame_louder_than_its_share_of_the_loudest_as_speech() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        watch.hear(&tone(100, 0.0), false);
        watch.hear(&tone(20, 0.1), true);
        let mut audio = tone(20, 0.0);
        audio.extend(tone(1, 0.005));
        audio.extend(tone(20, 0.0));
        // When
        let pause = watch.hear(&audio, true);
        // Then
        assert_eq!(pause, None);
    }

    #[test]
    fn should_count_a_frame_near_the_noise_floor_as_quiet() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        watch.hear(&tone(100, 0.001), false);
        watch.hear(&tone(20, 0.01), true);
        // When
        let pause = watch.hear(&tone(30, 0.0025), true);
        // Then
        assert_eq!(pause, Some(120 * FRAME_SAMPLES));
    }

    #[test]
    fn should_restart_the_quiet_run_after_a_louder_frame() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        watch.hear(&tone(20, 0.01), true);
        let mut audio = tone(20, 0.0);
        audio.extend(tone(1, 0.01));
        audio.extend(tone(30, 0.0));
        // When
        let pause = watch.hear(&audio, true);
        // Then
        assert_eq!(pause, Some(41 * FRAME_SAMPLES));
    }

    #[test]
    fn should_find_no_pause_before_any_speech() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        // When
        let pause = watch.hear(&tone(60, 0.0), true);
        // Then
        assert_eq!(pause, None);
    }

    #[test]
    fn should_find_no_pause_while_no_utterance_runs() {
        // Given
        let mut watch = PauseWatch::new(MIN_SILENCE_SAMPLES);
        watch.hear(&tone(20, 0.01), true);
        // When
        let pause = watch.hear(&tone(60, 0.0), false);
        // Then
        assert_eq!(pause, None);
    }

    #[test]
    fn should_place_the_first_sound_after_a_segment_end_at_its_first_loud_frame() {
        // Given
        let mut after_end = tone(5, 0.0);
        after_end.extend(tone(3, 0.05));
        // When
        let at = first_sound(&after_end, 0.01);
        // Then
        assert_eq!(at, 5 * FRAME_SAMPLES);
    }

    #[test]
    fn should_place_no_sound_after_a_segment_end_that_stays_quiet() {
        // Given
        let after_end = tone(8, 0.001);
        // When
        let at = first_sound(&after_end, 0.01);
        // Then
        assert_eq!(at, after_end.len());
    }
}
