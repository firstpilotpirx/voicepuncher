//! The phrases the service can spot, as the keyword spotter needs them.

use std::fmt;

/// `keywords.txt`: one phrase per line, `#` starts a comment line.
pub const KEYWORDS: &str = include_str!("../keywords.txt");

const COMMENT: char = '#';
const PHRASE_MARKER: &str = " @";
/// Joins the words of a phrase in the keyword spotter's output, which splits on spaces.
const WORD_JOINER: char = '_';
/// Separates keywords in a per-stream keyword list of sherpa-onnx.
const STREAM_SEPARATOR: &str = "/";

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
            Some(Keyword {
                phrase: phrase_of(phrase.trim()),
                line: line.to_owned(),
            })
        })
        .collect()
}

/// The phrase a keyword spotter result names.
pub fn phrase_of(spotted: &str) -> String {
    spotted.replace(WORD_JOINER, " ")
}

/// Every keyword line, one per line, for creating the keyword spotter.
pub fn all_lines(keywords: &[Keyword]) -> String {
    keywords
        .iter()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn phrases(list: &[&str]) -> Vec<String> {
        list.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn should_parse_the_five_phrases_of_voice_control() {
        // When
        let keywords = parse(KEYWORDS);
        // Then
        let mut names: Vec<&str> = keywords.iter().map(|k| k.phrase.as_str()).collect();
        names.dedup();
        assert_eq!(
            names,
            [
                "start dictation",
                "stop dictation",
                "cancel dictation",
                "press enter",
                "clear all text"
            ]
        );
    }

    #[test]
    fn should_select_only_the_requested_phrases() {
        // Given
        let keywords = parse(KEYWORDS);
        // When
        let selected = select(&keywords, &phrases(&["press enter", "stop dictation"])).unwrap();
        // Then
        assert!(selected.contains("@stop_dictation"));
        assert!(selected.contains("@press_enter"));
        assert!(!selected.contains("@start_dictation"));
    }

    #[test]
    fn should_name_a_spotted_keyword_by_its_phrase() {
        assert_eq!(phrase_of("clear_all_text"), "clear all text");
    }

    #[test]
    fn should_reject_an_unknown_phrase_by_name() {
        // Given
        let keywords = parse(KEYWORDS);
        // When
        let error = select(&keywords, &phrases(&["press enter", "open the door"])).unwrap_err();
        // Then
        assert_eq!(error, UnknownPhrases(phrases(&["open the door"])));
        assert_eq!(error.to_string(), "unknown phrases: open the door");
    }
}
