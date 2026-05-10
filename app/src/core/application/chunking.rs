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
}
