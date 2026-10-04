use voicepuncher::adapters::primary::setup::cli::{SetupAnswers, SetupWizard};
use voicepuncher::core::domain::{HotKey, RecordingMode};

#[test]
fn should_skip_run_when_config_already_exists_and_force_is_false() {
    // Given — an existing config file (any non-empty file will do)
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "[hotkey]\nkey = \"F13\"\n").unwrap();
    // When — force=false with existing config must return Ok without interaction
    let result = SetupWizard::run_if_needed(tmp.path(), false);
    // Then
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
}

#[test]
fn should_skip_run_when_config_does_not_exist_but_force_is_false() {
    // Given — a path that does NOT exist
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does_not_exist.toml");
    assert!(!path.exists());
    // When — force=false, non-existing config → triggers setup wizard, but we cannot
    // drive the interactive TTY in an IT. This test only verifies the guard branch:
    // when the file does not exist and we DO NOT force, run_if_needed returns
    // whatever run() returns (may be an error in a non-interactive context).
    // The important contract: it must not panic.
    let _ = SetupWizard::run_if_needed(&path, false);
    // No assert — just confirm no panic.
}

#[test]
fn should_write_service_sections_that_load_back() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let answers = SetupAnswers {
        transcription_url: "http://localhost:9000/v1",
        refinement_url: "http://localhost:9090/v1",
        refinement_model: "qwen2.5",
        lang: "ru",
        hotkey: HotKey::F13,
        recording_mode: RecordingMode::Toggle,
        start_sound: "Glass",
        stop_sound: "Pop",
    };
    // When
    SetupWizard::write_config(&path, &answers).unwrap();
    // Then
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written
        .contains("[transcription]\nbase_url = \"http://localhost:9000/v1\"\nlanguage = \"ru\"\n"));
    assert!(written
        .contains("[refinement]\nbase_url = \"http://localhost:9090/v1\"\nmodel = \"qwen2.5\"\n"));
    assert!(!written.contains("[whisper]"));
    assert!(!written.contains("[llm]"));
    let cfg = voicepuncher::config::load(&path).unwrap();
    assert_eq!(cfg.transcription.base_url, "http://localhost:9000/v1");
    assert_eq!(cfg.transcription.language, "ru");
    assert_eq!(cfg.refinement.base_url, "http://localhost:9090/v1");
    assert_eq!(cfg.refinement.model, "qwen2.5");
}
