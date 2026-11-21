use crate::config::{parse, Config};

#[test]
fn should_turn_the_claude_dictation_on_with_cmd_d_by_default() {
    // Given
    let file = "";
    // When
    let cfg = parse(file, "test").unwrap();
    // Then
    assert!(cfg.claude_dictation.enabled);
    assert_eq!(cfg.claude_dictation.shortcut, "cmd+d");
    assert_eq!(Config::default().claude_dictation.shortcut, "cmd+d");
}

#[test]
fn should_refuse_a_claude_dictation_shortcut_without_command() {
    // Given
    let file = "[claude_dictation]\nshortcut = \"d\"";
    // When
    let error = parse(file, "test").unwrap_err();
    // Then
    let message = format!("{error:#}");
    assert!(message.contains("claude_dictation.shortcut"), "{message}");
}
