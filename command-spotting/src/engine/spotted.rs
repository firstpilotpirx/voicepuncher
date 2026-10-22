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

/// The check a spotted phrase asks for, until it is asked.
#[derive(Debug, Default)]
pub(super) struct SpottedChecks {
    waiting: Option<DueCheck>,
}

impl SpottedChecks {
    /// A phrase spotted ending at sample `said_end`; an earlier one not asked yet keeps its
    /// time.
    pub fn spotted(&mut self, said_end: usize) {
        self.waiting.get_or_insert(DueCheck {
            said_end,
            due: said_end + samples_in(CHECK_AFTER_SECS),
            round: Round::First,
            utterance: None,
        });
    }

    /// The first check of the phrase spotted ending at `said_end` in `utterance` found no
    /// command in audio up to `heard_to`: the second check follows.
    pub fn found_nothing(&mut self, said_end: usize, heard_to: usize, utterance: u64) {
        self.waiting = Some(DueCheck {
            said_end,
            due: heard_to + samples_in(SECOND_CHECK_AFTER_SECS),
            round: Round::Second,
            utterance: Some(utterance),
        });
    }

    /// The check to ask at stream position `now`, when one is due; it is asked, or waits its
    /// turn behind another check, with the audio it was due for.
    pub fn take_due(&mut self, now: usize) -> Option<DueCheck> {
        self.waiting.take_if(|check| now >= check.due)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAID_END: usize = 32_000;
    const UTTERANCE: u64 = 3;

    #[test]
    fn should_ask_the_check_once_the_next_word_has_begun() {
        // Given
        let mut checks = SpottedChecks::default();
        checks.spotted(SAID_END);
        let due = SAID_END + samples_in(CHECK_AFTER_SECS);
        // When
        let early = checks.take_due(due - 1);
        let on_time = checks.take_due(due);
        // Then
        assert_eq!(early, None);
        assert_eq!(
            on_time,
            Some(DueCheck { said_end: SAID_END, due, round: Round::First, utterance: None })
        );
        assert_eq!(checks.take_due(due + 1), None);
    }

    #[test]
    fn should_keep_the_time_of_the_first_phrase_spotted() {
        // Given two phrases spotted before the first one's check is due
        let mut checks = SpottedChecks::default();
        checks.spotted(SAID_END);
        checks.spotted(SAID_END + samples_in(0.2));
        // When
        let asked = checks.take_due(SAID_END + samples_in(CHECK_AFTER_SECS));
        // Then
        assert_eq!(asked.map(|c| c.said_end), Some(SAID_END));
    }

    #[test]
    fn should_ask_the_second_check_0_3_s_after_the_audio_of_the_first() {
        // Given the first check found no command in audio up to `heard_to`
        let mut checks = SpottedChecks::default();
        let heard_to = SAID_END + samples_in(CHECK_AFTER_SECS);
        checks.found_nothing(SAID_END, heard_to, UTTERANCE);
        let due = heard_to + samples_in(SECOND_CHECK_AFTER_SECS);
        // When
        let early = checks.take_due(due - 1);
        let on_time = checks.take_due(due);
        // Then
        assert_eq!(early, None);
        assert_eq!(
            on_time.map(|c| (c.round, c.utterance)),
            Some((Round::Second, Some(UTTERANCE)))
        );
    }
}
