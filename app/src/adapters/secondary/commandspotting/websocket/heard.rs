//! The last seconds of the 16 kHz audio streamed to the service, so the speech said right
//! after a "start" can begin the recording that starts only once the command is heard.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::core::domain::AudioBuffer16k;

const SAMPLE_RATE: f32 = AudioBuffer16k::SAMPLE_RATE_HZ as f32;
/// How much audio is held: more than the time from a "start" or "restart" to its report,
/// which waits behind a dictation's own requests at the speech recognition service.
const HELD_SECS: f32 = 10.0;

pub struct HeardAudio {
    audio_start: Instant,
    /// Position in the stream of the first sample held.
    first: usize,
    samples: VecDeque<f32>,
}

impl HeardAudio {
    /// Audio whose first sample was captured at `audio_start`.
    pub fn new(audio_start: Instant) -> Self {
        Self {
            audio_start,
            first: 0,
            samples: VecDeque::new(),
        }
    }

    pub fn push(&mut self, samples: &[f32]) {
        self.samples.extend(samples.iter().copied());
        let excess = self
            .samples
            .len()
            .saturating_sub((HELD_SECS * SAMPLE_RATE) as usize);
        self.samples.drain(..excess);
        self.first += excess;
    }

    /// The samples heard from `since` on, as far as they are still held, and when the first
    /// of them was captured: the audio to send a service that missed it.
    pub fn replay_from(&self, since: Instant) -> (Instant, Vec<f32>) {
        let skip = self.skip_to(since);
        let first = Duration::from_secs_f32((self.first + skip) as f32 / SAMPLE_RATE);
        (
            self.audio_start + first,
            self.samples.range(skip..).copied().collect(),
        )
    }

    fn skip_to(&self, since: Instant) -> usize {
        let position = (since
            .saturating_duration_since(self.audio_start)
            .as_secs_f32()
            * SAMPLE_RATE) as usize;
        position.saturating_sub(self.first).min(self.samples.len())
    }

    /// The samples heard from `since` on, as far as they are still held.
    pub fn since(&self, since: Instant) -> AudioBuffer16k {
        AudioBuffer16k::new(
            self.samples
                .range(self.skip_to(since)..)
                .map(|s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_give_the_audio_heard_since_a_moment() {
        // Given one second heard
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        heard.push(&vec![0.5; SAMPLE_RATE as usize]);
        // When
        let last_quarter = heard.since(start + Duration::from_millis(750));
        // Then
        assert_eq!(last_quarter.duration_ms(), 250);
    }

    #[test]
    fn should_hold_only_the_last_seconds() {
        // Given twelve seconds heard
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        for _ in 0..12 {
            heard.push(&vec![0.0; SAMPLE_RATE as usize]);
        }
        // When asked from the start
        let all = heard.since(start);
        // Then only the held seconds come back
        assert_eq!(all.duration_ms(), 10_000);
    }

    #[test]
    fn should_replay_the_held_audio_from_a_moment_with_the_capture_time_of_its_first_sample() {
        // Given twelve seconds heard, ten of them held
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        for _ in 0..12 {
            heard.push(&vec![0.0; SAMPLE_RATE as usize]);
        }
        // When
        let (first_at, samples) = heard.replay_from(start + Duration::from_secs(9));
        // Then
        assert_eq!(first_at, start + Duration::from_secs(9));
        assert_eq!(samples.len(), 3 * SAMPLE_RATE as usize);
    }

    #[test]
    fn should_replay_from_the_oldest_sample_held_when_asked_for_earlier() {
        // Given twelve seconds heard, ten of them held
        let start = Instant::now();
        let mut heard = HeardAudio::new(start);
        for _ in 0..12 {
            heard.push(&vec![0.0; SAMPLE_RATE as usize]);
        }
        // When
        let (first_at, samples) = heard.replay_from(start);
        // Then
        assert_eq!(first_at, start + Duration::from_secs(2));
        assert_eq!(samples.len(), 10 * SAMPLE_RATE as usize);
    }
}
