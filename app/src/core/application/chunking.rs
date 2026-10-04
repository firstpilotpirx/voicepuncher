use log;

use crate::core::domain::AudioBuffer16k;

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

/// Words at the end of the text so far that the seam search looks at.
const SEAM_TAIL_WORDS: usize = 24;
/// Words at the start of the next chunk that the seam search looks at.
const SEAM_HEAD_WORDS: usize = 24;
/// Unmatched words allowed after the overlap at the end of a chunk: words cut short,
/// misheard or made up where the audio stops mid-sentence.
const TAIL_EDGE_SLACK: usize = 6;
/// Unmatched words allowed before the overlap at the start of the next chunk: words cut
/// short or misheard where its audio starts mid-word.
const HEAD_EDGE_SLACK: usize = 4;
/// Similarity from which two words count as the same word heard twice.
const WORD_MATCH_SIMILARITY: f64 = 0.6;
/// Similarity of a word and the same word cut short at a chunk edge.
const CUT_SHORT_SIMILARITY: f64 = 0.8;
/// A cut-short word must keep at least this many characters to count.
const CUT_SHORT_MIN_CHARS: usize = 3;
/// Shorter words match only when spelled the same: one letter is too much of them.
const FUZZY_MIN_CHARS: usize = 4;
const MATCH_SCORE: f64 = 2.0;
const MISMATCH_PENALTY: f64 = 1.5;
const GAP_PENALTY: f64 = 1.0;

