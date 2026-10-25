//! When the worker tries the command spotting service again after it went away, and from
//! which moment it sends the new connection the audio the service missed.

use std::time::{Duration, Instant};

/// Pauses between the attempts to reach the service again: growing, then the last one for
/// as long as listening is on, so the service is back within 5 s of answering again.
const RECONNECT_PAUSES: [Duration; 5] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
/// How far before the loss the replay reaches back: the service had not answered for up to
/// this long when the client gave up on it, so it may not have spotted that audio.
const REPLAY_BEFORE_LOSS: Duration = Duration::from_secs(5);
/// Failed attempts are logged at the first, then at every this many.
const ATTEMPTS_LOGGED_EVERY: u32 = 10;

/// How the worker paces its attempts and how far back it replays. The app runs the default;
/// a test runs them faster to prove a long outage without waiting it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkTimings {
    reconnect_pauses: [Duration; 5],
    replay_before_loss: Duration,
}

impl Default for LinkTimings {
    fn default() -> Self {
        Self { reconnect_pauses: RECONNECT_PAUSES, replay_before_loss: REPLAY_BEFORE_LOSS }
    }
}

impl LinkTimings {
    /// The same timings, `speedup` times shorter.
    pub fn faster(self, speedup: u32) -> Self {
        let speedup = speedup.max(1);
        Self {
            reconnect_pauses: self.reconnect_pauses.map(|pause| pause / speedup),
            replay_before_loss: self.replay_before_loss / speedup,
        }
    }

    /// The pause before attempt `attempt + 1`, counting the failed ones from 0.
    pub fn reconnect_pause(&self, failed_attempts: u32) -> Duration {
        let last = self.reconnect_pauses.len() - 1;
        self.reconnect_pauses[(failed_attempts as usize).min(last)]
    }

    /// Where the replay to the new connection starts: some time before the loss, never at or
    /// before the end of the last command the app was told of, so no command comes twice.
    pub fn replay_start(&self, lost_at: Instant, last_command_end: Option<Instant>) -> Instant {
        let before_loss = lost_at
            .checked_sub(self.replay_before_loss)
            .unwrap_or(lost_at);
        last_command_end.map_or(before_loss, |end| end.max(before_loss))
    }

    /// An outage is long from when the replay no longer reaches back to the loss: from then
    /// on the return no longer hides it.
    fn long_outage(&self) -> Duration {
        self.replay_before_loss
    }
}

/// A failed attempt is worth a log line: the first, then every tenth.
pub fn attempt_logged(failed_attempts: u32) -> bool {
    failed_attempts == 1 || failed_attempts % ATTEMPTS_LOGGED_EVERY == 0
}

/// The service is away: since when, and when to try it again.
pub struct Outage {
    timings: LinkTimings,
    pub lost_at: Instant,
    pub failed_attempts: u32,
    pub next_try: Instant,
    /// Whether the outage was already reported long.
    long: bool,
}

impl Outage {
    pub fn since(lost_at: Instant, timings: LinkTimings) -> Self {
        Self {
            timings,
            lost_at,
            failed_attempts: 0,
            next_try: lost_at + timings.reconnect_pause(0),
            long: false,
        }
    }

    pub fn due(&self, now: Instant) -> bool {
        now >= self.next_try
    }

    /// True once per outage: at the first `now` at which it has lasted long.
    pub fn became_long(&mut self, now: Instant) -> bool {
        let became = !self.long && now >= self.lost_at + self.timings.long_outage();
        self.long |= became;
        became
    }

    /// Counts a failed attempt at `now` and returns the pause before the next one.
    pub fn failed(&mut self, now: Instant) -> Duration {
        self.failed_attempts += 1;
        let pause = self.timings.reconnect_pause(self.failed_attempts);
        self.next_try = now + pause;
        pause
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_grow_the_pause_between_attempts_up_to_four_seconds() {
        let pauses: Vec<u64> = (0..7)
            .map(|n| u64::try_from(LinkTimings::default().reconnect_pause(n).as_millis()).unwrap())
            .collect();
        assert_eq!(pauses, [250, 500, 1_000, 2_000, 4_000, 4_000, 4_000]);
    }

    #[test]
    fn should_replay_from_five_seconds_before_the_loss() {
        // Given
        let lost_at = Instant::now() + Duration::from_secs(60);
        // When
        let start = LinkTimings::default().replay_start(lost_at, None);
        // Then
        assert_eq!(start, lost_at.checked_sub(Duration::from_secs(5)).unwrap());
    }

    #[test]
    fn should_not_replay_audio_of_a_command_already_reported() {
        // Given
        let lost_at = Instant::now() + Duration::from_secs(60);
        let command_end = lost_at.checked_sub(Duration::from_secs(2)).unwrap();
        // When
        let start = LinkTimings::default().replay_start(lost_at, Some(command_end));
        // Then
        assert_eq!(start, command_end);
    }

    #[test]
    fn should_shorten_every_pause_and_the_replay_when_faster() {
        // Given
        let timings = LinkTimings::default().faster(10);
        // When
        let pauses: Vec<u64> = (0..5)
            .map(|n| u64::try_from(timings.reconnect_pause(n).as_millis()).unwrap())
            .collect();
        let lost_at = Instant::now() + Duration::from_secs(60);
        // Then
        assert_eq!(pauses, [25, 50, 100, 200, 400]);
        assert_eq!(
            timings.replay_start(lost_at, None),
            lost_at.checked_sub(Duration::from_millis(500)).unwrap()
        );
    }

    #[test]
    fn should_log_the_first_and_every_tenth_failed_attempt() {
        let logged: Vec<u32> = (1..=30).filter(|&n| attempt_logged(n)).collect();
        assert_eq!(logged, [1, 10, 20, 30]);
    }

    #[test]
    fn should_try_again_after_the_pause_of_each_failed_attempt() {
        // Given
        let lost_at = Instant::now();
        let mut outage = Outage::since(lost_at, LinkTimings::default());
        // When
        let pause = outage.failed(lost_at + Duration::from_secs(1));
        // Then
        assert_eq!(pause, Duration::from_millis(500));
        assert!(!outage.due(lost_at + Duration::from_millis(1_400)));
        assert!(outage.due(lost_at + Duration::from_millis(1_500)));
    }

    #[test]
    fn should_become_long_once_when_the_replay_no_longer_reaches_back_to_the_loss() {
        // Given
        let lost_at = Instant::now();
        let mut outage = Outage::since(lost_at, LinkTimings::default());
        // When
        let before = outage.became_long(lost_at + Duration::from_millis(4_990));
        let at = outage.became_long(lost_at + Duration::from_secs(5));
        let later = outage.became_long(lost_at + Duration::from_secs(9));
        // Then
        assert_eq!((before, at, later), (false, true, false));
    }
}
