use log;

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

const OVERLAP_SEARCH_WINDOW: usize = 20;

fn find_overlap_len(accumulated: &[&str], new_words: &[&str]) -> usize {
    let tail_len = accumulated.len().min(OVERLAP_SEARCH_WINDOW);
    let tail = &accumulated[accumulated.len() - tail_len..];
    let max_possible = tail.len().min(new_words.len());
    for len in (1..=max_possible).rev() {
        if tail[tail.len() - len..] == new_words[..len] {
            return len;
        }
    }
    0
}

pub fn merge_chunk_texts(texts: Vec<String>, _overlap_word_fraction: f64) -> String {
    if texts.is_empty() {
        return String::new();
    }
    let chunk_words: Vec<Vec<&str>> = texts
        .iter()
        .map(|t| t.split_whitespace().collect())
        .collect();
    let mut result: Vec<&str> = chunk_words[0].clone();
    for new_words in chunk_words.iter().skip(1) {
        let overlap = find_overlap_len(&result, new_words);
        if overlap > 0 {
            log::debug!("chunk merge: dropping {overlap} overlapping word(s)");
        }
        let drop = overlap.min(new_words.len());
        let words_to_add = &new_words[drop..];
        result.extend_from_slice(words_to_add);
    }
    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_single_chunk_unchanged() {
        let texts = vec!["hello world foo".to_string()];
        let merged = merge_chunk_texts(texts, 0.5);
        assert_eq!(merged, "hello world foo");
    }

    #[test]
    fn should_drop_only_actual_overlap_words() {
        let texts = vec![
            "alpha beta gamma".to_string(),
            "gamma delta epsilon zeta".to_string(),
        ];
        let merged = merge_chunk_texts(texts, 0.5);
        assert_eq!(merged, "alpha beta gamma delta epsilon zeta");
    }

    #[test]
    fn should_produce_empty_string_from_all_empty_chunks() {
        let texts = vec![String::new(), String::new(), String::new()];
        let merged = merge_chunk_texts(texts, 0.3);
        assert_eq!(merged, "");
    }

    #[test]
    fn should_keep_all_words_from_first_chunk() {
        let texts = vec![
            "one two three four five".to_string(),
            "five six seven".to_string(),
        ];
        let merged = merge_chunk_texts(texts, 0.33);
        let first_five: Vec<&str> = merged.split_whitespace().take(5).collect();
        assert_eq!(first_five, vec!["one", "two", "three", "four", "five"]);
    }

    fn word_count(s: &str, word: &str) -> usize {
        s.split_whitespace().filter(|w| *w == word).count()
    }

    #[test]
    fn should_drop_actual_overlap_even_if_exceeds_fraction() {
        let texts = vec![
            "the quick brown fox".to_string(),
            "brown fox jumps over lazy dog".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        let merged = merge_chunk_texts(texts, overlap_fraction);
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
    fn should_drop_multi_word_phrase_overlap() {
        let texts = vec![
            "I think it is great".to_string(),
            "it is great that we are here".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        let merged = merge_chunk_texts(texts, overlap_fraction);
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
    fn should_keep_all_words_when_no_actual_overlap() {
        let texts = vec![
            "first sentence ends with a long pause".to_string(),
            "second sentence starts after the pause continues here for more".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        let merged = merge_chunk_texts(texts, overlap_fraction);
        assert!(
            merged.contains("second sentence"),
            "first word of chunk2 wrongly dropped: {merged:?}"
        );
    }

    #[test]
    fn should_drop_overlap_across_three_chunks() {
        let texts = vec![
            "good morning everyone today we will discuss".to_string(),
            "we will discuss the architecture of our system".to_string(),
            "of our system and how it scales".to_string(),
        ];
        let overlap_fraction = 2.0 / 30.0;
        let merged = merge_chunk_texts(texts, overlap_fraction);
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

    #[test]
    fn should_handle_completely_disjoint_chunks() {
        let texts = vec!["hello world".to_string(), "foo bar baz".to_string()];
        let merged = merge_chunk_texts(texts, 0.1);
        assert_eq!(merged, "hello world foo bar baz");
    }

    #[test]
    fn should_handle_identical_chunks() {
        let texts = vec![
            "alpha beta gamma".to_string(),
            "alpha beta gamma".to_string(),
        ];
        let merged = merge_chunk_texts(texts, 0.02);
        assert_eq!(merged, "alpha beta gamma");
    }

    #[test]
    fn should_handle_inner_empty_chunks() {
        let texts = vec![
            "first chunk text".to_string(),
            String::new(),
            "third chunk here".to_string(),
        ];
        let merged = merge_chunk_texts(texts, 0.1);
        assert!(merged.starts_with("first chunk text"));
        assert!(merged.ends_with("third chunk here"));
    }
}
