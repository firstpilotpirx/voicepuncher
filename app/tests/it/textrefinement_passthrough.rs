use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::TextRefiner;
use voicepuncher::core::domain::RawTranscript;

#[test]
fn should_return_transcript_unchanged() {
    // Given
    let mut refiner = PassthroughTextRefiner;
    let raw = RawTranscript::new("hello world".to_owned()).unwrap();
    // When
    let cleaned = refiner.refine(&raw).unwrap();
    // Then
    assert_eq!(cleaned.as_str(), "hello world");
}

#[test]
fn should_trim_surrounding_whitespace() {
    // Given
    let mut refiner = PassthroughTextRefiner;
    let raw = RawTranscript::new("  hello  ".to_owned()).unwrap();
    // When
    let cleaned = refiner.refine(&raw).unwrap();
    // Then
    assert_eq!(cleaned.as_str(), "hello");
}
