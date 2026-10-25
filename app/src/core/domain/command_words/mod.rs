//! Command words that reached a dictated text.
//!
//! Said right before "stop" without a pause, the recording is cut at the command, but a cut a word
//! late leaves the command in the transcript; and after a "start" or "restart" said in one breath
//! with the dictation, the command opens it.

use super::voice_command::VoiceCommand;

/// The ways a command reaches a transcript: English, or written in Cyrillic when the
/// speech recognition service took the dictation for Russian. Longer ways first.
pub const COMMAND_FORMS: &[&[&str]] = &[
    &["clear", "all", "text"],
    &["stop", "dictation"],
    &["cancel", "dictation"],
    &["restart", "dictation"],
    &["start", "dictation"],
    &["press", "enter"],
    &["clear", "text"],
    &["stop"],
    &["cancel"],
    &["restart"],
    &["start"],
    &["enter"],
    &["clear"],
    &["стоп"],
    &["старт"],
    &["энтер"],
    &["клир"],
    &["кенсел"],
    &["кэнсел"],
];
const START_FORMS: &[&[&str]] = &[&["start", "dictation"], &["start"], &["старт"]];
const RESTART_FORMS: &[&[&str]] = &[&["restart", "dictation"], &["restart"]];
const POLITE_WORDS: &[&str] = &["please", "пожалуйста"];
/// What may sit between the kept text and the removed command.
const SEPARATORS: &[char] = &[',', ';', ':', '-', '—'];
const SENTENCE_ENDS: &[char] = &['.', '!', '?'];

/// A word of `text`: where it starts, and its lowercase letters.
struct Word {
    start: usize,
    text: String,
}

fn words(text: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut current: Option<usize> = None;
    for (at, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (current, c.is_alphabetic()) {
            (None, true) => current = Some(at),
            (Some(start), false) => {
                words.push(Word { start, text: text[start..at].to_lowercase() });
                current = None;
            }
            _ => {}
        }
    }
    words
}

fn says(words: &[Word], form: &[&str]) -> bool {
    words.len() == form.len() && words.iter().zip(form).all(|(w, f)| w.text == *f)
}

/// Removes command words that end `text` ("…on Friday. Stop." → "…on Friday."): one
/// command, a doubled one or a chain, each maybe with "please".
pub fn without_trailing_command(text: &str) -> String {
    let words = words(text);
    let kept = trailing_commands_start(&words);
    if kept == words.len() {
        return text.to_owned();
    }
    let cut = words[kept].start;
    let head = text[..cut]
        .trim_end()
        .trim_end_matches(SEPARATORS)
        .trim_end();
    let ended = text
        .trim_end()
        .chars()
        .last()
        .filter(|c| SENTENCE_ENDS.contains(c));
    match ended {
        Some(end) if !head.ends_with(SENTENCE_ENDS) && !head.is_empty() => format!("{head}{end}"),
        _ => head.to_owned(),
    }
}

/// Where the command words that end `words` begin: past the last word when none do.
fn trailing_commands_start(words: &[Word]) -> usize {
    let mut kept = words.len();
    loop {
        let mut end = kept;
        if end > 0 && POLITE_WORDS.contains(&words[end - 1].text.as_str()) {
            end -= 1;
        }
        let Some(form) = COMMAND_FORMS
            .iter()
            .find(|form| end >= form.len() && says(&words[end - form.len()..end], form))
        else {
            return kept;
        };
        kept = end - form.len();
    }
}

/// Removes the words of `command`, the command that started a dictation, that open `text`
/// when more text follows.
///
/// "start", a doubled "start start" or "start dictation", and the same for "restart", each maybe
/// with "please"; or "please" alone, when the recording began after the command.
pub fn without_leading_command(command: VoiceCommand, text: &str) -> String {
    let forms = match command {
        VoiceCommand::StartDictation => START_FORMS,
        VoiceCommand::RestartDictation => RESTART_FORMS,
        _ => return text.to_owned(),
    };
    let words = words(text);
    let opening = opening_words(forms, &words);
    if opening == 0 {
        return text.to_owned();
    }
    capitalized(&text[words[opening].start..])
}

/// How many of `words` say the command in one of `forms`, a doubled single word and a closing
/// "please" included; a "please" alone counts too, when the recording began after the command.
fn opening_words(forms: &[&[&str]], words: &[Word]) -> usize {
    let opening = command_words(forms, words);
    if words.len() > opening + 1 && POLITE_WORDS.contains(&words[opening].text.as_str()) {
        return opening + 1;
    }
    opening
}

/// How many of `words` say the command in one of `forms`, a doubled single word included.
fn command_words(forms: &[&[&str]], words: &[Word]) -> usize {
    match forms
        .iter()
        .find(|form| words.len() > form.len() && says(&words[..form.len()], form))
    {
        Some(form) if form.len() == 1 && words.len() > 2 && words[1].text == words[0].text => 2,
        Some(form) => form.len(),
        None => 0,
    }
}

/// `text` with its first letter upper case.
pub(crate) fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
