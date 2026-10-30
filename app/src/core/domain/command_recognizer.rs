/// What hears the voice commands in the command spotting service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRecognizer {
    /// The English and the Russian streaming recognizers behind a voice gate.
    #[default]
    Streaming,
    /// The keyword spotter, confirmed by the speech recognition service for commands.
    KeywordSpotting,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_default_to_the_streaming_recognizer() {
        assert_eq!(CommandRecognizer::default(), CommandRecognizer::Streaming);
    }
}
