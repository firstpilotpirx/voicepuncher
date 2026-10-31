//! The words a streaming recognizer heard so far, each with its times, from the tokens it
//! decoded and the time it decoded each.

use crate::confirm::TimedWord;

use super::respell::{before_please, respell_word};

/// How long the last token of a word lasts after the time it was decoded at.
const TOKEN_SECS: f32 = 0.1;

/// The words of `tokens`, decoded at `times` seconds after `offset`.
///
/// A token opening with a word boundary (`▁` or a space) starts a word. With `respelled`, a Cyrillic word of command
/// sounds becomes its English words, sharing its time evenly.
pub fn timed_words(
    tokens: &[String],
    times: &[f32],
    offset: f32,
    respelled: bool,
) -> Vec<TimedWord> {
    let mut words: Vec<TimedWord> = Vec::new();
    for (token, &time) in tokens.iter().zip(times) {
        let opens = token.starts_with('▁') || token.starts_with(' ');
        let text = token.trim_start_matches(['▁', ' ']);
        let at = offset + time;
        match words.last_mut() {
            Some(word) if !opens => {
                word.text.push_str(text);
                word.end = at + TOKEN_SECS;
            }
            _ => words.push(TimedWord { text: text.to_owned(), start: at, end: at + TOKEN_SECS }),
        }
    }
    words.retain(|w| !w.text.is_empty());
    if respelled {
        words = words.into_iter().flat_map(respell_timed).collect();
        respell_before_please(&mut words);
    }
    words
}

/// A Russian word that is a command word only before "please" («сто плиз»), respelled.
fn respell_before_please(words: &mut [TimedWord]) {
    for i in 1..words.len() {
        let english = (words[i].text == "please")
            .then(|| before_please(&words[i - 1].text))
            .flatten();
        if let Some(english) = english {
            english.clone_into(&mut words[i - 1].text);
        }
    }
}

fn respell_timed(word: TimedWord) -> Vec<TimedWord> {
    let Some(english) = respell_word(&word.text) else {
        return vec![word];
    };
    let share = (word.end - word.start) / english.len() as f32;
    english
        .iter()
        .enumerate()
        .map(|(i, text)| TimedWord {
            text: (*text).to_owned(),
            start: word.start + share * i as f32,
            end: word.start + share * (i + 1) as f32,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(pieces: &[&str]) -> Vec<String> {
        pieces.iter().map(|p| (*p).to_owned()).collect()
    }

    fn texts(words: &[TimedWord]) -> Vec<&str> {
        words.iter().map(|w| w.text.as_str()).collect()
    }

    #[test]
    fn should_join_the_tokens_of_a_word_from_its_boundary_mark() {
        // When
        let words = timed_words(
            &tokens(&["▁pr", "ess", "▁en", "ter", "."]),
            &[0.1, 0.2, 0.5, 0.6, 0.7],
            10.0,
            false,
        );
        // Then
        assert_eq!(texts(&words), ["press", "enter."]);
        assert!((words[1].start - 10.5).abs() < 1e-4, "{}", words[1].start);
        assert!((words[1].end - 10.8).abs() < 1e-4, "{}", words[1].end);
    }

    #[test]
    fn should_open_a_word_at_a_space_too() {
        // When
        let words = timed_words(&tokens(&[" stop", " stop"]), &[0.1, 0.4], 0.0, false);
        // Then
        assert_eq!(texts(&words), ["stop", "stop"]);
    }

    #[test]
    fn should_respell_a_russian_double_into_two_english_words_sharing_its_time() {
        // When
        let words = timed_words(
            &tokens(&["▁эн", "тер", "эн", "тер"]),
            &[1.0, 1.1, 1.2, 1.3],
            0.0,
            true,
        );
        // Then
        assert_eq!(texts(&words), ["enter", "enter"]);
        assert!((words[1].start - 1.2).abs() < 1e-4, "{}", words[1].start);
    }

    #[test]
    fn should_keep_a_russian_word_of_no_command_sounds() {
        // When
        let words = timed_words(&tokens(&["▁при", "вет"]), &[1.0, 1.1], 0.0, true);
        // Then
        assert_eq!(texts(&words), ["привет"]);
    }

    #[test]
    fn should_take_sto_before_please_for_stop() {
        // When
        let words = timed_words(&tokens(&["▁сто", "▁плис"]), &[1.0, 1.3], 0.0, true);
        // Then
        assert_eq!(texts(&words), ["stop", "please"]);
    }

    #[test]
    fn should_keep_sto_before_another_word() {
        // When
        let words = timed_words(&tokens(&["▁сто", "▁рублей"]), &[1.0, 1.3], 0.0, true);
        // Then
        assert_eq!(texts(&words), ["сто", "рублей"]);
    }
}
