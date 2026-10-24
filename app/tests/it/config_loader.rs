use voicepuncher::config::load;
use voicepuncher::core::domain::HotKey;

#[test]
fn should_round_trip_full_example_toml() {
    // Given
    let toml_str = r#"
[hotkey]
key = "RightOption"
min_press_ms = 200

[audio]
input_device = "default"

[transcription]
base_url = "http://127.0.0.1:8081/v1"
language = "auto"
timeout_secs = 30

[paste]
clipboard_restore_delay_ms = 120

[archive]
enabled = true
dir = "~/.voicepuncher/history"
max_size_mb = 512

[logging]
level = "info"
"#;
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let cfg = load(temp.path()).unwrap();

    // Then
    assert_eq!(cfg.hotkey.key, HotKey::RightOption);
    assert_eq!(cfg.hotkey.min_press_ms, 200);
    assert_eq!(cfg.audio.input_device, "default");
    assert_eq!(cfg.transcription.base_url, "http://127.0.0.1:8081/v1");
    assert_eq!(cfg.transcription.language, "auto");
    assert_eq!(cfg.transcription.timeout_secs, 30);
    assert_eq!(cfg.paste.clipboard_restore_delay_ms, 120);
    assert!(cfg.archive.enabled);
    assert_eq!(
        cfg.archive.dir,
        std::path::Path::new("~/.voicepuncher/history")
    );
    assert_eq!(cfg.archive.max_size_mb, 512);
    assert_eq!(cfg.logging.level, "info");
}

#[test]
fn should_load_a_config_with_the_retired_text_refinement_settings_and_ignore_them() {
    // Given
    let toml_str = "[refinement]\nenabled = true\nmodel = \"qwen\"\n\n\
                    [llm]\nmodel_path = \"~/qwen.gguf\"\n\n\
                    [services]\nrefinement_copies = 2\ndictation_copies = 2\n";

    // When
    let cfg = voicepuncher::config::parse(toml_str, "retired").unwrap();

    // Then
    assert_eq!(cfg.services.dictation_copies, 2);
    let kept = toml::to_string(&cfg).unwrap();
    assert!(!kept.contains("refinement"), "{kept}");
    assert!(!kept.contains("llm"), "{kept}");
}

#[test]
fn should_load_shipped_example_config() {
    // Given
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.toml");

    // When
    let cfg = load(&example).unwrap();

    // Then
    assert_eq!(cfg.transcription.base_url, "http://127.0.0.1:8081/v1");
    assert_eq!(cfg.transcription.chunk_threshold_secs, 15);
    assert!(cfg.archive.enabled);
    assert_eq!(cfg.archive.max_size_mb, 2048);
}

#[test]
fn should_reject_old_whisper_section_naming_replacement() {
    // Given
    let toml_str = "[whisper]\nmodel_path = \"~/ggml.bin\"\nthreads = 4\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let msg = format!("{:#}", load(temp.path()).unwrap_err());

    // Then
    assert!(msg.contains("whisper.model_path"), "{msg}");
    assert!(msg.contains("whisper.threads"), "{msg}");
    assert!(msg.contains("[transcription].base_url"), "{msg}");
}

#[test]
fn should_reject_unknown_key_in_config() {
    // Given
    let toml_str = "[hotkey]\nkey = \"F13\"\n[transcription]\nthrads = 0\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let result = load(temp.path());

    // Then
    assert!(result.is_err(), "expected Err for unknown field 'thrads'");
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("thrads") || msg.contains("transcription"),
        "error message should mention the offending field or section, got: {msg}"
    );
}

#[test]
fn should_load_with_only_hotkey_section() {
    // Given
    let toml_str = "[hotkey]\nkey = \"F13\"\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let cfg = load(temp.path()).unwrap();

    // Then
    assert_eq!(cfg.hotkey.key, HotKey::F13);
    assert_eq!(cfg.audio.input_device, "default");
}

#[test]
fn should_load_a_config_with_the_retired_debug_save_wav_key() {
    // Given
    let toml_str = "[audio]\ndebug_save_wav = true\n";

    // When
    let cfg = voicepuncher::config::parse(toml_str, "retired").unwrap();

    // Then
    let audio = toml::to_string(&cfg.audio).unwrap();
    assert!(!audio.contains("debug_save_wav"), "{audio}");
}
