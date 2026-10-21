//! The notes under fields the command line of this run overrides.

use super::field::Field;
use super::table::FIELDS;
use super::values::{display, shown_value};
use crate::config::Config;

/// A field the command line overrides, with the note shown under its control.
#[derive(Debug, Clone, PartialEq)]
pub struct OverrideNote {
    pub field: &'static Field,
    pub note: String,
}

/// The fields whose value in `effective` (the file's configuration with this run's
/// command line applied) differs from `file`, each with its note.
pub fn override_notes(file: &Config, effective: &Config) -> Vec<OverrideNote> {
    FIELDS
        .iter()
        .filter_map(|field| {
            let overridden = shown_value(effective, field);
            (overridden != shown_value(file, field)).then(|| OverrideNote {
                field,
                note: format!(
                    "Set to {} by the command line for this run",
                    display(field, &overridden)
                ),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
