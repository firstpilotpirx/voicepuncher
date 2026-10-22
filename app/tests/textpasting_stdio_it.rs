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

#[test]
fn should_paste_text_with_punctuation_and_numbers() {
    // Given
    let mut paster = StdioTextPaster::new();
    let text = CleanedText::new("Meeting at 3pm: agenda #1 — 'discuss Q4'.".to_string()).unwrap();
    // When
    let result = paster.paste(&text);
    // Then
    assert!(result.is_ok());
}

#[test]
fn should_paste_single_word_text() {
    // Given
    let mut paster = StdioTextPaster::new();
    let text = CleanedText::new("OK".to_string()).unwrap();
    // When
    let result = paster.paste(&text);
    // Then
    assert!(result.is_ok());
}

#[test]
fn should_be_callable_multiple_times_in_sequence() {
    // Given
    let mut paster = StdioTextPaster::new();
    let texts = ["first", "second", "third"];
    // When
    for t in &texts {
        let text = CleanedText::new((*t).to_string()).unwrap();
        let result = paster.paste(&text);
        // Then
        assert!(result.is_ok(), "paste of '{t}' should succeed");
    }
}
