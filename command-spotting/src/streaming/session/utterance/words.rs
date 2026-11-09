//! The words a recognizer heard of the utterance, and how a finding sits among them.

use crate::streaming::words::TimedWord;

use super::super::super::decide::{doubted_by, Finding, Moment};
use super::super::super::ports::Decoded;
use super::super::super::words::timed_words;
use super::{Utterance, OPENING_SLACK_SECS};

impl Utterance {
    /// Whether `found`, one command recognizer `r` heard, waits for the pause because of the
    /// Russian recognizer's words over its span, when `r` is the English one (`doubted_by`).
    pub(super) fn doubted(&self, r: usize, found: &Finding, moment: Moment) -> bool {
        if self.respelled[r] || found.commands.len() != 1 {
            return false;
        }
        (0..self.last.len())
            .filter(|&other| self.respelled[other])
            .any(|ru| doubted_by(&self.words(ru, &self.last[ru]), found, moment))
    }

    /// `found` starting at the gate's onset when it opens the utterance: a recognizer times a
    /// first word later than it is said, and the dictation it ends would keep its first sounds.
    pub(super) fn started_at_onset(&self, mut found: Finding, words: &[TimedWord]) -> Finding {
        let onset = self.trace.onset;
        let opens = words.first().is_some_and(|w| w.start >= found.start);
        if opens && found.start - onset <= OPENING_SLACK_SECS {
            found.start = found.start.min(onset);
        }
        found
    }

    pub(super) fn words(&self, r: usize, decoded: &Decoded) -> Vec<TimedWord> {
        let mut words = timed_words(
            &decoded.tokens,
            &decoded.times,
            self.start,
            self.respelled[r],
        );
        for word in &mut words {
            word.start = self.unflushed(word.start);
            word.end = self.unflushed(word.end);
        }
        words
    }
}
