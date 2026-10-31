//! Whether the words a streaming recognizer heard so far say a command to report now.

use crate::confirm::TimedWord;
use crate::matcher::{Found, PhraseMatcher, Placement};

/// What a decision is asked at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Moment {
    /// While the utterance runs, with the audio decoded up to `now` seconds; `pause` is when
    /// the gate's current silence began, none while it hears speech.
    Decoding { now: f32, pause: Option<f32> },
    /// When the utterance ends: nothing more will be said in it.
    Closing,
}

/// Commands to report, said at `start..end` seconds of the stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub commands: Vec<String>,
    pub start: f32,
    pub end: f32,
    pub placement: Placement,
}

/// How long after a one-word command no new word may come before it counts as said alone:
/// "stop" opening "stop the car" is no command. Longer than the recognizer's lag in giving out
/// the next word: "the" of "stop the car" comes out about 0.35 s after "stop" ends.
const SETTLE_SECS: f32 = 0.4;

/// How long after a one-word command no new word may come when the gate's pause began right
/// after it (within `QUIET_AFTER_SECS`): the gate hears no next word, so the recognizer's lag
/// cannot hide one. The gate's pause after a lone "stop" begins about 0.26 s after the word, after
/// "stop the car" 0.66 s after "stop".
const QUIET_SETTLE_SECS: f32 = 0.2;
const QUIET_AFTER_SECS: f32 = 0.35;

/// How far past a command ending a sentence the recognizer decodes before it counts at the
/// gate's pause: the words before it may still change ("let's start" becomes "restart").
const DRAFT_SECS: f32 = 0.2;

/// How soon after a report the same command again is its doubled form ("enter enter"), not
/// a new command: the first word may settle before the recognizer decoded the second.
const DOUBLE_GAP_SECS: f32 = 0.6;

/// Whether `found` repeats `last`, the utterance's previous report, right after it.
pub fn doubles(last: Option<&Finding>, found: &Finding) -> bool {
    last.is_some_and(|last| {
        last.commands == found.commands && found.start - last.end < DOUBLE_GAP_SECS
    })
}

/// The commands to report of `words`, heard so far in the utterance, at `moment`; only the
/// words said after `reported_until`, the end of the last report, count.
pub fn decide(
    matcher: &PhraseMatcher,
    words: &[TimedWord],
    moment: Moment,
    reported_until: f32,
) -> Option<Finding> {
    let fresh: Vec<&TimedWord> = words.iter().filter(|w| w.start >= reported_until).collect();
    let texts: Vec<&str> = fresh.iter().map(|w| w.text.as_str()).collect();
    if let Some(found) = placed(matcher, &texts) {
        return Some(finding(&fresh, &found));
    }
    alone(matcher, &fresh, moment).or_else(|| {
        ended(moment)
            .then(|| matcher.at_end(&texts))
            .flatten()
            .map(|found| finding(&fresh, &found))
            .filter(|found| decoded_past(moment, found.end))
            .filter(|_| moment == Moment::Closing || !ends_a_longer_word(matcher, &texts))
    })
}

/// A command found anywhere in `texts` or opening them, reported as soon as it is heard.
fn placed(matcher: &PhraseMatcher, texts: &[&str]) -> Option<Found> {
    matcher
        .anywhere(texts)
        .or_else(|| {
            matcher
                .starts_early()
                .then(|| matcher.start_so_far(texts))
                .flatten()
        })
        .or_else(|| matcher.start_at_head(texts))
}

/// The command the whole of `fresh` says: at once when its words are the whole phrase
/// ("press enter"), else once settled, so "stop the" of "stop the car" waits for "car".
fn alone(matcher: &PhraseMatcher, fresh: &[&TimedWord], moment: Moment) -> Option<Finding> {
    let (first, last) = (fresh.first()?, fresh.last()?);
    let text: Vec<&str> = fresh.iter().map(|w| w.text.as_str()).collect();
    let phrase = matcher.phrase_in(&text.join(" "))?;
    let settled = is_phrase(&text, &phrase) || settled(moment, last.end);
    settled.then(|| Finding {
        commands: vec![phrase],
        start: first.start,
        end: last.end,
        placement: Placement::Tail,
    })
}

/// Whether a one-word command ending at `end` is said alone at `moment`: the utterance
/// ended, nothing came for a while after it (sooner when the gate's pause began right after
/// it), or the gate heard the pause after it.
fn settled(moment: Moment, end: f32) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, pause } => {
            let quiet_after = pause.is_some_and(|p| p - end <= QUIET_AFTER_SECS);
            now - end >= SETTLE_SECS
                || (quiet_after && now - end >= QUIET_SETTLE_SECS)
                || ended(moment)
        }
    }
}

/// Whether the recognizer decoded `DRAFT_SECS` past `end` at `moment`, or the utterance
/// closed: at a pause the words before a command may still change ("let's start" becomes
/// "restart").
fn decoded_past(moment: Moment, end: f32) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, .. } => now - end >= DRAFT_SECS,
    }
}

/// Whether the last of `texts` may be a draft of a longer command word: then only the
/// utterance's close decides.
fn ends_a_longer_word(matcher: &PhraseMatcher, texts: &[&str]) -> bool {
    texts.last().is_some_and(|w| matcher.ends_a_longer_word(w))
}

/// Whether nothing more is said for now at `moment`: the utterance ended, or the gate heard a
/// pause and the recognizer decoded up to it, so a command ending the words ends a sentence.
fn ended(moment: Moment) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, pause } => pause.is_some_and(|p| now >= p),
    }
}

/// Whether `words` say `phrase` word for word, case and punctuation aside.
fn is_phrase(words: &[&str], phrase: &str) -> bool {
    let bare = |w: &str| {
        w.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };
    let said: Vec<String> = words.iter().map(|w| bare(w).to_lowercase()).collect();
    said.len() > 1 && said.join(" ") == phrase
}

fn finding(fresh: &[&TimedWord], found: &Found) -> Finding {
    Finding {
        commands: found.commands.clone(),
        start: fresh[found.first].start,
        end: fresh[found.last].end,
        placement: found.placement,
    }
}

#[cfg(test)]
mod tests;
