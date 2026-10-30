//! What the speech recognition service made of what the keyword spotter spotted.

use serde::Serialize;

/// How an utterance's reported commands compare with the phrases spotted in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Every command reported was spotted.
    Agreed,
    /// A command was reported that was not spotted, beside spotted ones.
    Corrected,
    /// Phrases were spotted and no command was reported.
    Rejected,
    /// A command was reported and nothing was spotted.
    Missed,
    /// Neither: an utterance of the streaming recognizer kept without a command.
    None,
}

/// The outcome of an utterance with `spotted` phrases and `reported` commands; none when it
/// has neither.
pub fn outcome(spotted: &[&str], reported: &[&str]) -> Option<Outcome> {
    match (spotted.is_empty(), reported.is_empty()) {
        (true, true) => None,
        (true, false) => Some(Outcome::Missed),
        (false, true) => Some(Outcome::Rejected),
        (false, false) if reported.iter().all(|c| spotted.contains(c)) => Some(Outcome::Agreed),
        (false, false) => Some(Outcome::Corrected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_agree_when_every_reported_command_was_spotted() {
        // When
        let outcome = outcome(&["press enter"], &["press enter"]);
        // Then
        assert_eq!(outcome, Some(Outcome::Agreed));
    }

    #[test]
    fn should_be_corrected_when_another_command_was_reported() {
        // When
        let outcome = outcome(&["press enter"], &["paste text"]);
        // Then
        assert_eq!(outcome, Some(Outcome::Corrected));
    }

    #[test]
    fn should_be_rejected_when_a_spotted_phrase_reported_nothing() {
        // When
        let outcome = outcome(&["paste text"], &[]);
        // Then
        assert_eq!(outcome, Some(Outcome::Rejected));
    }

    #[test]
    fn should_be_missed_when_a_command_was_reported_without_a_spotted_phrase() {
        // When
        let outcome = outcome(&[], &["stop dictation"]);
        // Then
        assert_eq!(outcome, Some(Outcome::Missed));
    }

    #[test]
    fn should_have_no_outcome_without_a_spotted_phrase_or_a_command() {
        // When
        let outcome = outcome(&[], &[]);
        // Then
        assert_eq!(outcome, None);
    }
}
