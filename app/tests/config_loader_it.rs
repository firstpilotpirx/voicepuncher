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

[llm]
model_path = "~/.voicepuncher/models/Qwen2.5-3B-Instruct-Q4_K_M.gguf"
system_prompt = "You clean up speech-to-text transcripts."
max_tokens = 512
temperature = 0.0
context_size = 4096

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
    assert_eq!(cfg.llm.max_tokens, 512);
    assert_eq!(cfg.llm.temperature, 0.0_f32);
    assert_eq!(cfg.llm.context_size, 4096);
    assert_eq!(cfg.paste.clipboard_restore_delay_ms, 120);
    assert_eq!(cfg.logging.level, "info");
}

#[test]
fn should_expand_tilde_in_llm_model_path() {
    // Given
    let toml_str = "[hotkey]\nkey = \"F13\"\n[llm]\nmodel_path = \"~/bar\"\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let cfg = load(temp.path()).unwrap();

    // Then
    let home = dirs::home_dir().expect("home dir must be available");
    assert!(cfg.llm.model_path.starts_with(&home));
    assert!(cfg.llm.model_path.ends_with("bar"));
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
    assert_eq!(cfg.llm.context_size, 4096);
}
