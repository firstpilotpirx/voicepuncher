use crate::core::domain::raw_transcript::EmptyTranscript;

#[derive(Debug, Clone)]
pub struct CleanedText(String);

impl CleanedText {
    pub fn new(s: String) -> Result<Self, EmptyTranscript> {
        if s.trim().is_empty() {
            Err(EmptyTranscript)
        } else {
            Ok(Self(s.trim().to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_trim_surrounding_whitespace() {
        // Given
        let input = " hello ".to_owned();
        // When
        let result = CleanedText::new(input).unwrap();
        // Then
        assert_eq!(result.as_str(), "hello");
    }

    #[test]
    fn should_reject_empty_string() {
        // Given
        let input = String::new();
        // When
        let result = CleanedText::new(input);
        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_whitespace_only_string() {
        // Given
        let input = "   \n\t".to_owned();
        // When
        let result = CleanedText::new(input);
        // Then
        assert!(result.is_err());
    }
}
