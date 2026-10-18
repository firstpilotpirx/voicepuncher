//! Finds dictated Cyrillic words that a normalization respelled as Latin words that sound
//! alike (фидбек → feedback, о май зш → oh-my-zsh, вс код → VS Code).

/// Most words on either side of one respelling (пауэрлевел десять к → powerlevel10k).
const MAX_GROUP_WORDS: usize = 3;
/// Least similarity of two sound skeletons, 1 − edit distance / longer skeleton, for a
/// respelling (автоматик → automatic: 0.8; до вечера → by evening: 0.4).
const MIN_SIMILARITY: f64 = 0.6;

/// Sound classes shared by Cyrillic and Latin letters; vowels and silent letters have none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sound {
    P,
    F,
    T,
    K,
    S,
    Sh,
    L,
    R,
    M,
    N,
}

/// Which `cyrillic` and `latin` words (lowercase) are respellings of each other: one
/// flag per word, set when the word is part of a respelling. Both lists keep their order,
/// and respellings never cross: the first Cyrillic word matches before the second.
pub fn find_respellings(cyrillic: &[String], latin: &[String]) -> (Vec<bool>, Vec<bool>) {
    let (cyr, lat): (Vec<Vec<Sound>>, Vec<Vec<Sound>>) = (
        cyrillic.iter().map(|w| sounds(w)).collect(),
        latin.iter().map(|w| sounds(w)).collect(),
    );
    // best[i][j]: most Cyrillic words respelled using cyr[i..] and lat[j..]
    let mut best = vec![vec![0_usize; lat.len() + 1]; cyr.len() + 1];
    let mut step = vec![vec![Step::SkipBoth; lat.len() + 1]; cyr.len() + 1];
    for i in (0..=cyr.len()).rev() {
        for j in (0..=lat.len()).rev() {
            let mut choice = (0, Step::SkipBoth);
            if i < cyr.len() {
                choice = choice.max_by(best[i + 1][j], Step::SkipCyrillic);
            }
            if j < lat.len() {
                choice = choice.max_by(best[i][j + 1], Step::SkipLatin);
            }
            for a in 1..=MAX_GROUP_WORDS.min(cyr.len() - i) {
                for b in 1..=MAX_GROUP_WORDS.min(lat.len() - j) {
                    if respells(&cyr[i..i + a], &lat[j..j + b]) {
                        choice = choice.max_by(best[i + a][j + b] + a, Step::Match(a, b));
                    }
                }
            }
            (best[i][j], step[i][j]) = choice;
        }
    }
    let (mut cyr_flags, mut lat_flags) = (vec![false; cyr.len()], vec![false; lat.len()]);
    let (mut i, mut j) = (0, 0);
    while i < cyr.len() || j < lat.len() {
        match step[i][j] {
            Step::Match(a, b) => {
                cyr_flags[i..i + a].fill(true);
                lat_flags[j..j + b].fill(true);
                (i, j) = (i + a, j + b);
            }
            Step::SkipCyrillic => i += 1,
            Step::SkipLatin => j += 1,
            Step::SkipBoth => break,
        }
    }
    (cyr_flags, lat_flags)
}

#[derive(Debug, Clone, Copy)]
enum Step {
    Match(usize, usize),
    SkipCyrillic,
    SkipLatin,
    SkipBoth,
}

trait MaxBy {
    fn max_by(self, score: usize, step: Step) -> Self;
}

impl MaxBy for (usize, Step) {
    fn max_by(self, score: usize, step: Step) -> Self {
        if score > self.0 {
            (score, step)
        } else {
            self
        }
    }
}

/// The groups sound alike and none of their edge words is needed for that, so a short
/// word next to a respelling (мне фидбек → me feedback) is not taken for one.
fn respells(cyrillic: &[Vec<Sound>], latin: &[Vec<Sound>]) -> bool {
    let shrunk = |words: &[Vec<Sound>]| -> Vec<std::ops::Range<usize>> {
        if words.len() > 1 {
            vec![1..words.len(), 0..words.len() - 1]
        } else {
            Vec::new()
        }
    };
    sound_alike(cyrillic, latin)
        && !shrunk(cyrillic)
            .into_iter()
            .any(|range| sound_alike(&cyrillic[range], latin))
        && !shrunk(latin)
            .into_iter()
            .any(|range| sound_alike(cyrillic, &latin[range]))
}

