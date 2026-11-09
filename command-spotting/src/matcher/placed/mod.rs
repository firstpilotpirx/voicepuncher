//! Commands said without a pause around them: at the end of an utterance, "start" at its
//! start, and doubled or chained commands anywhere. Works on the words of a transcript,
//! so the caller can place what it finds in the audio by the words' times.

use std::ops::Range;

use super::PhraseMatcher;

/// Where in the utterance the command words sat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// The utterance ends with the command.
    Tail,
    /// "start" opens the utterance and speech follows.
    Head,
    /// A doubled short form or a chain of commands, wherever it sits.
    Anywhere,
}

/// Commands found in a transcript: carried out in order; `first..=last` are the indices
/// of their words in the words given, and `spans` those of each command's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub commands: Vec<String>,
    pub spans: Vec<(usize, usize)>,
    pub first: usize,
    pub last: usize,
    pub placement: Placement,
}

use voicepuncher_voice_commands::{
    accepted_after, Form, Mishearing, MishearingKind, DICTATING, FORMS, MISHEARINGS, RESTART,
    STANDBY, START,
};

/// The most commands a chain holds.
const LONGEST_CHAIN: usize = 3;

/// A word of the transcript as the matcher reads it, with its index in the words given.
struct Token {
    at: usize,
    text: String,
}

fn tokens(words: &[&str]) -> Vec<Token> {
    words
        .iter()
        .enumerate()
        .flat_map(|(at, word)| {
            super::words(word)
                .into_iter()
                .map(move |text| Token { at, text })
        })
        .collect()
}

/// How far the words at the start of `tokens` are from `form`: none when they do not say
/// it. A one-word form must be said exactly ("clean" is no "clear"); a longer one may
/// differ a little in each word, as the speech recognition service mishears it.
fn difference_from(tokens: &[Token], form: &Form) -> Option<f32> {
    if tokens.len() < form.words.len() {
        return None;
    }
    let mut total = 0.0;
    for (token, word) in tokens.iter().zip(form.words) {
        let difference = super::difference(&token.text, word);
        let close = if form.words.len() == 1 {
            difference == 0.0
        } else {
            difference <= super::MAX_WORD_DIFFERENCE
        };
        if !close {
            return None;
        }
        total += difference;
    }
    Some(total)
}

/// A command said in the words given, whatever the state accepts: its phrase and the
/// indices of its first and last word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub phrase: &'static str,
    pub first: usize,
    pub last: usize,
}

/// Every command said in `words` in order, whatever the state accepts: what a recognizer's
/// text holds of the commands the utterance reported, once the app switched state.
pub fn commands_said(words: &[&str]) -> Vec<Said> {
    let tokens = tokens(words);
    let mut said = Vec::new();
    let mut from = 0;
    while from < tokens.len() {
        match command_at(&tokens, from) {
            Some((phrase, used)) => {
                said.push(Said {
                    phrase,
                    first: tokens[from].at,
                    last: tokens[from + used - 1].at,
                });
                from += used;
            }
            None => from += 1,
        }
    }
    said
}

/// The command said at `from`, and how many tokens it takes.
fn command_at(tokens: &[Token], from: usize) -> Option<(&'static str, usize)> {
    let rest = tokens.get(from..)?;
    // The closest form said: "restart" is close to "start" too.
    let (form, _) = FORMS
        .iter()
        .filter_map(|form| difference_from(rest, form).map(|d| (form, d)))
        .min_by(|(_, da), (_, db)| da.total_cmp(db))?;
    Some((form.phrase, form.words.len()))
}

impl PhraseMatcher {
    /// The phrases the current state accepts: the selected ones the matcher knows.
    /// Whether it listens as during a dictation: "stop dictation" is selected.
    pub fn listens_while_dictating(&self) -> bool {
        self.selected().iter().any(|s| s == "stop dictation")
    }

