pub fn wer(reference: &str, hypothesis: &str) -> f64 {
    let ref_words = normalize_words(reference);
    let hyp_words = normalize_words(hypothesis);
    edit_distance_rate(&ref_words, &hyp_words)
}

pub fn cer(reference: &str, hypothesis: &str) -> f64 {
    let ref_chars = normalize_chars(reference);
    let hyp_chars = normalize_chars(hypothesis);
    edit_distance_rate(&ref_chars, &hyp_chars)
}

fn normalize_words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .replace('ё', "е")
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn normalize_chars(s: &str) -> Vec<String> {
    s.to_lowercase()
        .replace('ё', "е")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_string())
        .collect()
}

fn edit_distance_rate<T: PartialEq>(reference: &[T], hypothesis: &[T]) -> f64 {
    let n = reference.len();
    let m = hypothesis.len();
    if n == 0 {
        return if m == 0 { 0.0 } else { 1.0 };
    }
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if reference[i - 1] == hypothesis[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m] as f64 / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_zero_for_identical_strings() {
        assert_eq!(wer("hello world", "hello world"), 0.0);
    }

    #[test]
    fn should_return_one_for_single_word_substitution() {
        let result = wer("hello", "world");
        assert!((result - 1.0).abs() < 1e-9);
    }

    #[test]
    fn should_return_zero_for_both_empty() {
        assert_eq!(wer("", ""), 0.0);
    }

    #[test]
    fn should_return_one_for_empty_reference_non_empty_hypothesis() {
        assert_eq!(wer("", "something"), 1.0);
    }

    #[test]
    fn should_normalize_punctuation_and_case() {
        assert_eq!(wer("Hello, World!", "hello world"), 0.0);
    }

    #[test]
    fn should_normalize_yo_to_ye_for_russian() {
        assert_eq!(wer("ёлка", "елка"), 0.0);
    }
}
