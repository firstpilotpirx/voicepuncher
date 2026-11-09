//! Whether the words a streaming recognizer heard so far say a command to report now.

use crate::matcher::{commands_said, words, Found, PhraseMatcher, Placement, Said};
use crate::streaming::words::TimedWord;

/// What a decision is asked at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Moment {
    /// While the utterance runs, with the audio decoded up to `now` seconds; `pause` is when
    /// the gate's current silence began, none while it hears speech; `synced` while the app's
    /// sync at `now` is answered: the words before it are said, whatever the gate hears.
    Decoding { now: f32, pause: Option<f32>, synced: bool },
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

/// Whether `found` repeats `last`, the utterance's previous report.
///
/// It does when it holds the same commands right after it, or the end of its command word
/// anywhere in the utterance ("start" heard of a "restart" by the other recognizer, which may
/// time it well after the "restart" ends).
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

/// What the utterance reported so far: the end of its last report, and the commands of every
/// report in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reported {
    pub until: f32,
    pub commands: Vec<String>,
}

/// The commands to report of `words`, heard so far in the utterance, at `moment`.
///
/// Only the words said after what the utterance reported count: after the reported commands
/// where the recognizer's text holds them, and after `reported.until`, the end of the last
/// report.
pub fn decide(
    matcher: &PhraseMatcher,
    words: &[TimedWord],
    moment: Moment,
    reported: &Reported,
) -> Option<Finding> {
    decide_how(matcher, words, moment, reported).map(|(found, _)| found)
}

/// What `decide` finds, and whether only the at-once rule found it.
///
/// The rules that counted a command before (chains, doubled forms, a start opening speech, a
/// command at the pause) go first; a command word anywhere else counts at once (`at_once`),
/// which the session may still doubt (`doubted_by`).
pub fn decide_how(
    matcher: &PhraseMatcher,
    words: &[TimedWord],
    moment: Moment,
    reported: &Reported,
) -> Option<(Finding, bool)> {
    let all: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    let from = after_reported(&commands_said(&all), &reported.commands);
    if let Some(found) = chain_tail(matcher, words, from) {
        return Some((found, false));
    }
    let fresh: Vec<&TimedWord> = words[from..]
        .iter()
        .filter(|w| w.start >= reported.until)
        .collect();
    let texts: Vec<&str> = fresh.iter().map(|w| w.text.as_str()).collect();
    match by_earlier_rules(matcher, &fresh, &texts, moment) {
        Some(found) => Some((found, false)),
        None => at_once(matcher, &fresh, &texts, moment).map(|found| (found, true)),
    }
}

/// A command the rules before the at-once one count among the fresh words: said before a
/// pause, in its set places, alone, or ending the words at the pause.
fn by_earlier_rules(
    matcher: &PhraseMatcher,
    fresh: &[&TimedWord],
    texts: &[&str],
    moment: Moment,
) -> Option<Finding> {
    before_a_pause(matcher, fresh)
        .or_else(|| placed(matcher, texts).map(|found| finding(fresh, &found)))
        .or_else(|| alone(matcher, fresh, moment))
        .or_else(|| ending(matcher, fresh, texts, moment))
}

/// A command said anywhere among the fresh words, reported as soon as the recognizer decoded
/// past it: a word after it, or `SETTLE_SECS` of audio, as a draft of the last word may still
/// grow ("stop" → "stopped"). No pause after it is awaited. A last word that may still grow
/// into a longer command word ("start" of "restart") is left to `alone` and `ending`.
fn at_once(
    matcher: &PhraseMatcher,
    fresh: &[&TimedWord],
    texts: &[&str],
    moment: Moment,
) -> Option<Finding> {
    let found = matcher.said_anywhere(texts)?;
    let found_last = found.last + 1 == fresh.len();
    let finding = finding(fresh, &found);
    if !found_last {
        return Some(finding);
    }
    let grows = moment != Moment::Closing && ends_a_longer_word(matcher, texts);
    (!grows && decoded_past(moment, finding.end)).then_some(finding)
}

/// The index of the first word after the reported commands in `said`, the commands the
/// recognizer's text holds: the longest end of `reported` said in order; 0 when it holds none.
///
/// The commands, never the stamps, tell what is reported: the other recognizer stamps
/// the same words up to a second apart, and the app may have switched state since, so the
/// matcher no longer accepts the chain's first command.
fn after_reported(said: &[Said], reported: &[String]) -> usize {
    (1..=reported.len())
        .rev()
        .find_map(|n| {
            let mut left = reported[reported.len() - n..].iter();
            let mut next = left.next();
            let mut after = 0;
            for command in said {
                if next.is_some_and(|c| c == command.phrase) {
                    after = command.last + 1;
                    next = left.next();
                }
            }
            next.is_none().then_some(after)
        })
        .unwrap_or(0)
}

