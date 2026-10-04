use std::collections::HashMap;

const HALLUCINATION_PATTERNS: &[&str] = &[
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
    "[music]",
    "[applause]",
    "[blank_audio]",
    "(music)",
    "(applause)",
];

const MIN_WORDS_FOR_LOOP_CHECK: usize = 10;
/// A transcript is a loop when one word makes up more than this share of all words.
const LOOP_WORD_SHARE_NUMERATOR: usize = 4;
const LOOP_WORD_SHARE_DENOMINATOR: usize = 10;

/// True when a Whisper-family transcript is a known hallucination (subtitle credits,
/// subscription calls, blank-audio markers, a phrase repeated in a loop) rather than speech.
pub fn is_whisper_hallucination(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    HALLUCINATION_PATTERNS.iter().any(|p| lower.contains(p)) || has_repetition_loop(&lower)
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
    fn should_detect_russian_subtitle_hallucination() {
        assert!(is_whisper_hallucination(" Субтитры создавал DimaTorzok"));
    }

    #[test]
    fn should_detect_continuation_hallucination() {
        assert!(is_whisper_hallucination("Продолжение следует..."));
    }

    #[test]
    fn should_detect_english_subscribe_hallucination() {
        assert!(is_whisper_hallucination("Thank you for watching!"));
    }

    #[test]
    fn should_detect_blank_audio_token() {
        assert!(is_whisper_hallucination("[BLANK_AUDIO]"));
    }

    #[test]
    fn should_not_filter_real_speech() {
        assert!(!is_whisper_hallucination("Hello, how are you doing?"));
        assert!(!is_whisper_hallucination("Привет, как дела?"));
    }

    #[test]
    fn should_not_filter_empty_string() {
        assert!(!is_whisper_hallucination(""));
        assert!(!is_whisper_hallucination("   "));
    }

    #[test]
    fn should_detect_repeated_phrase_loop() {
        let repeated = "и сайта, ".repeat(20);
        assert!(is_whisper_hallucination(&repeated));
    }

    #[test]
    fn should_not_filter_speech_with_moderate_repetition() {
        assert!(!is_whisper_hallucination(
            "и потом я пошёл и потом я взял и потом позвонил другу"
        ));
    }
}
