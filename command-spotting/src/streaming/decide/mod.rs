//! Whether the words a streaming recognizer heard so far say a command to report now.

use crate::confirm::TimedWord;
use crate::matcher::{Found, PhraseMatcher, Placement};

/// What a decision is asked at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Moment {
    /// While the utterance runs, with the audio decoded up to `now` seconds.
    Decoding { now: f32 },
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
/// "stop" opening "stop the car" is no command.
const SETTLE_SECS: f32 = 0.2;

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
        (moment == Moment::Closing)
            .then(|| matcher.at_end(&texts))
            .flatten()
            .map(|found| finding(&fresh, &found))
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

/// The command the whole of `fresh` says: at once for a form of several words, once settled
/// for a single word.
fn alone(matcher: &PhraseMatcher, fresh: &[&TimedWord], moment: Moment) -> Option<Finding> {
    let (first, last) = (fresh.first()?, fresh.last()?);
    let text: Vec<&str> = fresh.iter().map(|w| w.text.as_str()).collect();
    let phrase = matcher.phrase_in(&text.join(" "))?;
    let settled = match moment {
        Moment::Closing => true,
        Moment::Decoding { now } => fresh.len() > 1 || now - last.end >= SETTLE_SECS,
    };
    settled.then(|| Finding {
        commands: vec![phrase],
        start: first.start,
        end: last.end,
        placement: Placement::Tail,
    })
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