    fn accepted(&self) -> Vec<&'static str> {
        STANDBY
            .iter()
            .chain(DICTATING)
            .copied()
            .filter(|phrase| self.selected().iter().any(|s| s == phrase))
            .collect()
    }

    /// The command of the selected set that ends `words`, speech before it or not. Its last
    /// word must be the last word said: "clear the" does not end with "clear".
    pub fn at_end(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let accepted = self.accepted();
        let last_said = words
            .iter()
            .rposition(|w| w.chars().any(char::is_alphabetic))?;
        if tokens.last()?.at != last_said {
            return None;
        }
        (0..tokens.len()).find_map(|from| {
            let (phrase, used) = command_at(&tokens, from)?;
            (from + used == tokens.len() && accepted.contains(&phrase)).then(|| Found {
                commands: vec![phrase.to_owned()],
                spans: vec![(tokens[from].at, tokens[tokens.len() - 1].at)],
                first: tokens[from].at,
                last: tokens[tokens.len() - 1].at,
                placement: Placement::Tail,
            })
        })
    }

    /// "start dictation" or "start" as the only words so far of an utterance still being
    /// spoken: the speech after it has not been heard yet.
    pub fn start_so_far(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let (phrase, used) = command_at(&tokens, 0)?;
        (phrase == START && self.accepted().contains(&START) && used == tokens.len())
            .then(|| start_opening(&tokens, used))
    }

    /// "start dictation" or "start" opening `words`, with more speech after it.
    pub fn start_at_head(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let (phrase, used) = command_at(&tokens, 0)?;
        // "start stop" ends the dictation it would start: no dictation at all.
        let command_follows = command_at(&tokens, used).is_some();
        let speech_follows = used < tokens.len();
        (phrase == START && self.accepted().contains(&START) && speech_follows && !command_follows)
            .then(|| start_opening(&tokens, used))
    }

    /// The first command of the selected set said anywhere in `words`, speech before or
    /// after it: `Tail` when it ends them, `Anywhere` otherwise.
    pub fn said_anywhere(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let accepted = self.accepted();
        (0..tokens.len()).find_map(|from| {
            let (phrase, used) = command_at(&tokens, from)?;
            let last = tokens[from + used - 1].at;
            let ends = from + used == tokens.len();
            accepted.contains(&phrase).then(|| Found {
                commands: vec![phrase.to_owned()],
                spans: vec![(tokens[from].at, last)],
                first: tokens[from].at,
                last,
                placement: if ends {
                    Placement::Tail
                } else {
                    Placement::Anywhere
                },
            })
        })
    }

    /// A doubled short form or a chain of two or three commands anywhere in `words`.
    pub fn anywhere(&self, words: &[&str]) -> Option<Found> {
        let tokens = tokens(words);
        let accepted = self.accepted();
        (0..tokens.len()).find_map(|from| {
            let (first, used) = command_at(&tokens, from)?;
            if !accepted.contains(&first) {
                return None;
            }
            let chain = chain_or_doubled(&tokens, first, from, from + used)?;
            let end = chain[chain.len() - 1].1.end;
            Some(Found {
                commands: chain.iter().map(|(c, _)| (*c).to_owned()).collect(),
                spans: chain
                    .iter()
                    .map(|(_, said)| (tokens[said.start].at, tokens[said.end - 1].at))
                    .collect(),
                first: tokens[from].at,
                last: tokens[end - 1].at,
                placement: Placement::Anywhere,
            })
        })
    }
}

/// The start command said by the first `used` of `tokens`, at the head of its utterance.
fn start_opening(tokens: &[Token], used: usize) -> Found {
    Found {
        commands: vec![START.to_owned()],
        spans: vec![(tokens[0].at, tokens[used - 1].at)],
        first: tokens[0].at,
        last: tokens[used - 1].at,
        placement: Placement::Head,
    }
}

/// The chain of commands from `first`, said in tokens `from..next`; with no chain, `first`
/// said twice in a row as one doubled form, or none.
fn chain_or_doubled(
    tokens: &[Token],
    first: &'static str,
    from: usize,
    next: usize,
) -> Option<Vec<(&'static str, Range<usize>)>> {
    let chain = chain_from(tokens, first, from, next);
    if chain.len() >= 2 {
        return Some(chain);
    }
    let (again, used_again) = command_at(tokens, next)?;
    (again == first).then(|| vec![(first, from..next + used_again)])
}

/// The commands said one after another from `first`, said in tokens `from..next`, each
/// accepted after the one before and each once, "start" only last; with the tokens of each.
fn chain_from(
    tokens: &[Token],
    first: &'static str,
    from: usize,
    mut next: usize,
) -> Vec<(&'static str, Range<usize>)> {
    let mut chain = vec![(first, from..next)];
    while chain.len() < LONGEST_CHAIN {
        let last = chain[chain.len() - 1].0;
        if last == START || last == RESTART {
            break;
        }
        if let Some(row) = after_command_at(tokens, next, last, LONGEST_CHAIN - chain.len()) {
            // Commands said quickly and heard as other words end the chain, one word each.
            chain.extend(
                row.means
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| (c, std::ops::Range { start: next + i, end: next + i + 1 })),
            );
            break;
        }
        let Some((command, used)) = command_at(tokens, next) else {
            break;
        };
        if chain.iter().any(|(c, _)| *c == command) || !accepted_after(last).contains(&command) {
            break;
        }
        chain.push((command, next..next + used));
        next += used;
    }
    chain
}

/// The after-command mishearing that closes the transcript at `next` after `last`, with
/// at most `room` commands.
fn after_command_at(
    tokens: &[Token],
    next: usize,
    last: &str,
    room: usize,
) -> Option<&'static Mishearing> {
    let rest: Vec<&str> = tokens[next.min(tokens.len())..]
        .iter()
        .map(|t| t.text.as_str())
        .collect();
    MISHEARINGS.iter().find(|m| {
        m.kind == MishearingKind::AfterCommand
            && m.heard == rest.as_slice()
            && m.means.len() <= room
            && accepted_after(last).contains(&m.means[0])
    })
}

#[cfg(test)]
mod tests;
