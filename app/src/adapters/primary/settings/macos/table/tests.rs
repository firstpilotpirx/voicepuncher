use std::collections::BTreeSet;

use super::*;
use crate::config::Config;

fn configuration_keys() -> BTreeSet<String> {
    let toml::Value::Table(sections) = toml::Value::try_from(Config::default()).unwrap() else {
        panic!("the configuration is not a table");
    };
    sections
        .iter()
        .flat_map(|(section, keys)| {
            let keys = keys.as_table().expect("every section is a table");
            keys.keys().map(move |key| format!("{section}.{key}"))
        })
        .collect()
}

#[test]
fn should_give_every_live_configuration_key_exactly_one_field() {
    // Given
    let retired: BTreeSet<String> = RETIRED
        .iter()
        .chain(KEPT)
        .map(|(s, k)| format!("{s}.{k}"))
        .collect();
    let expected: BTreeSet<String> = configuration_keys().difference(&retired).cloned().collect();
    // When
    let paths: Vec<String> = FIELDS
        .iter()
        .map(super::super::field::Field::path)
        .collect();
    let unique: BTreeSet<String> = paths.iter().cloned().collect();
    // Then
    assert_eq!(unique.len(), paths.len(), "a key has two fields: {paths:?}");
    assert_eq!(unique, expected);
}

#[test]
fn should_name_only_keys_the_configuration_still_accepts_as_retired() {
    // Given
    let files = RETIRED
        .iter()
        .map(|(section, key)| format!("[{section}]\n{key} = true\n"));
    // When
    let refused: Vec<_> = files
        .filter(|file| crate::config::parse(file, "retired").is_err())
        .collect();
    // Then
    assert!(refused.is_empty(), "{refused:?}");
}

#[test]
fn should_give_the_app_s_own_record_of_the_login_question_no_field() {
    // Given
    let keys = configuration_keys();
    // When
    let edited = FIELDS
        .iter()
        .any(|field| field.path() == "app.launch_at_login_asked");
    // Then
    assert!(keys.contains("app.launch_at_login_asked"));
    assert!(KEPT.contains(&("app", "launch_at_login_asked")));
    assert!(!edited);
}
