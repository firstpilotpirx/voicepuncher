//! How fast a scenario feeds the room between its utterances.
//!
//! A scenario under timing (`@cues`) runs in real time, every other ten times faster, so the check waits
//! on the app, not on the clock. Speech is fed at once either way.

use std::time::Duration;

/// How many times faster than real time the fast pace feeds the room.
const FAST_FACTOR: u32 = 10;

/// The tag of the scenarios timed on an idle machine, which keep real time.
const TIMED_TAG: &str = "cues";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pace {
    RealTime,
    #[default]
    Fast,
}

impl Pace {
    /// Real time for a scenario whose merged tags hold `@cues`, fast for every other.
    pub fn of_tags<'a>(tags: impl IntoIterator<Item = &'a str>) -> Self {
        if tags.into_iter().any(|tag| tag == TIMED_TAG) {
            Self::RealTime
        } else {
            Self::Fast
        }
    }

    /// How long to wait between two frames of `frame` of audio.
    pub fn frame_interval(self, frame: Duration) -> Duration {
        match self {
            Self::RealTime => frame,
            Self::Fast => frame / FAST_FACTOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_real_time_only_under_timing() {
        assert_eq!(Pace::of_tags(["serial", "cues"]), Pace::RealTime);
        assert_eq!(Pace::of_tags(["smoke", "voice_control"]), Pace::Fast);
        assert_eq!(Pace::of_tags([]), Pace::Fast);
    }

    #[test]
    fn should_feed_a_frame_ten_times_sooner_at_the_fast_pace() {
        let frame = Duration::from_millis(100);
        assert_eq!(Pace::RealTime.frame_interval(frame), frame);
        assert_eq!(Pace::Fast.frame_interval(frame), Duration::from_millis(10));
    }
}
