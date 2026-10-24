//! Command words that reached a dictated text: said right before "stop" without a pause,
//! the recording is cut at the command, but a cut a word late leaves the command in the
//! transcript; and after a "start" or "restart" said in one breath with the dictation, the
//! command opens it.

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
            break;
        };
        kept = end - form.len();
    }
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

/// Removes the words of `command`, the command that started a dictation, that open `text`
/// when more text follows: "start", a doubled "start start" or "start dictation", and the
/// same for "restart", each maybe with "please"; or "please" alone, when the recording
/// began after the command.
pub fn without_leading_command(command: VoiceCommand, text: &str) -> String {
    let forms = match command {
        VoiceCommand::StartDictation => START_FORMS,
        VoiceCommand::RestartDictation => RESTART_FORMS,
        _ => return text.to_owned(),
    };
    let words = words(text);
    let mut opening = match forms
        .iter()
        .find(|form| words.len() > form.len() && says(&words[..form.len()], form))
    {
        Some(form) if form.len() == 1 && words.len() > 2 && words[1].text == words[0].text => 2,
        Some(form) => form.len(),
        None => 0,
    };
    // A "please" closing the command, also when the recording began after the command.
    if words.len() > opening + 1 && POLITE_WORDS.contains(&words[opening].text.as_str()) {
        opening += 1;
    }
    if opening == 0 {
        return text.to_owned();
    }
    let rest = &text[words[opening].start..];
    let mut chars = rest.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_remove_the_command_that_ends_the_text() {
        for (text, kept) in [
            ("We meet on Friday. Stop.", "We meet on Friday."),
            (
                "We meet on Friday, stop dictation please.",
                "We meet on Friday.",
            ),
            ("We meet on Friday stop stop", "We meet on Friday"),
            ("We meet on Friday. Stop, enter.", "We meet on Friday."),
            ("Встреча в пятницу. Стоп.", "Встреча в пятницу."),
            ("Встреча в пятницу, стоп энтер старт", "Встреча в пятницу"),
            ("We meet on Friday. Cancel, clear.", "We meet on Friday."),
        ] {
            assert_eq!(without_trailing_command(text), kept, "{text}");
        }
    }

    #[test]
    fn should_keep_a_text_that_ends_in_other_words() {
        for text in [
            "We meet on Friday.",
            "Please don't stop the server.",
            "Clear text editing is enabled",
            "",
        ] {
            assert_eq!(without_trailing_command(text), text, "{text}");
        }
    }

    #[test]
    fn should_remove_a_start_that_opens_the_text() {
        assert_eq!(
            without_leading_command(
                VoiceCommand::StartDictation,
                "Start, meeting moved to Friday."
            ),
            "Meeting moved to Friday."
        );
        assert_eq!(
            without_leading_command(
                VoiceCommand::StartDictation,
                "start dictation we meet at ten"
            ),
            "We meet at ten"
        );
        assert_eq!(
            without_leading_command(VoiceCommand::StartDictation, "Старт. Встреча в пятницу."),
            "Встреча в пятницу."
        );
        assert_eq!(
            without_leading_command(VoiceCommand::StartDictation, "Starting tomorrow we meet."),
            "Starting tomorrow we meet."
        );
    }

    #[test]
    fn should_remove_a_restart_that_opens_the_text_of_a_restarted_dictation() {
        for (text, kept) in [
            (
                "Restart, restart, coffee machine is broken.",
                "Coffee machine is broken.",
            ),
            ("restart dictation we meet at ten", "We meet at ten"),
            ("Restart. Lunch is at noon.", "Lunch is at noon."),
        ] {
            assert_eq!(
                without_leading_command(VoiceCommand::RestartDictation, text),
                kept,
                "{text}"
            );
        }
    }

    #[test]
    fn should_remove_a_please_that_opens_the_text_after_a_voice_start() {
        for (command, text, kept) in [
            (
                VoiceCommand::StartDictation,
                "please. The build is green again.",
                "The build is green again.",
            ),
            (
                VoiceCommand::StartDictation,
                "Start dictation please, meeting moved to Friday.",
                "Meeting moved to Friday.",
            ),
            (
                VoiceCommand::RestartDictation,
                "Пожалуйста. Встреча в пятницу.",
                "Встреча в пятницу.",
            ),
            (VoiceCommand::StartDictation, "Please.", "Please."),
        ] {
            assert_eq!(without_leading_command(command, text), kept, "{text}");
        }
    }

    #[test]
    fn should_keep_words_of_another_command_that_open_the_text() {
        for (opened_by, text) in [
            (VoiceCommand::StartDictation, "Restart the server tonight."),
            (VoiceCommand::RestartDictation, "Start the engine."),
        ] {
            assert_eq!(without_leading_command(opened_by, text), text, "{text}");
        }
    }

    #[test]
    fn should_remove_a_doubled_start_that_opens_the_text() {
        for (text, kept) in [
            ("Start, start, the build is green.", "The build is green."),
            ("start start we meet at ten", "We meet at ten"),
            ("Старт старт. Встреча в пятницу.", "Встреча в пятницу."),
            ("Start dictation, start the engine.", "Start the engine."),
        ] {
            assert_eq!(
                without_leading_command(VoiceCommand::StartDictation, text),
                kept,
                "{text}"
            );
        }
    }
}
