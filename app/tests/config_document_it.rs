//! The settings window's writes to a real configuration file on disk.

use voicepuncher::config::document::ConfigDocument;

const USER_FILE: &str = "\
# voicepuncher — my settings
[hotkey]
key = \"RightOption\"   # thumb key
min_press_ms = 200

[sounds]
# quieter start
start_sound = \"Tink\"
stop_sound = \"Pop\"

[[refinement.examples]]
transcript = \"raw\"
normalized = \"Raw.\"
";

#[test]
fn should_change_only_the_edited_line_of_a_commented_file() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("config.toml");
    std::fs::write(&path, USER_FILE).unwrap();
    let mut document = ConfigDocument::load(&path).unwrap();
    // When
    document.set("sounds", "stop_sound", toml_edit::value("Purr"));
    document.save(&path).unwrap();
    // Then
    let saved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        saved,
        USER_FILE.replace("stop_sound = \"Pop\"", "stop_sound = \"Purr\"")
    );
}

#[test]
fn should_create_a_missing_file_with_the_edited_key() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("new").join("config.toml");
    let mut document = ConfigDocument::load(&path).unwrap();
    // When
    document.set("hotkey", "key", toml_edit::value("F13"));
    document.save(&path).unwrap();
    // Then
    let config = voicepuncher::config::load(&path).unwrap();
    assert_eq!(config.hotkey.key.to_string(), "F13");
    assert_eq!(config.hotkey.min_press_ms, 200);
}

#[test]
fn should_leave_no_temporary_file_beside_the_saved_one() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("config.toml");
    std::fs::write(&path, USER_FILE).unwrap();
    let mut document = ConfigDocument::load(&path).unwrap();
    // When
    document.set("hotkey", "min_press_ms", toml_edit::value(300));
    document.save(&path).unwrap();
    // Then
    let names: Vec<_> = std::fs::read_dir(folder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("config.toml")]);
}
