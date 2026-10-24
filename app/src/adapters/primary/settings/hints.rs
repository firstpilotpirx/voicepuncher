//! What the settings window adds to its controls beyond the configuration, without
//! `AppKit`: how a disabled login switch is drawn and the line naming what the runtime
//! waits for.

use super::macos::overrides::OverrideNote;
use super::macos::table::FIELDS;
use crate::core::domain::Permission;

/// The field that notes the wait for permissions.
const WAIT_FIELD: &str = "voice_control.enabled";

/// The alpha value of the login switch: dimmed when it does not respond.
pub fn switch_alpha(enabled: bool) -> f64 {
    if enabled {
        1.0
    } else {
        0.4
    }
}

/// The names of `permissions` as a sentence lists them: "A", "A and B", "A, B and C".
pub fn joined_names(permissions: &[Permission]) -> String {
    let names: Vec<&str> = permissions.iter().map(|p| p.name()).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}

/// The line under the voice control switch while the runtime waits for `missing`; none when
/// it waits for no permission.
pub fn waiting_note(missing: &[Permission]) -> Option<String> {
    let pronoun = if missing.len() > 1 {
        "they are"
    } else {
        "it is"
    };
    (!missing.is_empty()).then(|| {
        let names = joined_names(missing);
        format!("Waiting for {names} — dictation and voice control start once {pronoun} granted")
    })
}

/// The notes under the fields, by each field's place in the table: the command line's
/// overrides, then the wait for `missing` under the voice control switch.
pub fn placed_notes(overrides: Vec<OverrideNote>, missing: &[Permission]) -> Vec<(usize, String)> {
    let wait = waiting_note(missing).map(|note| (WAIT_FIELD.to_owned(), note));
    overrides
        .into_iter()
        .map(|note| (note.field.path(), note.note))
        .chain(wait)
        .filter_map(|(path, note)| Some((FIELDS.iter().position(|f| f.path() == path)?, note)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_note_the_missing_permissions_under_voice_control() {
        assert_eq!(
            waiting_note(&[Permission::Accessibility]).as_deref(),
            Some(
                "Waiting for Accessibility — dictation and voice control start once it is granted"
            )
        );
        let two = waiting_note(&[Permission::Accessibility, Permission::Microphone]);
        assert_eq!(
            two.as_deref(),
            Some(
                "Waiting for Accessibility and Microphone — dictation and voice control start \
                 once they are granted"
            )
        );
        assert_eq!(waiting_note(&[]), None);
    }

    #[test]
    fn should_place_the_waiting_note_under_the_voice_control_switch() {
        let notes = placed_notes(vec![], &[Permission::Accessibility]);
        let place = FIELDS.iter().position(|f| f.path() == WAIT_FIELD);
        assert_eq!(notes.len(), 1);
        assert_eq!(Some(notes[0].0), place);
    }

    #[test]
    fn should_join_three_names_with_commas_and_and() {
        let all = [
            Permission::Microphone,
            Permission::Accessibility,
            Permission::InputMonitoring,
        ];
        assert_eq!(
            joined_names(&all),
            "Microphone, Accessibility and Input Monitoring"
        );
    }

    #[test]
    fn should_dim_a_disabled_switch() {
        assert!(switch_alpha(false) < 0.5);
        assert!((switch_alpha(true) - 1.0).abs() < f64::EPSILON);
    }
}
