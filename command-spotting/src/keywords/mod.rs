//! The phrases the service can spot, as the keyword spotter needs them.

use std::fmt;

/// `keywords.txt`: one phrase per line, `#` starts a comment line.
pub const KEYWORDS: &str = include_str!("../../keywords.txt");

const COMMENT: char = '#';
const PHRASE_MARKER: &str = " @";
/// Joins the words of a phrase in the keyword spotter's output, which splits on spaces.
const WORD_JOINER: char = '_';
/// Separates keywords in a per-stream keyword list of sherpa-onnx.
const STREAM_SEPARATOR: &str = "/";
/// Trigger-only keywords, with the phrase each serves: a short "start" or "restart" said in a
/// running utterance only asks a check of it; the check decides whether a command was said.
const TRIGGERS: &[(&str, &str)] = &[
    ("start trigger", "start dictation"),
    ("restart trigger", "restart dictation"),
];

/// Whether `phrase` names a trigger-only keyword, which is never reported.
pub fn is_trigger(phrase: &str) -> bool {
    served_by(phrase).is_some()
}

/// The phrase the trigger-only keyword `trigger` serves.
pub fn served_by(trigger: &str) -> Option<&'static str> {
    TRIGGERS
        .iter()
        .find(|(t, _)| *t == trigger)
        .map(|(_, phrase)| *phrase)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keyword {
    pub phrase: String,
    /// The whole line: tokens, boost, threshold and phrase.
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPhrases(pub Vec<String>);

impl fmt::Display for UnknownPhrases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown phrases: {}", self.0.join(", "))
    }
}

impl std::error::Error for UnknownPhrases {}

pub fn parse(text: &str) -> Vec<Keyword> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(COMMENT))
        .filter_map(|line| {
            let (_, phrase) = line.split_once(PHRASE_MARKER)?;
            Some(Keyword { phrase: phrase_of(phrase.trim()), line: line.to_owned() })
        })
        .collect()
}

/// The phrase a keyword spotter result names.
pub fn phrase_of(spotted: &str) -> String {
    spotted.replace(WORD_JOINER, " ")
}

/// Every keyword line but the triggers, one per line, for creating the keyword spotter: the
/// spotter adds these to the keywords of each stream, and a trigger must never reach the
/// stream of the phrases.
pub fn all_lines(keywords: &[Keyword]) -> String {
    keywords
        .iter()
        .filter(|k| !is_trigger(&k.phrase))
        .map(|k| k.line.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The per-stream keyword list for `phrases`, or the phrases this service does not know.
pub fn select(keywords: &[Keyword], phrases: &[String]) -> Result<String, UnknownPhrases> {
    let unknown: Vec<String> = phrases
        .iter()
        .filter(|p| !keywords.iter().any(|k| &k.phrase == *p))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(UnknownPhrases(unknown));
    }
    let lines: Vec<&str> = keywords
        .iter()
        .filter(|k| phrases.contains(&k.phrase))
        .map(|k| k.line.as_str())
        .collect();
    Ok(lines.join(STREAM_SEPARATOR))
}

/// The per-stream keyword list of the trigger-only keywords that serve `phrases`; none when
/// no selected phrase has one.
pub fn select_triggers(keywords: &[Keyword], phrases: &[String]) -> Option<String> {
    let served = |trigger: &str| {
        TRIGGERS
            .iter()
            .any(|(t, phrase)| *t == trigger && phrases.iter().any(|p| p == phrase))
    };
    let lines: Vec<&str> = keywords
        .iter()
        .filter(|k| served(&k.phrase))
        .map(|k| k.line.as_str())
        .collect();
    (!lines.is_empty()).then(|| lines.join(STREAM_SEPARATOR))
}

#[cfg(test)]
mod tests;
