//! The words a streaming recognizer heard so far, each with its times, from the tokens it
//! decoded and the time it decoded each.

use super::respell::{respell_last_word, respell_word};

/// A word heard, with its times in the audio.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedWord {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

/// How long the last token of a word lasts after the time it was decoded at.
const TOKEN_SECS: f32 = 0.1;

/// The words of `tokens`, decoded at `times` seconds after `offset`.
///
/// A token opening with a word boundary (`▁` or a space) starts a word. With `respelled`, a Cyrillic word of command
/// sounds becomes its English words, sharing its time evenly; the last word also reads as "enter"
/// when it opens with the sounds of "энтер" (`respell_last_word`).
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
        let last = words.pop();
        words = words
            .into_iter()
            .flat_map(|w| respell_timed(w, respell_word))
            .collect();
        words.extend(
            last.into_iter()
                .flat_map(|w| respell_timed(w, respell_last_word)),
        );
    }
    words
}

fn respell_timed(
    word: TimedWord,
    respell: fn(&str) -> Option<Vec<&'static str>>,
) -> Vec<TimedWord> {
    let Some(english) = respell(&word.text) else {
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
    fn should_keep_sto_before_another_word() {
        // When
        let words = timed_words(&tokens(&["▁сто", "▁рублей"]), &[1.0, 1.3], 0.0, true);
        // Then
        assert_eq!(texts(&words), ["сто", "рублей"]);
    }

    #[test]
    fn should_read_a_last_russian_word_opening_with_the_sounds_of_enter_as_enter() {
        // When
        let words = timed_words(
            &tokens(&["▁там", "▁отмеча", "ть", "▁эн", "тер", "ф"]),
            &[1.0, 1.3, 1.5, 1.8, 1.9, 2.0],
            0.0,
            true,
        );
        // Then
        assert_eq!(texts(&words), ["там", "отмечать", "enter"]);
    }

    #[test]
    fn should_keep_a_word_opening_with_the_sounds_of_enter_inside_speech() {
        // When
        let words = timed_words(
            &tokens(&["▁эн", "тер", "ф", "▁там", "▁отмечать"]),
            &[1.0, 1.1, 1.2, 1.5, 1.8],
            0.0,
            true,
        );
        // Then
        assert_eq!(texts(&words), ["энтерф", "там", "отмечать"]);
    }

    #[test]
    fn should_keep_a_last_word_opening_with_inter() {
        for word in ["интересно", "интерфейс", "интернет"] {
            // When
            let words = timed_words(
                &tokens(&["▁это", &format!("▁{word}")]),
                &[1.0, 1.3],
                0.0,
                true,
            );
            // Then
            assert_eq!(texts(&words), ["это", word]);
        }
    }

    #[test]
    fn should_keep_a_last_english_word_opening_with_enter() {
        // When
        let words = timed_words(&tokens(&["▁the", "▁enterprise"]), &[1.0, 1.3], 0.0, false);
        // Then
        assert_eq!(texts(&words), ["the", "enterprise"]);
    }
}
