use std::path::PathBuf;

use super::*;
use crate::adapters::primary::settings::macos::table::{find, FIELDS};
use crate::config::Config;

fn home() -> PathBuf {
    std::env::temp_dir()
}

fn check(section: &str, key: &str, value: Value) -> Result<(), String> {
    validate(find(section, key).unwrap(), &value, &home())
}

#[test]
fn should_refuse_a_minimum_press_below_its_range_naming_the_range() {
    // When
    let result = check("hotkey", "min_press_ms", Value::Integer(5));
    // Then
    assert_eq!(
        result,
        Err("Minimum press must be between 50 and 1000 ms".to_owned())
    );
}

#[test]
fn should_accept_a_number_at_either_end_of_its_range() {
    // When
    let low = check("hotkey", "min_press_ms", Value::Integer(50));
    let high = check("hotkey", "min_press_ms", Value::Integer(1000));
    // Then
    assert_eq!((low, high), (Ok(()), Ok(())));
}

#[test]
fn should_refuse_a_temperature_above_its_slider() {
    // When
    let result = check("transcription", "temperature", Value::Float(1.5));
    // Then
    assert_eq!(
        result,
        Err("Temperature must be between 0 and 1".to_owned())
    );
}

#[test]
fn should_refuse_a_service_address_on_another_machine() {
    // When
    let result = check(
        "transcription",
        "base_url",
        Value::String("http://10.0.0.7:8081/v1".into()),
    );
    // Then
    assert_eq!(
        result,
        Err("Service address must be on this Mac (127.0.0.1 or localhost)".to_owned())
    );
}

#[test]
fn should_refuse_an_http_address_for_the_voice_control_service() {
    // When
    let result = check(
        "voice_control",
        "url",
        Value::String("http://127.0.0.1:8082".into()),
    );
    // Then
    assert!(result.is_err());
}

#[test]
fn should_refuse_an_unknown_or_empty_language_list() {
    // When
    let unknown = check(
        "transcription",
        "languages",
        Value::Array(vec![Value::String("xx".into())]),
    );
    let empty = check("transcription", "languages", Value::Array(vec![]));
    // Then
    assert!(unknown.unwrap_err().contains("xx"));
    assert!(empty.is_err());
}

#[test]
fn should_refuse_a_folder_inside_a_file() {
    // Given
    let file = tempfile::NamedTempFile::new().unwrap();
    let inside = file.path().join("history");
    // When
    let result = check(
        "archive",
        "dir",
        Value::String(inside.to_string_lossy().into_owned()),
    );
    // Then
    assert!(result.is_err());
}

#[test]
fn should_accept_a_folder_that_does_not_exist_yet_but_can_be_created() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let missing = folder.path().join("later").join("history");
    // When
    let result = check(
        "archive",
        "dir",
        Value::String(missing.to_string_lossy().into_owned()),
    );
    // Then
    assert_eq!(result, Ok(()));
}

#[test]
fn should_accept_every_default_value() {
    // Given
    let defaults = Value::try_from(Config::default()).unwrap();
    // When
    let refused: Vec<String> = FIELDS
        .iter()
        .filter_map(|field| {
            let value = &defaults[field.section][field.key];
            validate(field, value, &home()).err()
        })
        .collect();
    // Then
    assert!(refused.is_empty(), "{refused:?}");
}
