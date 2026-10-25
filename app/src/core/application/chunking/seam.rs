//! Merging the transcripts of two chunks at their seam.

/// Words at the end of the text so far that the seam search looks at.
pub(super) const SEAM_TAIL_WORDS: usize = 24;

/// Words at the start of the next chunk that the seam search looks at.
pub(super) const SEAM_HEAD_WORDS: usize = 24;

/// Unmatched words allowed after the overlap at the end of a chunk: words cut short,
/// misheard or made up where the audio stops mid-sentence.
pub(super) const TAIL_EDGE_SLACK: usize = 6;

/// Unmatched words allowed before the overlap at the start of the next chunk: words cut
/// short or misheard where its audio starts mid-word.
pub(super) const HEAD_EDGE_SLACK: usize = 4;

/// Similarity from which two words count as the same word heard twice.
pub(super) const WORD_MATCH_SIMILARITY: f64 = 0.6;

/// Similarity of a word and the same word cut short at a chunk edge.
pub(super) const CUT_SHORT_SIMILARITY: f64 = 0.8;

/// A cut-short word must keep at least this many characters to count.
pub(super) const CUT_SHORT_MIN_CHARS: usize = 3;

/// Shorter words match only when spelled the same: one letter is too much of them.
pub(super) const FUZZY_MIN_CHARS: usize = 4;

pub(super) const MATCH_SCORE: f64 = 2.0;

pub(super) const MISMATCH_PENALTY: f64 = 1.5;

pub(super) const GAP_PENALTY: f64 = 1.0;

/// Joins chunk transcripts, keeping the overlap of every two neighbouring chunks once even
/// when the two chunks transcribed it differently.
pub fn merge_chunk_texts(texts: &[String]) -> String {
    let mut merged: Vec<&str> = Vec::new();
    for text in texts {
        let next: Vec<&str> = text.split_whitespace().collect();
        match find_seam(&merged, &next) {
            Some(seam) => {
                log::debug!(
                    "chunk merge: seam after word {} of the text, word {} of the chunk",
                    seam.merged_until,
                    seam.next_from
                );
                merged.truncate(seam.merged_until);
                merged.extend_from_slice(&next[seam.next_from..]);
            }
            None => merged.extend_from_slice(&next),
        }
    }
    merged.join(" ")
}

/// Where two chunk texts meet: keep `merged[..merged_until]`, then `next[next_from..]`.
#[derive(Debug, PartialEq)]
pub(super) struct Seam {
    merged_until: usize,
    next_from: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Step {
    Stop,
    Diagonal,
    Up,
    Left,
}

/// Aligns the end of `merged` with the start of `next` (local alignment of normalized
/// words). The seam goes through the middle of the aligned overlap, where both chunks heard
/// the words with context on both sides; the words near either chunk edge are dropped.
pub(super) fn find_seam(merged: &[&str], next: &[&str]) -> Option<Seam> {
    let tail_start = merged.len().saturating_sub(SEAM_TAIL_WORDS);
    let tail: Vec<String> = merged[tail_start..].iter().map(|w| normalize(w)).collect();
    let head: Vec<String> = next[..next.len().min(SEAM_HEAD_WORDS)]
        .iter()
        .map(|w| normalize(w))
        .collect();
    let (n, m) = (tail.len(), head.len());
    if n == 0 || m == 0 {
        return None;
    }
    // Only the last word of the text so far and the first word of the chunk can be cut short.
    let similar = |i: usize, j: usize| similarity(&tail[i - 1], &head[j - 1], i == n || j == 1);
    let alignment = align(n, m, &similar);
    let pairs = alignment.matched_pairs(&similar)?;
    let (i, j) = split_point(&pairs, n)?;
    Some(Seam { merged_until: tail_start + i, next_from: j })
}

/// The local alignment table of `n` words of the text so far against `m` words of the chunk.
struct Alignment {
    score: Vec<Vec<f64>>,
    step: Vec<Vec<Step>>,
    n: usize,
    m: usize,
}

/// Fills the alignment table; `similar(i, j)` compares word `i` of the tail with word `j`
/// of the head, both counted from 1.
fn align(n: usize, m: usize, similar: &impl Fn(usize, usize) -> f64) -> Alignment {
    let mut score = vec![vec![0.0f64; m + 1]; n + 1];
    let mut step = vec![vec![Step::Stop; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            let s = similar(i, j);
            let diagonal = score[i - 1][j - 1]
                + if s >= WORD_MATCH_SIMILARITY {
                    MATCH_SCORE * s
                } else {
                    -MISMATCH_PENALTY
                };
            let candidates = [
                (diagonal, Step::Diagonal),
                (score[i - 1][j] - GAP_PENALTY, Step::Up),
                (score[i][j - 1] - GAP_PENALTY, Step::Left),
            ];
            (score[i][j], step[i][j]) = candidates.into_iter().fold(
                (0.0, Step::Stop),
                |best, c| if c.0 > best.0 { c } else { best },
            );
        }
    }
    Alignment { score, step, n, m }
}

impl Alignment {
    /// The matched words of the best alignment ending near the end of the text so far, in
    /// order, with their similarity; `None` when they do not reach both edges.
    fn matched_pairs(
        &self,
        similar: &impl Fn(usize, usize) -> f64,
    ) -> Option<Vec<(usize, usize, f64)>> {
        let pairs = self.traceback(self.best_end()?, similar);
        // The matched words must reach the edges themselves, not only through gaps: near the
        // end of the text so far and near the start of the next chunk.
        let (&(last_i, _, _), &(_, first_j, _)) = (pairs.last()?, pairs.first()?);
        (self.n - last_i <= TAIL_EDGE_SLACK && first_j - 1 <= HEAD_EDGE_SLACK).then_some(pairs)
    }

