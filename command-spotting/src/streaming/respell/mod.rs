//! The user's English command words as the Russian recognizer writes them ("стоп", "энтер").
//!
//! They are respelled in English so the phrase matcher reads them like the English
//! recognizer's. The patterns are those of the bench (`research/bench/common.py`, `RU_CMD` and `RU_FILL`).

/// How often a letter of a pattern is said.
#[derive(Clone, Copy)]
enum Times {
    Once,
    Maybe,
    /// Once or more.
    Many,
    /// Any letters to the end of the word ("диктейшн", "диктэйшен").
    Rest,
}

/// One letter of a pattern: any of `letters`, said `times`.
type Letter = (&'static str, Times);

use Times::{Many, Maybe, Once, Rest};

/// Each command and form word with the ways it is written, tried in this order: "restart"
/// before "start", the commands before the other words.
const SOUNDS: &[(&str, &[&[Letter]])] = &[
    (
        "restart",
        &[&[
            ("р", Once),
            ("еи", Once),
            ("с", Once),
            ("т", Once),
            ("а", Once),
            ("р", Maybe),
            ("тд", Maybe),
        ]],
    ),
    (
        "start",
        &[
            &[
                ("с", Maybe),
                ("т", Once),
                ("а", Once),
                ("р", Once),
                ("тд", Maybe),
            ],
            &[("с", Once), ("т", Once), ("а", Once), ("т", Once)],
        ],
    ),
    (
        "stop",
        &[&[("с", Once), ("т", Once), ("оа", Once), ("п", Once)]],
    ),
    (
        "enter",
        &[&[
            ("л", Maybe),
            ("эеия", Once),
            ("н", Once),
            ("тд", Once),
            ("еэоа", Maybe),
            ("р", Once),
        ]],
    ),
    (
        "clear",
        &[&[
            ("к", Once),
            ("л", Once),
            ("иеэ", Once),
            ("иеэао", Maybe),
            ("рт", Maybe),
        ]],
    ),
    (
        "cancel",
        &[&[
            ("к", Once),
            ("эеа", Once),
            ("н", Maybe),
            ("с", Once),
            ("еэ", Maybe),
            ("л", Once),
        ]],
    ),
    (
        "paste",
        &[
            &[
                ("п", Once),
                ("еэ", Once),
                ("й", Maybe),
                ("с", Once),
                ("т", Once),
            ],
            &[
                ("п", Once),
                ("о", Once),
                ("ий", Once),
                ("с", Once),
                ("т", Once),
            ],
        ],
    ),
    (
        "dictation",
        &[&[
            ("д", Once),
            ("и", Once),
            ("к", Once),
            ("т", Once),
            ("", Rest),
        ]],
    ),
    (
        "please",
        &[
            &[("п", Once), ("л", Once), ("и", Once), ("зс", Once)],
            &[
                ("п", Once),
                ("л", Once),
                ("е", Once),
                ("й", Once),
                ("с", Once),
            ],
        ],
    ),
    (
        "press",
        &[&[
            ("п", Once),
            ("р", Once),
            ("е", Once),
            ("с", Once),
            ("с", Maybe),
        ]],
    ),
    (
        "text",
        &[&[
            ("т", Once),
            ("еэ", Once),
            ("к", Once),
            ("с", Maybe),
            ("т", Once),
        ]],
    ),
    ("all", &[&[("о", Once), ("л", Many)]]),
];

/// The text with every Cyrillic word that is made only of command sounds respelled in English.
pub fn respell(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_alphabetic() {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            out.push_str(&respelled_or_kept(&word));
            word.clear();
        }
        out.push(c);
    }
    out.pop();
    out
}

fn respelled_or_kept(word: &str) -> String {
    if !word.chars().any(is_cyrillic) {
        return word.to_owned();
    }
    respell_word(word).map_or_else(|| word.to_owned(), |words| words.join(" "))
}

fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{04ff}').contains(&c)
}

/// Russian words that are a command word only right before "please": «сто плиз» is "stop
/// please", while «сто» alone is a hundred.
const BEFORE_PLEASE: &[(&str, &str)] = &[("сто", "stop")];

/// The command word `word` is when "please" follows it.
pub fn before_please(word: &str) -> Option<&'static str> {
    let word = word.to_lowercase();
    BEFORE_PLEASE
        .iter()
        .find(|(ru, _)| *ru == word)
        .map(|(_, en)| *en)
}

/// The English words a Cyrillic word sounds like; none when part of it is no command sound.
pub fn respell_word(word: &str) -> Option<Vec<&'static str>> {
    let letters: Vec<char> = word.to_lowercase().replace('ё', "е").chars().collect();
    (!letters.is_empty()).then(|| split(&letters)).flatten()
}

/// `letters` read as command sounds one after another; a sound that leaves letters no other
/// sound reads is tried shorter, so "тартарт" is "тар" and "тарт".
fn split(letters: &[char]) -> Option<Vec<&'static str>> {
    if letters.is_empty() {
        return Some(Vec::new());
    }
    SOUNDS.iter().find_map(|(english, ways)| {
        ways.iter().find_map(|way| {
            said(way, letters).into_iter().rev().find_map(|used| {
                let mut words = split(&letters[used..])?;
                words.insert(0, *english);
                Some(words)
            })
        })
    })
}

/// How many of the letters at the start of `word` can say `way`, shortest first.
fn said(way: &[Letter], word: &[char]) -> Vec<usize> {
    let mut ends = vec![0];
    for &(letters, times) in way {
        ends = ends
            .iter()
            .flat_map(|&at| taken(letters, times, word, at))
            .collect();
        ends.sort_unstable();
        ends.dedup();
    }
    ends.retain(|&at| at > 0);
    ends
}

/// Where `word` can go on after `letters`, said `times` from `at`; none when they are not there.
fn taken(letters: &str, times: Times, word: &[char], at: usize) -> Vec<usize> {
    let run = word[at..]
        .iter()
        .take_while(|c| letters.contains(**c))
        .count();
    match times {
        Once => (run > 0).then_some(vec![at + 1]).unwrap_or_default(),
        Maybe => (at..=at + run.min(1)).collect(),
        Many => (at + 1..=at + run).collect(),
        Rest => vec![word.len()],
    }
}

#[cfg(test)]
mod tests;
