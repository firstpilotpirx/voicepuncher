//! How a transcript compares with the spoken text: its words, repeated phrases, dropped phrases and missing words.

/// Lower-case words, split at every character that is not alphanumeric.
pub fn split_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Words lowercased, `ё` read as `е`, every non-alphanumeric character dropped.
pub fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.to_lowercase()
                .replace('ё', "е")
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Spoken word pairs that occur more often in the transcript than in the spoken text — the
/// trace a chunk seam leaves when it keeps the overlap twice.
///
/// A pair never spoken is a recognition error, measured by WER, however often it is heard.
pub fn repeated_phrases(spoken: &str, transcript: &str) -> Vec<String> {
    use std::collections::HashMap;
    fn pairs(words: &[String]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for w in words.windows(2) {
            *counts.entry(format!("{} {}", w[0], w[1])).or_insert(0) += 1;
        }
        counts
    }
    let spoken = pairs(&normalized_words(spoken));
    let mut repeated: Vec<String> = pairs(&normalized_words(transcript))
        .into_iter()
        .filter(|(pair, n)| spoken.get(pair).is_some_and(|said| n > said))
        .map(|(pair, _)| pair)
        .collect();
    repeated.sort();
    repeated
}

/// Spoken words of the longest phrase the transcript dropped: in every gap between two
/// words both texts share in order, the spoken words beyond those the transcript puts in
/// their place.
///
/// A misheard phrase fills its gap and counts nothing.
pub fn dropped_phrase_len(spoken: &str, transcript: &str) -> usize {
    let s = normalized_words(spoken);
    let t = normalized_words(transcript);
    // lcs[i][j]: longest common subsequence of s[i..] and t[j..]
    let mut lcs = vec![vec![0usize; t.len() + 1]; s.len() + 1];
    for i in (0..s.len()).rev() {
        for j in (0..t.len()).rev() {
            lcs[i][j] = if s[i] == t[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let (mut gap_start_s, mut gap_start_t) = (0, 0);
    let mut longest = 0;
    while i < s.len() && j < t.len() {
        if s[i] == t[j] {
            longest = longest.max((i - gap_start_s).saturating_sub(j - gap_start_t));
            i += 1;
            j += 1;
            (gap_start_s, gap_start_t) = (i, j);
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    longest.max((s.len() - gap_start_s).saturating_sub(t.len() - gap_start_t))
}

/// Share of spoken words that the transcript does not contain in order (longest common
/// subsequence of normalized words).
pub fn missing_word_share(spoken: &str, transcript: &str) -> f64 {
    let s = normalized_words(spoken);
    let t = normalized_words(transcript);
    if s.is_empty() {
        return 0.0;
    }
    let mut row = vec![0usize; t.len() + 1];
    for sw in &s {
        let mut diagonal = 0;
        for (j, tw) in t.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if sw == tw {
                diagonal + 1
            } else {
                above.max(row[j])
            };
            diagonal = above;
        }
    }
    (s.len() - row[t.len()]) as f64 / s.len() as f64
}
