use voicepuncher::adapters::secondary::textpasting::stdio::StdioTextPaster;
use voicepuncher::core::application::TextPaster;
use voicepuncher::core::domain::CleanedText;

#[test]
fn should_write_text_to_stdout() {
    // Given
    let mut paster = StdioTextPaster::new();
    let text = CleanedText::new("hello world".to_string()).expect("non-empty text");
    // When
    let result = paster.paste(&text);
    // Then
    assert!(result.is_ok());
}
