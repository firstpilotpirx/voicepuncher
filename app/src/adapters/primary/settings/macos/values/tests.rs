use super::*;
use crate::adapters::primary::settings::macos::table::find;
use crate::config::document::ConfigDocument;
use crate::core::domain::hotkey::HotKey;

fn titles(choices: &[Choice]) -> Vec<&str> {
    choices.iter().map(|c| c.title.as_str()).collect()
}

#[test]
fn should_show_the_value_the_file_sets() {
    // Given
    let config = crate::config::parse("[hotkey]\nkey = \"RightCommand\"\n", "test").unwrap();
    // When
    let value = shown_value(&config, find("hotkey", "key").unwrap());
    // Then
    assert_eq!(value, Value::String("RightCommand".into()));
}

#[test]
fn should_show_the_default_of_a_value_the_file_leaves_out() {
    // Given
    let config = crate::config::parse("", "test").unwrap();
    // When
    let value = shown_value(&config, find("hotkey", "min_press_ms").unwrap());
    // Then
    assert_eq!(
        value,
        Value::Integer(Config::default().hotkey.min_press_ms as i64)
    );
}

#[test]
fn should_write_an_edited_value_the_loader_reads_back() {
    // Given
    let mut document = ConfigDocument::parse("[hotkey]\nmin_press_ms = 300\n").unwrap();
    // When
    document.set("hotkey", "min_press_ms", to_item(&Value::Integer(450)));
    // Then
    assert_eq!(document.to_config().unwrap().hotkey.min_press_ms, 450);
}

#[test]
fn should_offer_every_hotkey_by_the_menu_bar_name() {
    // When
    let hotkeys = options(Choices::Hotkeys, &[]);
    // Then
    assert_eq!(hotkeys.len(), HotKey::ALL.len());
    let right_command = hotkeys
        .iter()
        .find(|c| c.value == Value::String("RightCommand".into()))
        .unwrap();
    assert_eq!(right_command.title, HotKey::RightCommand.human_name());
}

#[test]
fn should_offer_the_system_default_before_the_microphones() {
    // When
    let microphones = options(Choices::Microphones, &["USB Mic".into(), "AirPods".into()]);
    // Then
    assert_eq!(
        titles(&microphones),
        ["System default", "USB Mic", "AirPods"]
    );
    assert_eq!(microphones[0].value, Value::String("default".into()));
}

#[test]
fn should_offer_none_and_the_fourteen_system_sounds() {
    // When
    let sounds = options(Choices::Sounds, &[]);
    // Then
    assert_eq!(sounds.len(), 15);
    assert_eq!(sounds[0].title, "None");
    assert_eq!(sounds[0].value, Value::String(String::new()));
    assert!(titles(&sounds).contains(&"Tink"));
}

#[test]
fn should_offer_automatic_detection_and_every_known_language() {
    // When
    let languages = options(Choices::Languages, &[]);
    // Then
    assert_eq!(languages[0].value, Value::String("auto".into()));
    assert!(languages
        .iter()
        .any(|c| c.title == "Russian" && c.value == Value::String("ru".into())));
}

#[test]
fn should_offer_both_recording_modes_and_every_log_level() {
    // When
    let modes = options(Choices::RecordingModes, &[]);
    let levels = options(Choices::LogLevels, &[]);
    // Then
    assert_eq!(modes.len(), 2);
    assert_eq!(
        levels.iter().map(|c| c.value.clone()).collect::<Vec<_>>(),
        ["error", "warn", "info", "debug", "trace"].map(|l| Value::String(l.into()))
    );
}

#[test]
fn should_turn_the_language_list_into_text_and_back() {
    // Given
    let list = Value::Array(vec![Value::String("ru".into()), Value::String("en".into())]);
    // When
    let text = languages_text(&list);
    // Then
    assert_eq!(text, "ru, en");
    assert_eq!(languages_value(" ru ,en,, "), list);
}
