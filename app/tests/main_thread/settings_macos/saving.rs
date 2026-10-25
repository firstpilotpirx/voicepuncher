//! Values picked in the window, saved, refused or played.

use std::rc::Rc;

use super::{
    control_of, find, open_form, options, AppliedConfigs, Choices, Event, Form, NSSound, NSString,
    SettingsWindow, Value, FIELDS, RIGHT_COMMAND,
};

pub(super) fn should_find_every_offered_sound_among_the_system_sounds() {
    // Given
    let sounds = options(Choices::Sounds, &[]);
    // When
    let missing: Vec<String> = sounds
        .iter()
        .filter_map(|choice| choice.value.as_str())
        .filter(|name| !name.is_empty())
        .filter(|name| NSSound::soundNamed(&NSString::from_str(name)).is_none())
        .map(str::to_owned)
        .collect();
    // Then
    assert!(missing.is_empty(), "NSSound does not know {missing:?}");
}

/// The window over `folder` after the hotkey option `key` was set to `value` and saved.
fn saved(
    folder: &tempfile::TempDir,
    applied: &AppliedConfigs,
    key: &str,
    value: &Value,
) -> (SettingsWindow, Rc<Form>) {
    let (window, form) = open_form(folder.path(), applied);
    control_of(&form, "hotkey", key).show(value);
    form.handle(Event::Save);
    (window, form)
}

pub(super) fn should_build_a_control_for_every_option_and_close() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    // When
    let (_window, form) = open_form(folder.path(), &AppliedConfigs::default());
    // Then
    assert_eq!(form.controls.len(), FIELDS.len());
    assert_eq!(form.tabs.len(), 7);
    let tab = form.controller.selectedTabViewItemIndex();
    assert_eq!(tab, 0, "the window opens on General, the first tab");
    let hotkey = control_of(&form, "hotkey", "key").read();
    assert_eq!(hotkey, Value::String("RightCommand".to_owned()));
    form.window.close();
}

pub(super) fn should_save_a_value_picked_in_the_window_to_the_file() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let applied = AppliedConfigs::default();
    // When
    let (_window, form) = saved(&folder, &applied, "key", &Value::String("F13".to_owned()));
    // Then
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert!(file.contains("# my hotkey"), "{file}");
    assert!(file.contains("key = \"F13\""), "{file}");
    assert_eq!(applied.0.lock().unwrap().len(), 1);
    assert_eq!(form.status_line(), "Saved");
    form.window.close();
}

pub(super) fn should_mark_a_refused_value_and_leave_the_file_unchanged() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let applied = AppliedConfigs::default();
    // When
    let (_window, form) = saved(&folder, &applied, "min_press_ms", &Value::Integer(5));
    // Then
    assert_eq!(
        form.status_line(),
        "Minimum press must be between 50 and 1000 ms"
    );
    let marked = control_of(&form, "hotkey", "min_press_ms").outlined.layer();
    assert!(marked.is_some_and(|layer| layer.borderWidth() > 0.0));
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert_eq!(file, RIGHT_COMMAND);
    assert!(applied.0.lock().unwrap().is_empty());
    form.handle(Event::Revert);
    form.window.close();
}

pub(super) fn should_play_a_sound_without_changing_the_file() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let applied = AppliedConfigs::default();
    let (_window, form) = open_form(folder.path(), &applied);
    let start = find("sounds", "start_sound").expect("the start sound has a field");
    let tag = FIELDS
        .iter()
        .position(|field| field.path() == start.path())
        .unwrap();
    control_of(&form, "sounds", "start_sound").show(&Value::String("Ping".to_owned()));
    // When
    form.handle(Event::Play(isize::try_from(tag).unwrap()));
    // Then
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert_eq!(file, RIGHT_COMMAND);
    assert!(applied.0.lock().unwrap().is_empty());
    assert_eq!(form.status_line(), "");
    form.handle(Event::Revert);
    form.window.close();
}
