//! The checks of a running utterance asked because the keyword spotter heard "start" or
//! "restart" in it: sooner than the timed checks, once the next word has begun.

use super::samples_in;

/// The phrases whose spotting asks a check of the running utterance.
pub(super) const CHECKED_WHEN_SPOTTED: &[&str] = &["start dictation", "restart dictation"];
/// Audio that follows a spotted phrase before its check is asked: the next word has begun,
/// so "start stop" is heard as such and starts nothing.
const CHECK_AFTER_SECS: f32 = 0.35;

/// Audio between the first check of a spotted "start" and the second, when the first heard
/// "start" alone: the speech after it has begun by then, or it was said on its own.
const SECOND_CHECK_AFTER_SECS: f32 = 0.3;

/// Which check of a spotted phrase it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Round {
    First,
    /// Asked when the first heard "start" alone.
    Second,
}

/// A check to ask once the stream reaches `due`, for a phrase spotted ending at `said_end`
/// in `utterance`; none: the utterance running when it falls due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DueCheck {
    pub said_end: usize,
    pub due: usize,
    pub round: Round,
    pub utterance: Option<u64>,
}

/// The checks spotted phrases ask for, until each is asked.
#[derive(Debug, Default)]
pub(super) struct SpottedChecks {
    waiting: Vec<DueCheck>,
}

impl SpottedChecks {
    /// A phrase spotted ending at sample `said_end`; while a check waits, it covers this
    /// phrase too and keeps its time.
    pub fn spotted(&mut self, said_end: usize) {
        if self.waiting.is_empty() {
            self.waiting.push(DueCheck {
                said_end,
                due: said_end + samples_in(CHECK_AFTER_SECS),
                round: Round::First,
                utterance: None,
            });
        }
    }

    /// The first check of the phrase spotted ending at `said_end` in `utterance` found no
    /// command in audio up to `heard_to`: the second check follows.
    pub fn found_nothing(&mut self, said_end: usize, heard_to: usize, utterance: u64) {
        self.waiting.push(DueCheck {
            said_end,
            due: heard_to + samples_in(SECOND_CHECK_AFTER_SECS),
            round: Round::Second,
            utterance: Some(utterance),
        });
    }

    /// The check to ask at stream position `now`, when one is due; it is asked, or waits its
    /// turn behind another check, with the audio it was due for.
    pub fn take_due(&mut self, now: usize) -> Option<DueCheck> {
        let (index, _) = (self.waiting.iter().enumerate())
            .filter(|(_, check)| now >= check.due)
            .min_by_key(|(_, check)| check.due)?;
        Some(self.waiting.remove(index))
    }
}

#[cfg(test)]
mod tests;
