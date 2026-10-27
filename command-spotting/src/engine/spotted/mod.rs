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

/// Two hits of one phrase at most this far apart are its doubled form: said on purpose, it
/// is asked about at once, without waiting for the pause after it or the next word.
const DOUBLED_WITHIN_SECS: f32 = 0.8;

/// Audio that follows a short "stop" or "cancel" before the check of its doubled form is
/// asked: the keyword spotter often hears the doubled form once, before its last sound.
const DOUBLED_CHECK_AFTER_SECS: f32 = 0.15;

/// Which check of a spotted phrase it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    First,
    /// Asked when the first heard "start" alone.
    Second,
    /// Asked at once because one phrase was spotted twice in a row: any doubled command counts.
    Doubled,
    /// Asked soon after a short "stop" or "cancel": only a doubled command counts, a chain is
    /// left to the checks of the utterance.
    Short,
}

/// A check to ask once the stream reaches `due`, for a phrase spotted ending at `said_end`
/// in `utterance`; none: the utterance running when it falls due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueCheck {
    pub said_end: usize,
    pub due: usize,
    pub round: Round,
    pub utterance: Option<u64>,
}

/// The checks spotted phrases ask for, until each is asked.
#[derive(Debug, Default)]
pub struct SpottedChecks {
    waiting: Vec<DueCheck>,
    /// The last phrase spotted and where it ended.
    last_hit: Option<(String, usize)>,
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

    /// A short "stop" or "cancel" spotted ending at sample `said_end`: whether it was said
    /// doubled is asked soon.
    pub fn spotted_short(&mut self, said_end: usize) {
        if self.waiting.iter().any(|check| check.round == Round::Short) {
            return;
        }
        self.waiting.push(DueCheck {
            said_end,
            due: said_end + samples_in(DOUBLED_CHECK_AFTER_SECS),
            round: Round::Short,
            utterance: None,
        });
    }

    /// Any phrase spotted ending at sample `said_end`; the second hit of one phrase within
    /// `DOUBLED_WITHIN_SECS` asks a doubled check at once, and is told as doubled.
    pub fn hit(&mut self, phrase: &str, said_end: usize) -> bool {
        let doubled = self.last_hit.as_ref().is_some_and(|(last, end)| {
            last == phrase && said_end.saturating_sub(*end) <= samples_in(DOUBLED_WITHIN_SECS)
        });
        if doubled {
            self.last_hit = None;
            self.waiting.push(DueCheck {
                said_end,
                due: said_end,
                round: Round::Doubled,
                utterance: None,
            });
        } else {
            self.last_hit = Some((phrase.to_owned(), said_end));
        }
        doubled
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
