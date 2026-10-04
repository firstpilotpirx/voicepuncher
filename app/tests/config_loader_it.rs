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
debug_save_wav = false

[transcription]
base_url = "http://127.0.0.1:8081/v1"
language = "auto"
timeout_secs = 30

[refinement]
base_url = "http://127.0.0.1:8080/v1"
model = "qwen"
system_prompt = "You clean up speech-to-text transcripts."
max_tokens = 256
temperature = 0.0
timeout_secs = 20

[paste]
clipboard_restore_delay_ms = 120

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
    assert!(!cfg.audio.debug_save_wav);
    assert_eq!(cfg.transcription.base_url, "http://127.0.0.1:8081/v1");
    assert_eq!(cfg.transcription.language, "auto");
    assert_eq!(cfg.transcription.timeout_secs, 30);
    assert_eq!(cfg.refinement.base_url, "http://127.0.0.1:8080/v1");
    assert_eq!(cfg.refinement.model, "qwen");
    assert_eq!(cfg.refinement.max_tokens, 256);
    assert_eq!(cfg.refinement.temperature, 0.0_f32);
    assert_eq!(cfg.refinement.timeout_secs, 20);
    assert_eq!(cfg.paste.clipboard_restore_delay_ms, 120);
    assert_eq!(cfg.logging.level, "info");
}

#[test]
fn should_reject_old_llm_section_naming_replacement() {
    // Given
    let toml_str = "[llm]\nmodel_path = \"~/qwen.gguf\"\ncontext_size = 4096\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let msg = format!("{:#}", load(temp.path()).unwrap_err());

    // Then
    assert!(msg.contains("llm.model_path"), "{msg}");
    assert!(msg.contains("llm.context_size"), "{msg}");
    assert!(msg.contains("[refinement].base_url"), "{msg}");
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
    assert_eq!(cfg.refinement.base_url, "http://127.0.0.1:8080/v1");
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
    assert_eq!(cfg.refinement.max_tokens, 512);
}
