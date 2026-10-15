use std::collections::HashMap;

/// Phrases Whisper makes up for silence: the sentence that holds one is removed, so a
/// subtitle credit goes together with its name.
const HALLUCINATION_PHRASES: &[&str] = &[
    "субтитры создавал",
    "продолжение следует",
    "спасибо за просмотр",
    "подписывайтесь на канал",
    "ставьте лайк",
    "thank you for watching",
    "thanks for watching",
    "please subscribe",
    "like and subscribe",
    "subtitles by",
    "subtitled by",
    "captions by",
];

/// Markers Whisper writes for non-speech: removed wherever they stand.
const HALLUCINATION_MARKERS: &[&str] = &[
    "[music]",
    "[applause]",
    "[blank_audio]",
    "(music)",
    "(applause)",
];

/// Characters that end a sentence; a run of them ("...", "?!") ends it once.
const SENTENCE_ENDS: &[char] = &['.', '!', '?', '…'];

/// Whisper returns this word alone, or repeated, for a recording of silence.
const SILENCE_WORD: &str = "you";

const MIN_WORDS_FOR_LOOP_CHECK: usize = 10;
/// A transcript is a loop when one word makes up more than this share of all words.
const LOOP_WORD_SHARE_NUMERATOR: usize = 4;
const LOOP_WORD_SHARE_DENOMINATOR: usize = 10;

/// The transcript without the known hallucinations of Whisper-family services: markers are
/// removed, and so is every sentence holding a subtitle credit or subscription call. `None`
/// when no speech is left: nothing, a phrase repeated in a loop, or a lone "you".
pub fn strip_whisper_hallucinations(text: &str) -> Option<String> {
    let without_markers = text
        .split_whitespace()
        .filter(|token| !is_marker(token))
        .collect::<Vec<_>>()
        .join(" ");
    let speech = sentences(&without_markers)
        .into_iter()
        .filter(|sentence| {
            let lower = sentence.to_lowercase();
            !HALLUCINATION_PHRASES.iter().any(|p| lower.contains(p))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let lower = speech.to_lowercase();
    if speech.is_empty() || has_repetition_loop(&lower) || is_only_silence_word(&lower) {
        return None;
    }
    Some(speech)
}

/// True when the token is a non-speech marker, trailing punctuation aside.
fn is_marker(token: &str) -> bool {
    let lower = token.to_lowercase();
    let bare = lower.trim_end_matches(|c: char| !c.is_alphanumeric() && c != ']' && c != ')');
    HALLUCINATION_MARKERS.contains(&bare)
}

/// `text` split after each run of [`SENTENCE_ENDS`]; text after the last run is a sentence too.
fn sentences(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let run_goes_on = chars
            .peek()
            .is_some_and(|(_, next)| SENTENCE_ENDS.contains(next));
        if SENTENCE_ENDS.contains(&c) && !run_goes_on {
            let end = i + c.len_utf8();
            found.push(text[start..end].trim());
            start = end;
        }
    }
    found.push(text[start..].trim());
    found.retain(|sentence| !sentence.is_empty());
    found
}

/// True when every word is "you"; tokens made only of punctuation are ignored.
fn is_only_silence_word(text: &str) -> bool {
    let mut words = text
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .peekable();
    words.peek().is_some() && words.all(|word| word == SILENCE_WORD)
}

fn has_repetition_loop(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < MIN_WORDS_FOR_LOOP_CHECK {
        return false;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for &w in &words {
        *counts.entry(w).or_insert(0) += 1;
    }
    let max_freq = counts.values().max().copied().unwrap_or(0);
    max_freq * LOOP_WORD_SHARE_DENOMINATOR > words.len() * LOOP_WORD_SHARE_NUMERATOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_speech_before_a_trailing_subscription_call() {
        assert_eq!(
            strip_whisper_hallucinations(
                "I restarted the service. We split the module into several small parts. Thanks for watching."
            )
            .as_deref(),
            Some("I restarted the service. We split the module into several small parts.")
        );
    }

    #[test]
    fn should_remove_a_credit_with_its_name_and_keep_the_speech() {
        assert_eq!(
            strip_whisper_hallucinations("Привет, как дела? Субтитры создавал DimaTorzok")
                .as_deref(),
            Some("Привет, как дела?")
        );
    }

    #[test]
    fn should_remove_a_marker_inside_speech() {
        assert_eq!(
            strip_whisper_hallucinations("the index [BLANK_AUDIO] was never built.").as_deref(),
            Some("the index was never built.")
        );
    }

    #[test]
    fn should_leave_speech_without_hallucination_unchanged() {
        for text in [
            "Hello, how are you doing?",
            "Привет, как дела?",
            "you know",
            "Thank you",
            "и потом я пошёл и потом я взял и потом позвонил другу",
        ] {
            assert_eq!(strip_whisper_hallucinations(text).as_deref(), Some(text));
        }
    }

    #[test]
    fn should_give_no_text_for_hallucination_only() {
        let looped = "и сайта, ".repeat(20);
        for text in [
            " Субтитры создавал DimaTorzok",
            "Продолжение следует...",
            "Thank you for watching!",
            "[BLANK_AUDIO]",
            "[Music] [Applause]",
            "you",
            " You.",
            "you you",
            "YOU, you!",
            "You. - You.",
            "You. Thanks for watching.",
            looped.as_str(),
        ] {
            assert_eq!(strip_whisper_hallucinations(text), None, "{text:?}");
        }
    }

    #[test]
    fn should_give_no_text_for_an_empty_transcript() {
        assert_eq!(strip_whisper_hallucinations(""), None);
        assert_eq!(strip_whisper_hallucinations("   "), None);
    }
}