/// Joins chunk transcripts, keeping the overlap of every two neighbouring chunks once even
/// when the two chunks transcribed it differently.
pub fn merge_chunk_texts(texts: Vec<String>) -> String {
    let mut merged: Vec<&str> = Vec::new();
    for text in &texts {
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
struct Seam {
    merged_until: usize,
    next_from: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    Stop,
    Diagonal,
    Up,
    Left,
}

/// Aligns the end of `merged` with the start of `next` (local alignment of normalized
/// words). The seam goes through the middle of the aligned overlap, where both chunks heard
/// the words with context on both sides; the words near either chunk edge are dropped.
fn find_seam(merged: &[&str], next: &[&str]) -> Option<Seam> {
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

    let (mut i, mut j) = (n.saturating_sub(TAIL_EDGE_SLACK).max(1)..=n)
        .flat_map(|i| (1..=m).map(move |j| (i, j)))
        .filter(|&(i, j)| score[i][j] > 0.0)
        .max_by(|&(a, b), &(c, d)| score[a][b].total_cmp(&score[c][d]))?;
    let mut pairs = Vec::new();
    while i > 0 && j > 0 {
        match step[i][j] {
            Step::Diagonal => {
                let s = similar(i, j);
                if s >= WORD_MATCH_SIMILARITY {
                    pairs.push((i, j, s));
                }
                i -= 1;
                j -= 1;
            }
            Step::Up => i -= 1,
            Step::Left => j -= 1,
            Step::Stop => break,
        }
    }
    pairs.reverse();
    // The matched words must reach the edges themselves, not only through gaps: near the
    // end of the text so far and near the start of the next chunk.
    let (Some(&(last_i, _, _)), Some(&(_, first_j, _))) = (pairs.last(), pairs.first()) else {
        return None;
    };
    if n - last_i > TAIL_EDGE_SLACK || first_j - 1 > HEAD_EDGE_SLACK {
        return None;
    }
    let same = |&&(_, _, s): &&(usize, usize, f64)| s == 1.0;
    let trusted = match pairs.as_slice() {
        [] => false,
        // A lone word counts only as the same word at both very edges.
        [(i, j, s)] => *s == 1.0 && *i == n && *j == 1,
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
    Some(Seam {
        merged_until: tail_start + i,
        next_from: j,
    })
}

/// Lowercase, `ё` read as `е`, every non-alphanumeric character dropped.
fn normalize(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// 1.0 for the same word; [`CUT_SHORT_SIMILARITY`] when one is the other cut short and
/// `at_edge` allows that; for words of [`FUZZY_MIN_CHARS`] or more, 1 − edit distance /
/// length of the longer word; otherwise 0.
fn similarity(a: &str, b: &str, at_edge: bool) -> f64 {
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

fn levenshtein(a: &[char], b: &[char]) -> usize {
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

/// Length of the frames the silence search measures.
const QUIET_FRAME_MS: usize = 20;

/// Where the chunk that started after `last_cut` ends: the start of the latest quiet frame
/// within the lookback before the hard boundary (`chunk_threshold_secs` after `last_cut`),
/// or the hard boundary when there is none.
pub fn next_cut(samples: &[i16], sample_rate: u32, last_cut: usize, cfg: ChunkingCfg) -> usize {
    let rate = sample_rate as usize;
    let hard_boundary = (last_cut + cfg.chunk_threshold_secs as usize * rate).min(samples.len());
    let lookback_start = hard_boundary.saturating_sub(cfg.vad_lookback_secs as usize * rate);
    let frame = rate * QUIET_FRAME_MS / 1000;
    if frame == 0 {
        return hard_boundary;
    }
    let quiet = |start: usize| {
        let samples = &samples[start..(start + frame).min(samples.len())];
        let mean_square = samples
            .iter()
            .map(|&s| {
                let f = s as f32 / 32_768.0;
                f * f
            })
            .sum::<f32>()
            / samples.len() as f32;
        !samples.is_empty() && mean_square.sqrt() < cfg.vad_rms_threshold
    };
    (lookback_start / frame..hard_boundary / frame)
        .rev()
        .map(|i| i * frame)
        .find(|&start| quiet(start))
        .unwrap_or(hard_boundary)
}

/// Splits a whole recording the way a live dictation is split: cut with [`next_cut`] every
/// time a chunk threshold of audio is available, and start every later chunk the overlap
/// before the previous cut.
pub fn split_into_chunks(audio: &AudioBuffer16k, cfg: ChunkingCfg) -> Vec<AudioBuffer16k> {
    let samples = audio.samples();
    let rate = AudioBuffer16k::SAMPLE_RATE_HZ as usize;
    let chunk_len = cfg.chunk_threshold_secs as usize * rate;
    if chunk_len == 0 || samples.len() <= chunk_len {
        return vec![audio.clone()];
    }
    let overlap = cfg.chunk_overlap_secs as usize * rate;
    let mut chunks = Vec::new();
    let (mut last_cut, mut start) = (0, 0);
    while samples.len() - last_cut >= chunk_len {
        let cut = next_cut(samples, AudioBuffer16k::SAMPLE_RATE_HZ, last_cut, cfg);
        chunks.push(AudioBuffer16k::new(samples[start..cut].to_vec()));
        last_cut = cut;
        start = cut.saturating_sub(overlap);
    }
    if start < samples.len() {
        chunks.push(AudioBuffer16k::new(samples[start..].to_vec()));
    }
    chunks
}

#[cfg(test)]
#[path = "chunking_seam_cases.rs"]
mod seam_cases;

#[cfg(test)]
mod tests {
    use super::*;

    fn merge_two(tail: &str, head: &str) -> String {
        merge_chunk_texts(vec![tail.to_owned(), head.to_owned()])
    }

    #[test]
    fn should_merge_every_recorded_seam_as_reviewed() {
        let failures: Vec<String> = super::seam_cases::SEAM_CASES
            .iter()
            .filter_map(|case| {
                let merged = merge_two(case.tail, case.head);
                (merged != case.merged).then(|| {
                    format!(
                        "{}\n    tail:     {}\n    head:     {}\n    expected: {}\n    merged:   {}",
                        case.recording, case.tail, case.head, case.merged, merged
                    )
                })
            })
            .collect();
        assert!(
            failures.is_empty(),
            "{} of {} recorded seams merged wrongly:\n{}",
            failures.len(),
            super::seam_cases::SEAM_CASES.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn should_keep_overlap_once_when_punctuation_and_case_differ() {
        let merged = merge_two(
            "сущность, у которой миллион уровней, миллион граней.",
            "Миллион граней, и мы на эти грани можем смотреть",
        );
        assert_eq!(
            merged,
            "сущность, у которой миллион уровней, миллион граней, и мы на эти грани можем смотреть"
        );
    }

    #[test]
    fn should_keep_overlap_once_when_yo_is_written_as_ye() {
        let merged = merge_two(
            "иметь понятный интерфейс. Ещё нужно написать",
            "Еще нужно написать тесты на возврат денег",
        );
        assert_eq!(
            merged,
            "иметь понятный интерфейс. Ещё нужно написать тесты на возврат денег"
        );
    }

    #[test]
    fn should_keep_overlap_once_when_a_word_is_hyphenated_in_one_chunk() {
        let merged = merge_two(
            "просто к обычным раунд-робинам тебе один запрос кидает,",
            "обычным раундробином тебе один запрос кидает на одну ноду",
        );
        assert_eq!(
            merged,
            "просто к обычным раунд-робинам тебе один запрос кидает на одну ноду"
        );
    }

    #[test]
    fn should_keep_the_whole_word_when_one_chunk_cut_it_short() {
        let merged = merge_two(
            "чтобы на нём такие. Но своего рода чека,",
            "Но своего рода чекап, это как нагрузочное тестирование",
        );
        assert_eq!(
            merged,
            "чтобы на нём такие. Но своего рода чекап, это как нагрузочное тестирование"
        );
    }

    #[test]
    fn should_keep_overlap_once_when_chunks_read_a_word_differently() {
        let merged = merge_two(
            "прохожу 300 тысяч шагов по 30-30 дней.",
            "шагов по 30 километров в день пробегаю, и вот",
        );
        assert_eq!(
            merged,
            "прохожу 300 тысяч шагов по 30 километров в день пробегаю, и вот"
        );
    }

    #[test]
    fn should_keep_overlap_once_when_the_first_chunk_misheard_its_last_number() {
        let merged = merge_two(
            "and the response time dropped from 2 seconds to 4.",
            "dropped from 2 seconds to 40 milliseconds. In the evening",
        );
        assert_eq!(
            merged,
            "and the response time dropped from 2 seconds to 40 milliseconds. In the evening"
        );
    }

    #[test]
    fn should_keep_a_lone_word_once_when_it_ends_one_chunk_and_starts_the_next() {
        let merged = merge_two(
            "have a clear interface. face. We",
            "We also need tests for refunds",
        );
        assert_eq!(
            merged,
            "have a clear interface. face. We also need tests for refunds"
        );
    }

    #[test]
    fn should_not_trust_a_lone_similar_word() {
        let merged = merge_two(
            "на несколько маленьких частей.",
            "Каждую часть должна отвечать только за одну вещь",
        );
        assert_eq!(
            merged,
            "на несколько маленьких частей. Каждую часть должна отвечать только за одну вещь"
        );
    }

    #[test]
    fn should_keep_a_phrase_repeated_outside_the_overlap() {
        let merged = merge_two(
            "I said yes and then we left the building quietly",
            "Next morning I said yes again to the offer",
        );
        assert_eq!(
            merged,
            "I said yes and then we left the building quietly Next morning I said yes again to the offer"
        );
    }

    #[test]
    fn should_keep_every_word_when_chunks_share_none() {
        let merged = merge_two(
            "it was a long day at the office.",
            "Tomorrow we start early and review the plan.",
        );
        assert_eq!(
            merged,
            "it was a long day at the office. Tomorrow we start early and review the plan."
        );
    }

    #[test]
    fn should_keep_overlap_once_across_three_differing_chunks() {
        let merged = merge_chunk_texts(vec![
            "Сегодня утром я проверил очередь задач и понял, что половина".to_owned(),
            "что половина из них устарела. Мы договорились с командой".to_owned(),
            "Мы договорились с командой, что будем пересматривать список".to_owned(),
        ]);
        assert_eq!(
            merged,
            "Сегодня утром я проверил очередь задач и понял, что половина из них устарела. Мы договорились с командой, что будем пересматривать список"
        );
    }

    const RATE: u32 = 16_000;

    fn cfg(threshold_secs: u32) -> ChunkingCfg {
        ChunkingCfg {
            chunk_threshold_secs: threshold_secs,
            ..ChunkingCfg::default()
        }
    }

    fn tone(secs: usize) -> Vec<i16> {
        vec![10_000i16; secs * RATE as usize]
    }

    #[test]
    fn should_cut_at_hard_boundary_when_no_silence_found() {
        let samples = tone(40);
        let cut = next_cut(&samples, RATE, 0, cfg(30));
        assert_eq!(cut, 30 * RATE as usize);
    }

    #[test]
    fn should_cut_at_latest_quiet_frame_within_lookback() {
        let mut samples = tone(40);
        let quiet_from = 27 * RATE as usize;
        let quiet_to = quiet_from + 640;
        samples[quiet_from..quiet_to].fill(0);
        let cut = next_cut(&samples, RATE, 0, cfg(30));
        assert_eq!(cut, quiet_from + 320);
    }

    #[test]
    fn should_ignore_silence_before_the_lookback() {
        let mut samples = tone(40);
        samples[20 * RATE as usize..21 * RATE as usize].fill(0);
        let cut = next_cut(&samples, RATE, 0, cfg(30));
        assert_eq!(cut, 30 * RATE as usize);
    }

    #[test]
    fn should_keep_a_recording_up_to_the_threshold_in_one_chunk() {
        let audio = AudioBuffer16k::new(tone(30));
        let chunks = split_into_chunks(&audio, cfg(30));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].samples().len(), audio.samples().len());
    }

    #[test]
    fn should_keep_a_recording_in_one_chunk_when_chunking_is_off() {
        let audio = AudioBuffer16k::new(tone(70));
        assert_eq!(split_into_chunks(&audio, cfg(0)).len(), 1);
    }

    #[test]
    fn should_split_a_long_recording_into_overlapping_chunks() {
        let rate = RATE as usize;
        let audio = AudioBuffer16k::new(tone(70));
        let chunks = split_into_chunks(&audio, cfg(30));
        let lens: Vec<usize> = chunks.iter().map(|c| c.samples().len()).collect();
        // cuts at 30 s and 60 s; every chunk after the first starts 2 s before the cut
        assert_eq!(lens, vec![30 * rate, 32 * rate, 12 * rate]);
    }

    #[test]
    fn should_split_at_a_silence_inside_the_lookback() {
        let rate = RATE as usize;
        let mut samples = tone(50);
        samples[28 * rate..28 * rate + 320].fill(0);
        let chunks = split_into_chunks(&AudioBuffer16k::new(samples), cfg(30));
        let lens: Vec<usize> = chunks.iter().map(|c| c.samples().len()).collect();
        assert_eq!(lens, vec![28 * rate, 24 * rate]);
    }

    #[test]
    fn should_return_single_chunk_unchanged() {
        let texts = vec!["hello world foo".to_string()];
        let merged = merge_chunk_texts(texts);
        assert_eq!(merged, "hello world foo");
    }

    #[test]
    fn should_drop_only_actual_overlap_words() {
        let texts = vec![
            "alpha beta gamma".to_string(),
            "gamma delta epsilon zeta".to_string(),
        ];
        let merged = merge_chunk_texts(texts);
        assert_eq!(merged, "alpha beta gamma delta epsilon zeta");
    }

    #[test]
    fn should_produce_empty_string_from_all_empty_chunks() {
        let texts = vec![String::new(), String::new(), String::new()];
        let merged = merge_chunk_texts(texts);
        assert_eq!(merged, "");
    }

    #[test]
    fn should_keep_all_words_from_first_chunk() {
        let texts = vec![
            "one two three four five".to_string(),
            "five six seven".to_string(),
        ];
        let merged = merge_chunk_texts(texts);
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
        let merged = merge_chunk_texts(texts);
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
        let merged = merge_chunk_texts(texts);
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
        let merged = merge_chunk_texts(texts);
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
        let merged = merge_chunk_texts(texts);
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
        let merged = merge_chunk_texts(texts);
        assert_eq!(merged, "hello world foo bar baz");
    }

    #[test]
    fn should_handle_identical_chunks() {
        let texts = vec![
            "alpha beta gamma".to_string(),
            "alpha beta gamma".to_string(),
        ];
        let merged = merge_chunk_texts(texts);
        assert_eq!(merged, "alpha beta gamma");
    }

    #[test]
    fn should_handle_inner_empty_chunks() {
        let texts = vec![
            "first chunk text".to_string(),
            String::new(),
            "third chunk here".to_string(),
        ];
        let merged = merge_chunk_texts(texts);
        assert!(merged.starts_with("first chunk text"));
        assert!(merged.ends_with("third chunk here"));
    }
}
