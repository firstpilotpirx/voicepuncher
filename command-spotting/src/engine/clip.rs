//! The steady clip: an utterance cut a fixed lead before its first sound and raised toward a
//! normal level when quiet, so the speech recognition service hears the same words alike
//! whatever sound came before them. The second hearing of a near miss sends it.

use super::pause::{rms, FRAME_SAMPLES};
use super::samples_in;

/// A window louder than this share of the clip's loudest is the utterance's first sound: the
/// faint sound the user makes before a quiet "restart" (at most a quarter of its loudest)
/// stays under it, and the "re" (half of it) rises above it.
const ONSET_SHARE: f32 = 0.4;
/// How much comes before the first sound: from 0.2 to 0.3 s the user's quiet "restart" is
/// heard as "Restart." after any silence, with more of the faint sound as "Let's start.".
const LEAD_SECS: f32 = 0.25;
/// The loudest 10 ms a quiet clip is raised toward: about half of `say`'s level.
const TARGET_LEVEL: f32 = 0.1;
/// The most a quiet clip is raised: a room at 0.0005 stays below 0.005.
const MAX_GAIN: f32 = 10.0;
/// How far the window that finds the first sound slides at a time: 1 ms.
const SLIDE_SAMPLES: usize = 16;

/// The audio to send, and the stream position its first sample stands for.
#[derive(Debug, PartialEq)]
pub(super) struct SteadyClip {
    pub samples: Vec<f32>,
    pub origin: usize,
    pub gain: f32,
}

/// The steady clip of `audio`, which starts at stream position `start`; a window no louder
/// than `quiet_level` is never the first sound.
pub(super) fn steady_clip(audio: &[f32], start: usize, quiet_level: f32) -> SteadyClip {
    let loudest = audio.chunks(FRAME_SAMPLES).map(rms).fold(0.0_f32, f32::max);
    let onset = quiet_level.max(loudest * ONSET_SHARE);
    let first = (0..audio.len().saturating_sub(FRAME_SAMPLES))
        .step_by(SLIDE_SAMPLES)
        .find(|&at| rms(&audio[at..at + FRAME_SAMPLES]) > onset)
        .unwrap_or(0);
    let lead = samples_in(LEAD_SECS);
    let cut = first.saturating_sub(lead);
    let padding = lead - (first - cut);
    let gain = if loudest > 0.0 {
        (TARGET_LEVEL / loudest).clamp(1.0, MAX_GAIN)
    } else {
        1.0
    };
    let mut samples = vec![0.0; padding];
    samples.extend(audio[cut..].iter().map(|s| s * gain));
    SteadyClip { samples, origin: (start + first).saturating_sub(lead), gain }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.25 s at 16 kHz.
    const LEAD: usize = 4_000;

    /// Within one 10 ms window: the sliding window finds a sound before all of it is in.
    fn near(at: usize, expected: usize) -> bool {
        at.abs_diff(expected) < FRAME_SAMPLES
    }

    fn tone(secs: f32, level: f32) -> Vec<f32> {
        vec![level; samples_in(secs)]
    }

    /// 1 s of room, 0.5 s of a faint sound, then a word at `level`.
    fn stream(level: f32) -> Vec<f32> {
        let mut audio = tone(1.0, 0.0002);
        audio.extend(tone(0.5, 0.002));
        audio.extend(tone(0.3, level));
        audio
    }

    #[test]
    fn should_start_the_clip_a_fixed_lead_before_the_first_sound_wherever_the_audio_starts() {
        // Given
        let audio = stream(0.1);
        // When
        let early = steady_clip(&audio[samples_in(0.3)..], samples_in(0.3), 0.0);
        let late = steady_clip(&audio[samples_in(0.9)..], samples_in(0.9), 0.0);
        // Then
        assert!(
            early == late,
            "the clips differ: origins {} and {}",
            early.origin,
            late.origin
        );
        assert!(
            near(early.origin, samples_in(1.5) - LEAD),
            "{}",
            early.origin
        );
    }

    #[test]
    fn should_keep_only_the_lead_of_a_faint_sound_under_the_onset() {
        // Given
        let audio = stream(0.1);
        // When
        let clip = steady_clip(&audio, 0, 0.0);
        // Then
        assert!(near(clip.samples.len(), LEAD + samples_in(0.3)));
        assert!(clip.samples[..LEAD - FRAME_SAMPLES]
            .iter()
            .all(|s| (s - 0.002).abs() < 1e-6));
    }

    #[test]
    fn should_leave_out_nothing_louder_than_the_onset() {
        // Given
        let mut audio = tone(0.5, 0.0002);
        audio.extend(tone(0.2, 0.05));
        audio.extend(tone(0.2, 0.1));
        // When
        let clip = steady_clip(&audio, 0, 0.0);
        // Then
        assert!(near(clip.samples.len(), LEAD + samples_in(0.4)));
    }

    #[test]
    fn should_pad_the_lead_with_silence_when_the_audio_starts_closer_to_the_first_sound() {
        // Given
        let mut audio = tone(0.1, 0.0002);
        audio.extend(tone(0.3, 0.1));
        // When
        let clip = steady_clip(&audio, samples_in(2.0), 0.0);
        // Then
        let padding = LEAD - samples_in(0.1) - FRAME_SAMPLES;
        assert!(near(clip.samples.len(), LEAD + samples_in(0.3)));
        assert!(clip.samples[..padding].iter().all(|s| *s == 0.0));
        assert!(near(clip.origin, samples_in(2.0) + samples_in(0.1) - LEAD));
    }

    #[test]
    fn should_not_take_a_sound_at_the_quiet_level_for_the_first_sound() {
        // Given
        let audio = stream(0.004);
        // When
        let by_share = steady_clip(&audio, 0, 0.0);
        let by_quiet_level = steady_clip(&audio, 0, 0.003);
        // Then
        assert!(near(by_share.origin, samples_in(1.0) - LEAD));
        assert!(near(by_quiet_level.origin, samples_in(1.5) - LEAD));
    }

    #[test]
    fn should_raise_a_quiet_clip_toward_a_normal_level() {
        // Given
        let audio = stream(0.02);
        // When
        let clip = steady_clip(&audio, 0, 0.0);
        // Then
        assert!((clip.gain - 5.0).abs() < 1e-4, "{}", clip.gain);
        assert!((clip.samples.last().unwrap() - 0.1).abs() < 1e-6);
    }

    #[test]
    fn should_raise_a_very_quiet_clip_no_more_than_the_cap() {
        // Given
        let audio = stream(0.005);
        // When
        let clip = steady_clip(&audio, 0, 0.0);
        // Then
        assert_eq!(clip.gain, MAX_GAIN);
    }

    #[test]
    fn should_not_lower_a_loud_clip() {
        // Given
        let audio = stream(0.3);
        // When
        let clip = steady_clip(&audio, 0, 0.0);
        // Then
        assert_eq!(clip.gain, 1.0);
        assert_eq!(clip.samples.last(), Some(&0.3));
    }
}