fn sound_alike(cyrillic: &[Vec<Sound>], latin: &[Vec<Sound>]) -> bool {
    let joined = |words: &[Vec<Sound>]| collapse(words.iter().flatten().copied());
    let (a, b) = (joined(cyrillic), joined(latin));
    let longer = a.len().max(b.len());
    longer > 0 && 1.0 - edit_distance(&a, &b) as f64 / longer as f64 >= MIN_SIMILARITY
}

/// The word's consonant sounds, a repeated sound kept once (пулл → P L).
fn sounds(word: &str) -> Vec<Sound> {
    let chars: Vec<char> = word.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let next = chars.get(i + 1).copied();
        let (sound, len): (&[Sound], usize) = match (chars[i], next) {
            ('s' | 'c' | 'z', Some('h')) => (&[Sound::Sh], 2),
            ('c', Some('k')) => (&[Sound::K], 2),
            ('p', Some('h')) => (&[Sound::F], 2),
            ('t', Some('h')) => (&[Sound::T], 2),
            (c, _) => (letter_sounds(c), 1),
        };
        out.extend_from_slice(sound);
        i += len;
    }
    collapse(out.into_iter())
}

fn letter_sounds(c: char) -> &'static [Sound] {
    match c {
        'б' | 'п' | 'b' | 'p' => &[Sound::P],
        'в' | 'ф' | 'v' | 'f' => &[Sound::F],
        'д' | 'т' | 'd' | 't' => &[Sound::T],
        'г' | 'к' | 'g' | 'k' | 'c' | 'q' => &[Sound::K],
        'з' | 'с' | 'ц' | 'z' | 's' => &[Sound::S],
        'ж' | 'ш' | 'щ' | 'ч' => &[Sound::Sh],
        'j' => &[Sound::T, Sound::Sh],
        'x' => &[Sound::K, Sound::S],
        'л' | 'l' => &[Sound::L],
        'р' | 'r' => &[Sound::R],
        'м' | 'm' => &[Sound::M],
        'н' | 'n' => &[Sound::N],
        _ => &[],
    }
}

fn collapse(sounds: impl Iterator<Item = Sound>) -> Vec<Sound> {
    let mut out: Vec<Sound> = Vec::new();
    for sound in sounds {
        if out.last() != Some(&sound) {
            out.push(sound);
        }
    }
    out
}

fn edit_distance(a: &[Sound], b: &[Sound]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(x != y))
                .min(row[j] + 1)
                .min(above + 1);
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn should_match_each_english_word_with_its_cyrillic_spelling() {
        // Given
        let cyrillic = words("фидбек пулл реквесту");
        let latin = words("feedback pull request");
        // When
        let (cyr, lat) = find_respellings(&cyrillic, &latin);
        // Then
        assert_eq!(cyr, [true, true, true]);
        assert_eq!(lat, [true, true, true]);
    }

    #[test]
    fn should_match_syllables_with_the_words_of_a_term() {
        // Given
        let cyrillic = words("вс код о май зш");
        let latin = words("vs code oh my zsh");
        // When
        let (cyr, lat) = find_respellings(&cyrillic, &latin);
        // Then
        assert_eq!(cyr, [true, true, false, true, true]);
        assert_eq!(lat, [true, true, false, true, true]);
    }

    #[test]
    fn should_leave_spoken_digits_out_of_a_respelling() {
        // Given
        let cyrillic = words("пауэрлевел десять к");
        let latin = words("powerlevel10k");
        // When
        let (cyr, lat) = find_respellings(&cyrillic, &latin);
        // Then
        assert_eq!(cyr, [true, false, false]);
        assert_eq!(lat, [true]);
    }

    #[test]
    fn should_not_take_a_short_word_next_to_a_respelling_for_part_of_it() {
        // Given
        let cyrillic = words("мне фидбек");
        let latin = words("me feedback");
        // When
        let (cyr, lat) = find_respellings(&cyrillic, &latin);
        // Then
        assert_eq!(cyr, [false, true]);
        assert_eq!(lat, [false, true]);
    }

    #[test]
    fn should_not_match_a_translation() {
        // Given
        let cyrillic = words("сделай этого до вечера");
        let latin = words("make a of this before bedtime");
        // When
        let (cyr, _) = find_respellings(&cyrillic, &latin);
        // Then
        assert_eq!(cyr, [false, false, false, false]);
    }
}
