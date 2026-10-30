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
            ("т", Maybe),
        ]],
    ),
    (
        "start",
        &[
            &[
                ("с", Once),
                ("т", Once),
                ("а", Once),
                ("р", Once),
                ("т", Once),
            ],
            &[("с", Once), ("т", Once), ("а", Once), ("т", Once)],
            &[("с", Once), ("т", Once), ("а", Once), ("р", Once)],
        ],
    ),
    (
        "stop",
        &[&[("с", Once), ("т", Once), ("оа", Once), ("п", Once)]],
    ),
    (
        "enter",
        &[&[
            ("эеи", Once),
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
            ("иеэа", Maybe),
            ("р", Once),
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
        &[&[
            ("п", Once),
            ("еэ", Once),
            ("й", Maybe),
            ("с", Once),
            ("т", Once),
        ]],
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

/// The English words a Cyrillic word sounds like; none when part of it is no command sound.
pub fn respell_word(word: &str) -> Option<Vec<&'static str>> {
    let letters: Vec<char> = word.to_lowercase().replace('ё', "е").chars().collect();
    let mut rest = letters.as_slice();
    let mut words = Vec::new();
    while !rest.is_empty() {
        let (english, used) = SOUNDS.iter().find_map(|(english, ways)| {
            ways.iter()
                .find_map(|way| said(way, rest))
                .map(|used| (*english, used))
        })?;
        words.push(english);
        rest = &rest[used..];
    }
    (!words.is_empty()).then_some(words)
}

/// How many of the letters at the start of `word` say `way`, each letter taken greedily.
fn said(way: &[Letter], word: &[char]) -> Option<usize> {
    let mut at = 0;
    for &(letters, times) in way {
        at = taken(letters, times, word, at)?;
    }
    (at > 0).then_some(at)
}

/// Where `word` goes on after `letters`, said `times` from `at`; none when they are not there.
fn taken(letters: &str, times: Times, word: &[char], at: usize) -> Option<usize> {
    let run = word[at..]
        .iter()
        .take_while(|c| letters.contains(**c))
        .count();
    match times {
        Once => (run > 0).then_some(at + 1),
        Maybe => Some(at + run.min(1)),
        Many => (run > 0).then_some(at + run),
        Rest => Some(word.len()),
    }
}

#[cfg(test)]
mod tests;