    /// The best scoring cell near the end of the text so far, where the traceback begins.
    fn best_end(&self) -> Option<(usize, usize)> {
        let (n, m, score) = (self.n, self.m, &self.score);
        (n.saturating_sub(TAIL_EDGE_SLACK).max(1)..=n)
            .flat_map(|i| (1..=m).map(move |j| (i, j)))
            .filter(|&(i, j)| score[i][j] > 0.0)
            .max_by(|&(a, b), &(c, d)| score[a][b].total_cmp(&score[c][d]))
    }

    /// The words matched on the way back from `end`, in order, each with its similarity.
    fn traceback(
        &self,
        end: (usize, usize),
        similar: &impl Fn(usize, usize) -> f64,
    ) -> Vec<(usize, usize, f64)> {
        let mut pairs = Vec::new();
        let mut at = Some(end);
        while let Some((i, j)) = at.filter(|&(i, j)| i > 0 && j > 0) {
            let here = self.step[i][j];
            let matched = (here == Step::Diagonal)
                .then(|| similar(i, j))
                .filter(|&s| s >= WORD_MATCH_SIMILARITY);
            pairs.extend(matched.map(|s| (i, j, s)));
            at = here.back(i, j);
        }
        pairs.reverse();
        pairs
    }
}

impl Step {
    /// The cell the traceback goes to from `(i, j)`; none where it stops.
    const fn back(self, i: usize, j: usize) -> Option<(usize, usize)> {
        match self {
            Self::Diagonal => Some((i - 1, j - 1)),
            Self::Up => Some((i - 1, j)),
            Self::Left => Some((i, j - 1)),
            Self::Stop => None,
        }
    }
}

/// The matched pair the seam goes through: the middle one of the words heard the same, or
/// of all matched words when none is; `None` when the match is not to be trusted.
fn split_point(pairs: &[(usize, usize, f64)], n: usize) -> Option<(usize, usize)> {
    let same = |&&(_, _, s): &&(usize, usize, f64)| s >= 1.0;
    let trusted = match pairs {
        [] => false,
        // A lone word counts only as the same word at both very edges.
        [(i, j, s)] => *s >= 1.0 && *i == n && *j == 1,
        _ => true,
    };
    if !trusted {
        return None;
    }
    let same_words: Vec<_> = pairs.iter().filter(same).collect();
    let splits = if same_words.is_empty() {
        pairs.iter().collect()
    } else {
        same_words
    };
    let &(i, j, _) = splits[(splits.len() - 1) / 2];
    Some((i, j))
}

/// Lowercase, `ё` read as `е`, every non-alphanumeric character dropped.
pub(super) fn normalize(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// 1.0 for the same word; [`CUT_SHORT_SIMILARITY`] when one is the other cut short and
/// `at_edge` allows that; for words of [`FUZZY_MIN_CHARS`] or more, 1 − edit distance /
/// length of the longer word; otherwise 0.
pub(super) fn similarity(a: &str, b: &str, at_edge: bool) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if at_edge
        && a.len().min(b.len()) >= CUT_SHORT_MIN_CHARS
        && (a.starts_with(&b) || b.starts_with(&a))
    {
        return CUT_SHORT_SIMILARITY;
    }
    if a.len().min(b.len()) < FUZZY_MIN_CHARS {
        return 0.0;
    }
    1.0 - levenshtein(&a, &b) as f64 / a.len().max(b.len()) as f64
}

pub(super) fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != cb));
            diagonal = above;
        }
    }
    row[b.len()]
}
