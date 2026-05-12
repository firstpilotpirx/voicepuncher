#[derive(Debug, Clone, Copy)]
pub struct ChunkingCfg {
    pub chunk_threshold_secs: u32,
    pub chunk_overlap_secs: u32,
    pub vad_rms_threshold: f32,
    pub vad_lookback_secs: u32,
}

impl Default for ChunkingCfg {
    fn default() -> Self {
        Self {
            chunk_threshold_secs: 0,
            chunk_overlap_secs: 2,
            vad_rms_threshold: 0.01,
            vad_lookback_secs: 5,
        }
    }
}

pub fn merge_chunk_texts(texts: Vec<String>, overlap_word_fraction: f64) -> String {
    let mut all_words: Vec<&str> = Vec::new();
    for (chunk_index, text) in texts.iter().enumerate() {
        let words: Vec<&str> = text.split_whitespace().collect();
        let words_to_keep = if chunk_index == 0 {
            &words[..]
        } else {
            let drop_count = (words.len() as f64 * overlap_word_fraction).ceil() as usize;
            let drop_count = drop_count.min(words.len());
            &words[drop_count..]
        };
        all_words.extend_from_slice(words_to_keep);
    }
    all_words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_single_chunk_unchanged() {
        // Given
        let texts = vec!["hello world foo".to_string()];
        // When
        let merged = merge_chunk_texts(texts, 0.5);
        // Then
        assert_eq!(merged, "hello world foo");
    }

    #[test]
    fn should_drop_overlap_words_from_subsequent_chunks() {
        // Given — overlap_word_fraction 0.5, chunk2 has 4 words → drop ceil(4*0.5)=2
        let texts = vec![
            "alpha beta gamma".to_string(),
            "gamma delta epsilon zeta".to_string(),
        ];
        // When
        let merged = merge_chunk_texts(texts, 0.5);
        // Then — chunk1 kept whole, 2 words dropped from chunk2
        assert_eq!(merged, "alpha beta gamma epsilon zeta");
    }

    #[test]
    fn should_produce_empty_string_from_all_empty_chunks() {
        // Given
        let texts = vec![String::new(), String::new(), String::new()];
        // When
        let merged = merge_chunk_texts(texts, 0.3);
        // Then
        assert_eq!(merged, "");
    }

    #[test]
    fn should_keep_all_words_from_first_chunk() {
        // Given
        let texts = vec![
            "one two three four five".to_string(),
            "five six seven".to_string(),
        ];
        // When
        let merged = merge_chunk_texts(texts, 0.33);
        // Then — all 5 words from first chunk present
        let first_five: Vec<&str> = merged.split_whitespace().take(5).collect();
        assert_eq!(first_five, vec!["one", "two", "three", "four", "five"]);
    }

    fn word_count(s: &str, word: &str) -> usize {
        s.split_whitespace().filter(|w| *w == word).count()
    }

    // The four tests below characterize the chunk-stitching duplication artifact
    // reported in transcripts (spurious 2–3 word repeats). They use the realistic
    // default config: 2 s overlap inside a 30 s chunk → overlap_fraction ≈ 0.067.
    // Each test fails on the current naïve positional-drop implementation; they
    // serve as the spec for a content-aware merge.

    #[test]
    fn should_not_duplicate_overlap_word_when_actual_overlap_exceeds_fraction() {
        // Given — Whisper transcribed the 2 s boundary twice; chunk2 starts with
        //   2 words that already appear at the tail of chunk1. The positional
        //   drop with fraction 0.067 removes only ceil(6 * 0.067) = 1 word from
        //   chunk2, so "fox" survives a second time.
        let texts = vec![
            "the quick brown fox".to_string(),
            "brown fox jumps over lazy dog".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        // When
        let merged = merge_chunk_texts(texts, overlap_fraction);
        // Then
        assert_eq!(
            word_count(&merged, "brown"),
            1,
            "word 'brown' duplicated at chunk boundary: {merged:?}"
        );
        assert_eq!(
            word_count(&merged, "fox"),
            1,
            "word 'fox' duplicated at chunk boundary: {merged:?}"
        );
    }

    #[test]
    fn should_not_duplicate_three_word_phrase_at_chunk_boundary() {
        // Given — chunks share a 3-word overlap phrase ("it is great").
        //   With fraction 0.067 over chunk2's 7 words, only 1 word is dropped,
        //   leaving "is great" duplicated.
        let texts = vec![
            "I think it is great".to_string(),
            "it is great that we are here".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        // When
        let merged = merge_chunk_texts(texts, overlap_fraction);
        // Then
        assert_eq!(
            word_count(&merged, "is"),
            1,
            "word 'is' duplicated at chunk boundary: {merged:?}"
        );
        assert_eq!(
            word_count(&merged, "great"),
            1,
            "word 'great' duplicated at chunk boundary: {merged:?}"
        );
    }

    #[test]
    fn should_not_remove_real_words_when_actual_overlap_smaller_than_fraction() {
        // Given — the 2 s overlap window contained silence (no spoken words),
        //   so chunk2 starts with brand-new content. The positional drop still
        //   removes the legitimate first word.
        let texts = vec![
            "first sentence ends with a long pause".to_string(),
            "second sentence starts after the pause continues here for more".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        // When
        let merged = merge_chunk_texts(texts, overlap_fraction);
        // Then — chunk2's opening must survive intact
        assert!(
            merged.contains("second sentence"),
            "first word of chunk2 wrongly dropped: {merged:?}"
        );
    }

    #[test]
    fn should_not_duplicate_overlap_across_three_chunks() {
        // Given — three chunks where each consecutive pair shares a 3-word
        //   overlap phrase. The positional drop fails to remove enough words
        //   at both boundaries.
        let texts = vec![
            "good morning everyone today we will discuss".to_string(),
            "we will discuss the architecture of our system".to_string(),
            "of our system and how it scales".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        // When
        let merged = merge_chunk_texts(texts, overlap_fraction);
        // Then — overlap words must not duplicate across the seams
        assert_eq!(
            word_count(&merged, "will"),
            1,
            "word 'will' duplicated across seam 1: {merged:?}"
        );
        assert_eq!(
            word_count(&merged, "discuss"),
            1,
            "word 'discuss' duplicated across seam 1: {merged:?}"
        );
        assert_eq!(
            word_count(&merged, "our"),
            1,
            "word 'our' duplicated across seam 2: {merged:?}"
        );
        assert_eq!(
            word_count(&merged, "system"),
            1,
            "word 'system' duplicated across seam 2: {merged:?}"
        );
    }
}