/// The commands of a chain after its reported ones, the chain matched over every word of
/// the utterance: a recognizer that emits late decodes the chain's last command a decode
/// after the ones before it were reported, and alone among the fresh words it is no command
/// ("start" while dictating counts only after "stop"). The commands whose first word comes
/// before `from` are reported; nothing when none is, or every one.
fn chain_tail(matcher: &PhraseMatcher, words: &[TimedWord], from: usize) -> Option<Finding> {
    let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    let found = matcher.anywhere(&texts)?;
    let reported = found
        .spans
        .iter()
        .take_while(|(first, _)| *first < from)
        .count();
    if reported == 0 || reported == found.commands.len() {
        return None;
    }
    let (first, _) = found.spans[reported];
    Some(Finding {
        commands: found.commands[reported..].to_vec(),
        start: words[first].start,
        end: words[found.last].end,
        placement: Placement::Anywhere,
    })
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
    decide(matcher, &before, Moment::Closing, &Reported::default())
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

/// How far past a word the Russian recognizer decodes before a word it heard over it shows:
/// past that, nothing over the word means it heard nothing there.
const RUSSIAN_WORD_LAG_SECS: f32 = 0.15;

/// Whether `found`, a command the English recognizer heard, waits for the pause.
///
/// `russian` holds the Russian recognizer's words, decoded up to `decoded_to` in the stream:
/// the English one hears Russian speech as English words ("чтобы" → "stop"). A command the
/// Russian recognizer heard over the span counts at once; one where it heard other words, or
/// has not decoded far enough past the span to tell (no later word, the utterance not closed),
/// counts only when it ends the words and the gate's pause began right after it, as before
/// commands counted at once.
pub fn doubted_by(russian: &[TimedWord], decoded_to: f32, found: &Finding, moment: Moment) -> bool {
    let over: Vec<&str> = russian
        .iter()
        .filter(|w| w.end >= found.start && w.start <= found.end)
        .map(|w| w.text.as_str())
        .collect();
    if !commands_said(&over).is_empty() {
        return false;
    }
    let unsure = !over.is_empty() || !russian_past(russian, decoded_to, found.end, moment);
    unsure && (found.placement != Placement::Tail || !settled(moment, found.end))
}

/// Whether the Russian recognizer decoded far enough past `end` to have shown a word it heard
/// there: a later word, a decode `RUSSIAN_WORD_LAG_SECS` past it, or the utterance closed.
fn russian_past(russian: &[TimedWord], decoded_to: f32, end: f32, moment: Moment) -> bool {
    moment == Moment::Closing
        || russian.iter().any(|w| w.start >= end)
        || decoded_to >= end + RUSSIAN_WORD_LAG_SECS
}

/// Whether a one-word command ending at `end` is said alone at `moment`: the utterance
/// ended, the recognizer decoded up to the gate's pause, or the gate's pause began right after
/// the word and nothing came for a while after it. While the gate still hears speech the word
/// is not alone yet, however much is decoded past it: the next word may come out late. At the
/// app's sync it is, once a while is decoded past it: the sync stands for the pause.
fn settled(moment: Moment, end: f32) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, pause, synced } => {
            let paused_after = pause.is_some_and(|p| p - end <= SPEECH_AFTER_SECS);
            ((paused_after || synced) && now - end >= SETTLE_SECS) || ended(moment)
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
/// A sync is no pause here: the app asks for one inside a sentence too.
fn ended(moment: Moment) -> bool {
    match moment {
        Moment::Closing => true,
        Moment::Decoding { now, pause, .. } => pause.is_some_and(|p| now >= p),
    }
}

/// `found` as a finding: a doubled one-word form ends with its first word, as a recognizer
/// that emits late times the second word inside the speech said after it, and the dictation
/// a doubled start begins would lose its first word; the app removes the second word from
/// the text.
fn finding(fresh: &[&TimedWord], found: &Found) -> Finding {
    // WHY: the recognizers write "Restart, restart": the words are compared as the matcher
    // reads them, lowercase letters only.
    let doubled_one_word = found.last == found.first + 1
        && words(&fresh[found.first].text) == words(&fresh[found.last].text);
    let last = if doubled_one_word {
        found.first
    } else {
        found.last
    };
    Finding {
        commands: found.commands.clone(),
        start: fresh[found.first].start,
        end: fresh[last].end,
        placement: found.placement,
    }
}

#[cfg(test)]
mod tests;
