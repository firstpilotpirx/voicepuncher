//! Whether a value the user entered may be saved, and why not.

use std::path::{Path, PathBuf};

use toml::Value;

use super::field::{Choices, Control, Field, Scheme};
use crate::config::{ensure_loopback, ensure_loopback_websocket, TranscriptionCfg};

/// What the window says when an address is on another machine or not a valid address.
const NOT_LOOPBACK: &str = "Service address must be on this Mac (127.0.0.1 or localhost)";
/// The `language` value that lets the speech recognition service detect the language.
const AUTO_LANGUAGE: &str = "auto";
/// How a folder in the configuration file names the home folder.
const HOME_PREFIX: &str = "~";

/// `Ok` when `value` may be saved for `field`, else the message the window shows;
/// `home` resolves a folder written with `~`.
pub fn validate(field: &Field, value: &Value, home: &Path) -> Result<(), String> {
    match field.control {
        Control::Number { min, max, unit } => in_range(field, value, min as f64, max as f64, unit),
        Control::Slider { min, max } => in_range(field, value, min, max, ""),
        Control::Address(scheme) => loopback(field, value, scheme),
        Control::LanguageList => languages(value),
        Control::PopUp(Choices::Languages) => language(value),
        Control::Folder => folder(value, home),
        _ => Ok(()),
    }
}

fn in_range(field: &Field, value: &Value, min: f64, max: f64, unit: &str) -> Result<(), String> {
    let number = value.as_integer().map(|n| n as f64).or(value.as_float());
    match number {
        Some(n) if (min..=max).contains(&n) => Ok(()),
        _ => {
            let unit = if unit.is_empty() {
                String::new()
            } else {
                format!(" {unit}")
            };
            Err(format!(
                "{} must be between {min} and {max}{unit}",
                field.label
            ))
        }
    }
}

fn loopback(field: &Field, value: &Value, scheme: Scheme) -> Result<(), String> {
    let url = value.as_str().unwrap_or_default();
    let path = field.path();
    let checked = match scheme {
        Scheme::Http => ensure_loopback(&path, url),
        Scheme::WebSocket => ensure_loopback_websocket(&path, url),
    };
    checked.map_err(|_| NOT_LOOPBACK.to_owned())
}

fn languages(value: &Value) -> Result<(), String> {
    let codes: Vec<&str> = value
        .as_array()
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if codes.is_empty() {
        return Err("Name at least one language".to_owned());
    }
    match codes
        .iter()
        .find(|code| TranscriptionCfg::language_name(code).is_none())
    {
        Some(unknown) => Err(format!("Unknown language code: {unknown}")),
        None => Ok(()),
    }
}

fn language(value: &Value) -> Result<(), String> {
    let code = value.as_str().unwrap_or_default();
    if code == AUTO_LANGUAGE || TranscriptionCfg::language_name(code).is_some() {
        Ok(())
    } else {
        Err(format!("Unknown language code: {code}"))
    }
}

/// A folder may be saved when it exists or every missing part of it can be created:
/// its nearest existing ancestor is a folder.
fn folder(value: &Value, home: &Path) -> Result<(), String> {
    let written = PathBuf::from(value.as_str().unwrap_or_default());
    let folder = match written.strip_prefix(HOME_PREFIX) {
        Ok(rest) => home.join(rest),
        Err(_) => written,
    };
    let existing = folder.ancestors().find(|part| part.exists());
    match existing {
        Some(part) if part.is_dir() => Ok(()),
        _ => Err(format!(
            "Folder {} does not exist and cannot be created",
            folder.display()
        )),
    }
}

#[cfg(test)]
mod tests;
