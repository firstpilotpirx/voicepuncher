//! How long a confirmation may decode, following the pace of the recent ones: the budget
//! that cuts speech stuck repeating short on an idle machine would cut ordinary speech on a
//! loaded one, where every confirmation decodes slowly.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// The longest a copy may decode a confirmation on an idle machine: an ordinary one is
/// answered in at most 0.3 s, while speech that gets stuck repeating decodes whisper's whole
/// 219-token limit for 1.7–1.8 s and is never a command. Closing the connection stops the
/// copy's decoding at once (fast-confirmation-of-repeating-russian-speech, design.md).
const FLOOR: Duration = Duration::from_secs(1);
/// How long a confirmation sets the pace: a loaded spell slows a few confirmations in a row
/// among quick ones, and the last four outcomes alone forgot it within a second of quick
/// answers (the voice smoke of 2026-10-22 lost commands cut short at the floor that way).
const WINDOW: Duration = Duration::from_secs(10);
/// The budget beyond the slowest recent confirmation: speech stuck repeating takes several
/// times an ordinary confirmation, a loaded copy only somewhat longer than its last ones.
const SLACK: u32 = 2;

/// The times of the confirmations of the last `window`, and the decoding budget they give
/// the next one.
#[derive(Debug)]
pub struct Pace {
    window: Duration,
    recent: Mutex<VecDeque<(Instant, Duration)>>,
}

impl Default for Pace {
    fn default() -> Self {
        Self::remembering(WINDOW)
    }
}

impl Pace {
    /// A pace set by the confirmations of the last `window`.
    pub fn remembering(window: Duration) -> Self {
        Self { window, recent: Mutex::new(VecDeque::new()) }
    }

    /// The decoding budget of the next confirmation whose deadline is `deadline`: twice the
    /// slowest confirmation of the window, at least `FLOOR`, never past the deadline.
    pub fn budget(&self, deadline: Duration) -> Duration {
        let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        Self::forget_old(&mut recent, self.window);
        let slowest = recent
            .iter()
            .map(|(_, took)| *took)
            .max()
            .unwrap_or(Duration::ZERO);
        (slowest * SLACK).max(FLOOR).min(deadline)
    }

    /// A confirmation took `took`: the time a copy answered in, or the budget a live copy was
    /// cut short at.
    pub fn note(&self, took: Duration) {
        let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        Self::forget_old(&mut recent, self.window);
        recent.push_back((Instant::now(), took));
    }

    fn forget_old(recent: &mut VecDeque<(Instant, Duration)>, window: Duration) {
        let now = Instant::now();
        while recent
            .front()
            .is_some_and(|(noted, _)| now.duration_since(*noted) > window)
        {
            recent.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEADLINE: Duration = Duration::from_secs(3);

    fn millis(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn should_give_the_floor_before_any_confirmation() {
        assert_eq!(Pace::default().budget(DEADLINE), FLOOR);
    }

    #[test]
    fn should_keep_the_floor_while_the_confirmations_come_back_quickly() {
        let pace = Pace::default();
        pace.note(millis(300));
        pace.note(millis(450));
        assert_eq!(pace.budget(DEADLINE), FLOOR);
    }

    #[test]
    fn should_give_twice_the_slowest_recent_confirmation() {
        let pace = Pace::default();
        pace.note(millis(300));
        pace.note(millis(900));
        pace.note(millis(600));
        assert_eq!(pace.budget(DEADLINE), millis(1_800));
    }

    #[test]
    fn should_keep_a_slow_confirmation_through_many_quick_ones_within_the_window() {
        let pace = Pace::default();
        pace.note(millis(1_200));
        for _ in 0..20 {
            pace.note(millis(200));
        }
        assert_eq!(pace.budget(DEADLINE), millis(2_400));
    }

    #[test]
    fn should_forget_a_confirmation_older_than_the_window() {
        let pace = Pace::remembering(millis(50));
        pace.note(millis(1_200));
        std::thread::sleep(millis(80));
        pace.note(millis(300));
        assert_eq!(pace.budget(DEADLINE), FLOOR);
    }

    #[test]
    fn should_never_pass_the_deadline() {
        let pace = Pace::default();
        pace.note(millis(2_000));
        assert_eq!(pace.budget(DEADLINE), DEADLINE);
    }

    #[test]
    fn should_raise_the_budget_after_a_confirmation_cut_short_at_the_floor() {
        let pace = Pace::default();
        pace.note(FLOOR);
        assert_eq!(pace.budget(DEADLINE), FLOOR * 2);
    }
}
