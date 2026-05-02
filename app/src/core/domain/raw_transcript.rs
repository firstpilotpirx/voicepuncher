#[derive(Debug, thiserror::Error)]
#[error("transcript is empty or whitespace-only")]
pub struct EmptyTranscript;

#[derive(Debug, Clone)]
pub struct RawTranscript(String);

impl RawTranscript {
    pub fn new(s: String) -> Result<Self, EmptyTranscript> {
        if s.trim().is_empty() {
            Err(EmptyTranscript)
        } else {
            Ok(Self(s))
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
    fn should_create_transcript_from_non_empty_string() {
        // Given
        let input = "hello".to_owned();
        // When
        let result = RawTranscript::new(input).unwrap();
        // Then
        assert_eq!(result.as_str(), "hello");
    }

    #[test]
    fn should_reject_empty_string() {
        // Given
        let input = String::new();
        // When
        let result = RawTranscript::new(input);
        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_whitespace_only_string() {
        // Given
        let input = "   \n\t".to_owned();
        // When
        let result = RawTranscript::new(input);
        // Then
        assert!(result.is_err());
    }
}
