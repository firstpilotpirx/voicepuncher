//! Whether the words a streaming recognizer heard so far say a command to report now.

use crate::matcher::{Found, PhraseMatcher, Placement};
use crate::streaming::words::TimedWord;

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
/// "stop" opening "stop the car" is no command. Sooner when the gate hears a pause after it and
/// the recognizer decoded up to that pause.
const SETTLE_SECS: f32 = 0.2;

/// When the gate's pause begins later than this after the word, it heard speech go on after it,
/// and the word settles only once decoded up to the pause: the next word may come out of the
/// recognizer well after the command ends, so "stop" of "stop the car" would settle before "the"
/// shows. The gate's pause after a lone "stop" begins about 0.26 s after the word, after
/// "stop the car" 0.66 s after "stop".
const SPEECH_AFTER_SECS: f32 = 0.35;

/// A silence between two words this long is a pause the gate missed: the words before it are
/// said alone. As long as a doubled command's gap.
const WORD_PAUSE_SECS: f32 = DOUBLE_GAP_SECS;

/// How soon after a report the same command again is its doubled form ("enter enter"), not
/// a new command: the first word may settle before the recognizer decoded the second.
const DOUBLE_GAP_SECS: f32 = 0.6;

/// Whether `found` repeats `last`, the utterance's previous report: the same commands right
/// after it, or the end of its command word anywhere in the utterance ("start" heard of a
/// "restart" by the other recognizer, which may time it well after the "restart" ends).
pub fn doubles(last: Option<&Finding>, found: &Finding) -> bool {
    last.is_some_and(|last| {
        ends_its_word(last, found)
            || (last.commands == found.commands && found.start - last.end < DOUBLE_GAP_SECS)
    })
}

/// Whether the first word of `found`'s command ends the longer first word of `last`'s.
fn ends_its_word(last: &Finding, found: &Finding) -> bool {
    let first = |f: &Finding| -> Option<String> {
        let phrase = f.commands.first()?;
        phrase.split_whitespace().next().map(str::to_owned)
    };
    match (first(last), first(found)) {
        (Some(long), Some(short)) => long.len() > short.len() && long.ends_with(&short),
        _ => false,
    }
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
    if let Some(found) = before_a_pause(matcher, &fresh) {
        return Some(found);
    }
    if let Some(found) = placed(matcher, &texts) {
        return Some(finding(&fresh, &found));
    }
    alone(matcher, &fresh, moment).or_else(|| ending(matcher, &fresh, &texts, moment))
}

/// A command ending the words heard, once nothing more is said for now.
fn ending(
    matcher: &PhraseMatcher,
    fresh: &[&TimedWord],
    texts: &[&str],
    moment: Moment,
) -> Option<Finding> {
    ended(moment)
        .then(|| matcher.at_end(texts))
        .flatten()
        .map(|found| finding(fresh, &found))
        .filter(|found| decoded_past(moment, found.end))
        .filter(|_| moment == Moment::Closing || !ends_a_longer_word(matcher, texts))
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

/// The command the words of `fresh` before a pause between them say, as if the utterance
/// closed there: the gate misses a pause in a noisy room, and "press enter … press enter" is
/// two commands.
fn before_a_pause(matcher: &PhraseMatcher, fresh: &[&TimedWord]) -> Option<Finding> {
    let cut = fresh
        .windows(2)
        .position(|pair| pair[1].start - pair[0].end >= WORD_PAUSE_SECS)?;
    let before: Vec<TimedWord> = fresh[..=cut].iter().map(|w| (*w).clone()).collect();
    decide(matcher, &before, Moment::Closing, 0.0)
}

/// The command the whole of `fresh` says once settled, so "stop the" of "stop the car" waits
/// for "car".
fn alone(matcher: &PhraseMatcher, fresh: &[&TimedWord], moment: Moment) -> Option<Finding> {
    let (first, last) = (fresh.first()?, fresh.last()?);
    let text: Vec<&str> = fresh.iter().map(|w| w.text.as_str()).collect();
    let phrase = matcher.phrase_in(&text.join(" "))?;
    settled(moment, last.end).then(|| Finding {
        commands: vec![phrase],
        start: first.start,
        end: last.end,
        placement: Placement::Tail,
    })
}

/// Whether a one-word command ending at `end` is said alone at `moment`: the utterance
/// ended, the gate heard the pause after it, or nothing came for a while after it unless the
/// gate heard speech go on after it.
fn settled(moment: Moment, end: f32) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, pause } => {
            let speech_after = pause.is_some_and(|p| p - end > SPEECH_AFTER_SECS);
            (!speech_after && now - end >= SETTLE_SECS) || ended(moment)
        }
    }
}

/// Whether the recognizer decoded `SETTLE_SECS` past `end` at `moment`, or the utterance
/// closed: at a pause the words before a command may still change ("let's start" becomes
/// "restart").
fn decoded_past(moment: Moment, end: f32) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, .. } => now - end >= SETTLE_SECS,
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
