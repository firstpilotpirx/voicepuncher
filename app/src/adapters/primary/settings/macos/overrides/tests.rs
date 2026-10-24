use super::*;
use crate::core::domain::hotkey::HotKey;

#[test]
fn should_note_the_hotkey_the_command_line_set_by_its_name() {
    // Given
    let file = Config::default();
    let mut effective = file.clone();
    effective.hotkey.key = HotKey::F13;
    // When
    let notes = override_notes(&file, &effective);
    // Then
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].field.path(), "hotkey.key");
    assert_eq!(notes[0].note, "Set to F13 by the command line for this run");
}

#[test]
fn should_note_voice_control_turned_on_on_the_command_line() {
    // Given
    let mut file = Config::default();
    file.voice_control.enabled = false;
    let mut effective = file.clone();
    effective.voice_control.enabled = true;
    // When
    let notes = override_notes(&file, &effective);
    // Then
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].field.path(), "voice_control.enabled");
    assert_eq!(notes[0].note, "Set to On by the command line for this run");
}

#[test]
fn should_note_nothing_without_overrides() {
    // When
    let notes = override_notes(&Config::default(), &Config::default());
    // Then
    assert!(notes.is_empty());
}
