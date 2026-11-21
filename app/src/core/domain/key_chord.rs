//! A KeyChord: the Command key and one letter or digit, as the configuration writes it.

use std::fmt;

/// The Command key and one lowercase letter or digit (`cmd+d`): another app's own key
/// combination, which voicepuncher presses as a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    key: char,
}

/// A text that is not `cmd+` and one letter or digit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAKeyChord(pub String);

impl fmt::Display for NotAKeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"{}\" is not a Command shortcut: write cmd+ and one letter or digit, like cmd+d",
            self.0
        )
    }
}

impl std::error::Error for NotAKeyChord {}

impl KeyChord {
    /// Reads `cmd+<letter or digit>`, ignoring case and spaces around the parts.
    pub fn parse(text: &str) -> Result<Self, NotAKeyChord> {
        let _ = text;
        Err(NotAKeyChord(text.to_owned()))
    }

    /// The letter or digit pressed with Command, lowercase.
    pub const fn key(self) -> char {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_command_and_a_letter() {
        assert_eq!(KeyChord::parse("cmd+d").map(KeyChord::key), Ok('d'));
    }

    #[test]
    fn should_parse_command_and_a_digit() {
        assert_eq!(KeyChord::parse("cmd+5").map(KeyChord::key), Ok('5'));
    }

    #[test]
    fn should_refuse_texts_that_are_not_command_and_one_key() {
        for text in ["d", "ctrl+d", "cmd+dd", "cmd+", ""] {
            assert_eq!(
                KeyChord::parse(text),
                Err(NotAKeyChord(text.to_owned())),
                "{text:?}"
            );
        }
    }

    #[test]
    fn should_name_the_text_it_refuses() {
        let message = NotAKeyChord("ctrl+d".to_owned()).to_string();
        assert!(message.contains("\"ctrl+d\""), "{message}");
    }
}
