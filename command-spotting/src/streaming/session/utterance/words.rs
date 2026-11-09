//! The words a recognizer heard of the utterance, and how a finding sits among them.

use crate::matcher::PhraseMatcher;
use crate::protocol::ServerEvent;
use crate::streaming::words::TimedWord;

use super::super::super::decide::{decide_how, doubted_by, Finding, Moment};
use super::super::super::ports::Decoded;
use super::super::super::words::timed_words;
use super::{Utterance, OPENING_SLACK_SECS};

impl Utterance {
    /// What recognizer `r` decided of its last words at `moment`: the report it makes at `now`.
    pub(super) fn decided(
        &mut self,
        matcher: &PhraseMatcher,
        r: usize,
        moment: Moment,
        now: f32,
    ) -> Option<ServerEvent> {
        let words = self.words(r, &self.last[r]);
        let (found, at_once) = decide_how(matcher, &words, moment, &self.reported())?;
        // WHY: Russian dictation is what the English recognizer mishears as a command word.
        if at_once && matcher.listens_while_dictating() && self.doubted(r, &found, moment) {
            return None;
        }
        if self
            .reports
            .handover
            .is_some_and(|from| found.start >= from)
        {
            return None;
        }
        let found = self.started_at_onset(found, &words);
        self.report(r, found, now)
    }

    /// The English words decided again once respelled recognizer `r` decoded at `moment`.
    ///
    /// A doubt lifts at the Russian decode that settles it, not at the next English one: the
    /// two arrive in either order, and the real-time and fast paces must agree.
    pub(super) fn recheck_doubted(
        &mut self,
        matcher: &PhraseMatcher,
        r: usize,
        moment: Moment,
        now: f32,
    ) -> Option<ServerEvent> {
        if !self.respelled[r] || !matcher.listens_while_dictating() {
            return None;
        }
        for e in 0..self.last.len() {
            // WHY: the English words count as closed only once the English recognizer closed.
            if self.respelled[e] || (moment == Moment::Closing && !self.decoders[e].finished()) {
                continue;
            }
            if let Some(event) = self.decided(matcher, e, moment, now) {
                return Some(event);
            }
        }
        None
    }

    /// Whether `found`, one command recognizer `r` heard, waits for the pause because of the
    /// Russian recognizer's words over its span, when `r` is the English one (`doubted_by`).
    pub(super) fn doubted(&self, r: usize, found: &Finding, moment: Moment) -> bool {
        if self.respelled[r] || found.commands.len() != 1 || self.said_long(r, found) {
            return false;
        }
        (0..self.last.len())
            .filter(|&other| self.respelled[other])
            .any(|ru| {
                doubted_by(
                    &self.words(ru, &self.last[ru]),
                    found,
                    self.as_seen_by(ru, moment),
                )
            })
    }

    /// `moment` as respelled recognizer `ru` saw it: the utterance closed only once `ru` closed
    /// too, as it may still be decoding the words over the command; until then, at the pause.
    fn as_seen_by(&self, ru: usize, moment: Moment) -> Moment {
        if moment != Moment::Closing || self.decoders[ru].finished() {
            return moment;
        }
        Moment::Decoding { now: f32::INFINITY, pause: Some(self.trace.speech_end), synced: false }
    }

    /// Whether recognizer `r` heard the rest of `found`'s long form right after its command
    /// word ("dictation" after "stop"): no Russian word is heard as a whole English phrase.
    fn said_long(&self, r: usize, found: &Finding) -> bool {
        let Some(phrase) = found.commands.first() else {
            return false;
        };
        let rest: Vec<String> = phrase
            .split_whitespace()
            .skip(1)
            .map(str::to_owned)
            .collect();
        let after: Vec<String> = self
            .words(r, &self.last[r])
            .into_iter()
            .filter(|w| w.start >= found.end)
            .map(|w| w.text.to_lowercase())
            .take(rest.len())
            .collect();
        !rest.is_empty() && after == rest
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
