use super::{CleanedText, RawTranscript};

/// Hesitation sounds a dictation never means to write, in lowercase without hyphens.
const FILLER_SOUNDS: [&str; 13] = [
    "э", "ээ", "эээ", "эм", "мм", "ммм", "um", "umm", "uh", "uhm", "hmm", "er", "erm",
];

const SENTENCE_MARKS: [char; 3] = ['.', '!', '?'];
const CLAUSE_MARKS: [char; 3] = [',', ';', ':'];

/// Whether `word` (any case, hyphens allowed) is a hesitation sound such as "э-э" or "um".
pub fn is_filler_sound(word: &str) -> bool {
    let letters: String = word
        .chars()
        .filter(|c| c.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    FILLER_SOUNDS.contains(&letters.as_str())
}

/// `text` without its hesitation sounds and the comma that follows each of them; a
/// sentence mark after a dropped sound moves to the word before it.
pub fn without_filler_sounds(text: &str) -> String {
    let mut kept: Vec<String> = Vec::new();
    for token in text.split_whitespace() {
        if !is_filler_sound(token) {
            kept.push(token.to_owned());
            continue;
        }
        let Some(mark) = token.chars().find(|c| SENTENCE_MARKS.contains(c)) else {
            continue;
        };
        if let Some(previous) = kept.last_mut() {
            let bare_len = previous.trim_end_matches(CLAUSE_MARKS).len();
            previous.truncate(bare_len);
            if !previous.ends_with(SENTENCE_MARKS) {
                previous.push(mark);
            }
        }
    }
    kept.join(" ")
}

/// The text a dictation pastes: `raw` without its filler sounds, or `raw` as returned
/// when it holds nothing but filler sounds; `None` only when `raw` cannot be pasted at all.
pub fn text_to_paste(raw: &RawTranscript) -> Option<CleanedText> {
    let spoken = without_filler_sounds(raw.as_str());
    let text = if spoken.is_empty() {
        raw.as_str().to_owned()
    } else {
        spoken
    };
    CleanedText::new(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_recognize_russian_and_english_sounds_in_any_case() {
        for word in ["э-э", "Э-э", "эм", "Ммм", "um", "Uh", "hmm"] {
            assert!(is_filler_sound(word), "{word}");
        }
    }

    #[test]
    fn should_not_take_words_for_sounds() {
        for word in ["ну", "типа", "эмоции", "umbrella", "это", "мама"] {
            assert!(!is_filler_sound(word), "{word}");
        }
    }

    #[test]
    fn should_drop_sounds_with_the_comma_after_them() {
        // Given / When
        let text = without_filler_sounds("Эм, давай, э-э, созвонимся завтра утром.");
        // Then
        assert_eq!(text, "давай, созвонимся завтра утром.");
    }

    #[test]
    fn should_drop_english_sounds_between_words() {
        assert_eq!(
            without_filler_sounds("I think um we should uh postpone the release"),
            "I think we should postpone the release"
        );
    }

    #[test]
    fn should_keep_the_sentence_mark_of_a_trailing_sound() {
        assert_eq!(without_filler_sounds("Я не уверен, ммм."), "Я не уверен.");
    }

    #[test]
    fn should_leave_nothing_of_a_sound_only_transcript() {
        assert_eq!(without_filler_sounds("Э-э, эм..."), "");
    }

    fn pasted(transcript: &str) -> String {
        let raw = RawTranscript::new(transcript.to_owned()).unwrap();
        text_to_paste(&raw).unwrap().as_str().to_owned()
    }

    #[test]
    fn should_paste_the_transcript_without_its_filler_sounds() {
        assert_eq!(
            pasted("Эм, давай, э-э, созвонимся завтра утром."),
            "давай, созвонимся завтра утром."
        );
    }

    #[test]
    fn should_paste_a_transcript_without_filler_sounds_unchanged() {
        assert_eq!(
            pasted("переведи это на английский пожалуйста"),
            "переведи это на английский пожалуйста"
        );
    }

    #[test]
    fn should_paste_a_filler_only_transcript_as_it_was_returned() {
        assert_eq!(pasted("Э-э, эм..."), "Э-э, эм...");
    }
}
