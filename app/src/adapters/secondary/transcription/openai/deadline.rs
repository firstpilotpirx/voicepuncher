//! How long a transcription request may take, and how often it is tried again.

use std::time::Duration;

use crate::core::domain::AudioBuffer16k;

/// Pauses before each new attempt when the service refuses or drops the connection
/// before it answers (a busy service resets connections it has no room for).
pub(super) const RETRY_PAUSES: [Duration; 3] = [
    Duration::from_millis(200),
    Duration::from_millis(400),
    Duration::from_millis(800),
];
/// New attempts after a request the service did not answer in time or answered with an
/// error; each goes to another copy than the one still working on the abandoned one.
pub(super) const SLOW_RETRIES: u32 = 2;
pub(super) const SLOW_RETRY_PAUSE: Duration = Duration::from_millis(200);
/// The wait for one request: this, plus half the length of its audio.
const DEADLINE_BASE: Duration = Duration::from_secs(5);
const DEADLINE_PER_AUDIO_SECOND: f32 = 0.5;

/// The longest one recording or chunk may take with every retry, each attempt at its
/// longest deadline `timeout`: what a stopped dictation may wait for its last chunk.
pub fn longest_wait(timeout: Duration) -> Duration {
    timeout * (1 + SLOW_RETRIES) + SLOW_RETRY_PAUSE * SLOW_RETRIES + RETRY_PAUSES.iter().sum()
}

/// How long one request may take: a few seconds for a short recording, more for long
/// audio, never more than `timeout`; or `fixed`, whatever the audio, when a test client
/// set one.
pub(super) fn request_deadline(
    audio: &AudioBuffer16k,
    timeout: Duration,
    fixed: Option<Duration>,
) -> Duration {
    if let Some(fixed) = fixed {
        return fixed;
    }
    let audio = Duration::from_millis(audio.duration_ms());
    (DEADLINE_BASE + audio.mul_f32(DEADLINE_PER_AUDIO_SECOND)).min(timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_wait_a_few_seconds_for_a_short_recording() {
        let five_seconds = AudioBuffer16k::new(vec![0; 5 * 16_000]);
        assert_eq!(
            request_deadline(&five_seconds, Duration::from_secs(60), None),
            Duration::from_millis(7_500)
        );
    }

    #[test]
    fn should_wait_longer_for_a_long_chunk() {
        let chunk = AudioBuffer16k::new(vec![0; 32 * 16_000]);
        assert_eq!(
            request_deadline(&chunk, Duration::from_secs(60), None),
            Duration::from_secs(21)
        );
    }

    #[test]
    fn should_wait_every_attempt_and_pause_at_the_longest() {
        assert_eq!(
            longest_wait(Duration::from_secs(60)),
            Duration::from_millis(181_800)
        );
    }

    #[test]
    fn should_never_wait_longer_than_the_configured_timeout() {
        let chunk = AudioBuffer16k::new(vec![0; 32 * 16_000]);
        assert_eq!(
            request_deadline(&chunk, Duration::from_secs(10), None),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn should_take_the_fixed_deadline_for_any_audio_length() {
        let fixed = Some(Duration::from_secs(30));
        for seconds in [1, 32] {
            let audio = AudioBuffer16k::new(vec![0; seconds * 16_000]);
            assert_eq!(
                request_deadline(&audio, Duration::from_secs(10), fixed),
                Duration::from_secs(30)
            );
        }
    }
}
