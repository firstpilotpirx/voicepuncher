//! The values a field shows and offers, and how an edited value goes into the file.

use toml::Value;

use super::field::{Choices, Control, Field};
use crate::config::{Config, TranscriptionCfg};
use crate::core::domain::hotkey::HotKey;
use crate::core::domain::recording_mode::RecordingMode;

/// The system sounds of macOS a sound field offers after None.
const SYSTEM_SOUNDS: [&str; 14] = [
    "Basso",
    "Blow",
    "Bottle",
    "Frog",
    "Funk",
    "Glass",
    "Hero",
    "Morse",
    "Ping",
    "Pop",
    "Purr",
    "Sosumi",
    "Submarine",
    "Tink",
];
/// The sound value that plays nothing.
const NO_SOUND: &str = "";
/// The microphone value that records from the system default input device.
const DEFAULT_MICROPHONE: &str = "default";
/// The language value that lets the speech recognition service detect the language.
const AUTO_LANGUAGE: &str = "auto";
const LOG_LEVELS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];
/// What separates the codes of the language list in its text field.
const LANGUAGE_SEPARATOR: char = ',';
/// The key `to_item` wraps a value in to let `toml` write it.
const WRAPPER_KEY: &str = "value";

/// One entry of a pop-up or a pair of radio buttons: what it reads and what it saves.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub title: String,
    pub value: Value,
}

impl Choice {
    fn new(title: impl Into<String>, value: impl Into<String>) -> Self {
        Self { title: title.into(), value: Value::String(value.into()) }
    }
}

/// The value `field` has in `config`: the file's value, or the default the file leaves.
pub fn shown_value(config: &Config, field: &Field) -> Value {
    // Every field names a configuration key (the settings tests check each); one that did not
    // would show empty
    Value::try_from(config)
        .ok()
        .and_then(|all| all.get(field.section)?.get(field.key).cloned())
        .unwrap_or_else(|| Value::String(String::new()))
}

/// `value` as a `toml_edit` item for [`crate::config::document::ConfigDocument::set`].
///
/// A list of tables becomes `[[section.key]]` blocks, everything else a plain value; no item for a
/// value TOML cannot write, which a value read from TOML never is.
pub fn to_item(value: &Value) -> toml_edit::Item {
    let mut wrapper = toml::Table::new();
    wrapper.insert(WRAPPER_KEY.to_owned(), value.clone());
    toml::to_string(&wrapper)
        .ok()
        .and_then(|text| text.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|mut document| document.remove(WRAPPER_KEY))
        .unwrap_or_default()
}

/// The entries `choices` offers; `microphones` are the Mac's input devices by name.
pub fn options(choices: Choices, microphones: &[String]) -> Vec<Choice> {
    match choices {
        Choices::Hotkeys => HotKey::ALL.iter().copied().map(hotkey_choice).collect(),
        Choices::RecordingModes => [RecordingMode::Hold, RecordingMode::Toggle]
            .into_iter()
            .map(mode_choice)
            .collect(),
        Choices::Microphones => with_first(
            Choice::new("System default", DEFAULT_MICROPHONE),
            microphones.iter().map(|name| Choice::new(name, name)),
        ),
        Choices::Languages => with_first(
            Choice::new("Detect automatically", AUTO_LANGUAGE),
            TranscriptionCfg::language_names()
                .iter()
                .map(|(code, name)| Choice::new(capitalized(name), *code)),
        ),
        Choices::Sounds => with_first(
            Choice::new("None", NO_SOUND),
            SYSTEM_SOUNDS
                .iter()
                .map(|sound| Choice::new(*sound, *sound)),
        ),
        Choices::LogLevels => LOG_LEVELS.iter().map(level_choice).collect(),
    }
}

fn hotkey_choice(key: HotKey) -> Choice {
    Choice::new(key.human_name(), serialized(&key))
}

fn mode_choice(mode: RecordingMode) -> Choice {
    Choice::new(mode.to_string(), serialized(&mode))
}

fn level_choice(level: &&str) -> Choice {
    Choice::new(capitalized(level), *level)
}

/// `value` of `field` in words: a choice by its title, a switch as On or Off.
pub fn display(field: &Field, value: &Value) -> String {
    match (field.control, value) {
        (Control::Switch, Value::Boolean(on)) => if *on { "On" } else { "Off" }.to_owned(),
        (Control::PopUp(choices) | Control::Radio(choices), _) => options(choices, &[])
            .into_iter()
            .find(|choice| choice.value == *value)
            .map_or_else(|| plain(value), |choice| choice.title),
        (Control::LanguageList, _) => languages_text(value),
        _ => plain(value),
    }
}

fn plain(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The language list as the window's text field shows it: `ru, en`.
pub fn languages_text(value: &Value) -> String {
    value
        .as_array()
        .map(|codes| codes.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default()
        .join(", ")
}

/// The text field's language list as the file stores it.
pub fn languages_value(text: &str) -> Value {
    Value::Array(
        text.split(LANGUAGE_SEPARATOR)
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(|code| Value::String(code.to_owned()))
            .collect(),
    )
}

fn serialized(value: &impl serde::Serialize) -> String {
    match Value::try_from(value) {
        Ok(Value::String(text)) => text,
        // Every choice is a unit enum variant, which serializes to its name
        _ => String::new(),
    }
}

fn with_first(first: Choice, rest: impl Iterator<Item = Choice>) -> Vec<Choice> {
    std::iter::once(first).chain(rest).collect()
}

fn capitalized(word: &str) -> String {
    let mut letters = word.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
