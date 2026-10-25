use super::*;

/// Options the default file leaves out: the retired key is only tolerated.
const LEFT_OUT: [(&str, &str); 1] = [("audio", "debug_save_wav")];

#[test]
fn should_hold_the_defaults_in_the_default_file() {
    // Given / When
    let cfg = parse(DEFAULT_FILE, "the default file").unwrap();
    // Then
    assert_eq!(
        toml::Value::try_from(&cfg).unwrap(),
        toml::Value::try_from(Config::default()).unwrap()
    );
    assert!(without_retired_settings(DEFAULT_FILE, "the default file").is_none());
}

#[test]
fn should_warn_once_naming_every_retired_setting() {
    // Given
    let log = crate::core::application::pipeline::tests::doubles::captured_log();
    let raw = "[refinement]\nenabled = true\n\n[llm]\nmodel_path = \"q.gguf\"\n\n\
               [services]\nrefinement_copies = 2\n";
    // When
    parse(raw, "old.toml").unwrap();
    // Then
    assert_eq!(
        log.starting_with("config: old.toml:"),
        ["config: old.toml: [refinement], [llm], services.refinement_copies retired with text refinement, ignored"]
    );
}

#[test]
fn should_set_every_option_with_a_comment_above_it_in_the_default_file() {
    // Given
    let defaults = toml::Value::try_from(Config::default()).unwrap();
    let document = DEFAULT_FILE.parse::<toml_edit::DocumentMut>().unwrap();
    // When
    let mut missing = Vec::new();
    for (section, keys) in defaults.as_table().unwrap() {
        for key in keys.as_table().unwrap().keys() {
            if LEFT_OUT.contains(&(section.as_str(), key.as_str())) {
                continue;
            }
            let commented = document
                .get(section)
                .and_then(|table| table.as_table())
                .and_then(|table| table.key(key))
                .and_then(|key| key.leaf_decor().prefix())
                .and_then(|prefix| prefix.as_str())
                .is_some_and(|prefix| prefix.trim_start().starts_with('#'));
            if !commented {
                missing.push(format!("{section}.{key}"));
            }
        }
    }
    // Then
    assert!(missing.is_empty(), "not set with a comment: {missing:?}");
}

#[test]
fn should_write_the_default_file_when_there_is_none() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("nested/config.toml");
    // When
    let outcome = ensure_exists(&path).unwrap();
    // Then
    assert_eq!(outcome, FileOutcome::Created);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_FILE);
}

#[test]
fn should_keep_an_existing_file() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("config.toml");
    std::fs::write(&path, "[hotkey]\nkey = \"F13\"\n").unwrap();
    // When
    let outcome = ensure_exists(&path).unwrap();
    // Then
    assert_eq!(outcome, FileOutcome::Existed);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[hotkey]\nkey = \"F13\"\n"
    );
}
