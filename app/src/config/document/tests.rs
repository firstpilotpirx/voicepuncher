use super::*;

const COMMENTED: &str = "\
# my voicepuncher settings
[hotkey]
key = \"RightOption\" # the one I like
min_press_ms = 200

# sounds I chose
[sounds]
start_sound = \"Glass\"
";

const WITH_EXAMPLES: &str = "\
[refinement]
model = \"qwen\"

[[refinement.examples]]
transcript = \"raw one\"
normalized = \"Raw one.\"

[[refinement.examples]]
transcript = \"raw two\"
normalized = \"Raw two.\"

[logging]
level = \"info\"
";

#[test]
fn should_change_only_the_edited_value_and_keep_comments_and_order() {
    // Given
    let mut document = ConfigDocument::parse(COMMENTED).unwrap();
    // When
    document.set("hotkey", "min_press_ms", toml_edit::value(300));
    // Then
    assert_eq!(
        document.to_string(),
        COMMENTED.replace("min_press_ms = 200", "min_press_ms = 300")
    );
}

#[test]
fn should_keep_the_comment_beside_a_changed_value() {
    // Given
    let mut document = ConfigDocument::parse(COMMENTED).unwrap();
    // When
    document.set("hotkey", "key", toml_edit::value("F13"));
    // Then
    assert!(document
        .to_string()
        .contains("key = \"F13\" # the one I like"));
}

#[test]
fn should_add_a_missing_section_at_the_end() {
    // Given
    let mut document = ConfigDocument::parse(COMMENTED).unwrap();
    // When
    document.set("paste", "focus_wait_seconds", toml_edit::value(10));
    // Then
    let text = document.to_string();
    assert!(text.starts_with(COMMENTED));
    assert!(text.ends_with("[paste]\nfocus_wait_seconds = 10\n"));
}

#[test]
fn should_keep_the_examples_when_another_section_changes() {
    // Given
    let mut document = ConfigDocument::parse(WITH_EXAMPLES).unwrap();
    // When
    document.set("logging", "level", toml_edit::value("debug"));
    // Then
    assert_eq!(
        document.to_string(),
        WITH_EXAMPLES.replace("level = \"info\"", "level = \"debug\"")
    );
}

#[test]
fn should_read_a_value_the_file_sets_and_nothing_for_one_it_does_not() {
    // Given
    let document = ConfigDocument::parse(COMMENTED).unwrap();
    // When
    let set = document.get("hotkey", "min_press_ms");
    let unset = document.get("paste", "focus_wait_seconds");
    // Then
    assert_eq!(set.and_then(|item| item.as_integer()), Some(200));
    assert!(unset.is_none());
}

#[test]
fn should_reject_a_document_whose_values_do_not_validate() {
    // Given
    let mut document = ConfigDocument::parse(COMMENTED).unwrap();
    // When
    document.set(
        "transcription",
        "base_url",
        toml_edit::value("http://10.0.0.1:8081/v1"),
    );
    // Then
    assert!(document.to_config().is_err());
}

#[test]
fn should_read_the_configuration_the_document_holds() {
    // Given
    let document = ConfigDocument::parse(COMMENTED).unwrap();
    // When
    let config = document.to_config().unwrap();
    // Then
    assert_eq!(config.hotkey.min_press_ms, 200);
}
