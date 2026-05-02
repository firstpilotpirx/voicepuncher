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

[whisper]
model_path = "~/.voicepuncher/models/ggml-large-v3-turbo-q5_0.bin"
language = "auto"
threads = 0

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
    assert_eq!(cfg.whisper.language, "auto");
    assert_eq!(cfg.whisper.threads, 0);
    assert_eq!(cfg.llm.max_tokens, 512);
    assert_eq!(cfg.llm.temperature, 0.0_f32);
    assert_eq!(cfg.llm.context_size, 4096);
    assert_eq!(cfg.paste.clipboard_restore_delay_ms, 120);
    assert_eq!(cfg.logging.level, "info");
}

#[test]
fn should_expand_tilde_in_model_paths() {
    // Given
    let toml_str = "[hotkey]\nkey = \"F13\"\n[whisper]\nmodel_path = \"~/foo\"\n[llm]\nmodel_path = \"~/bar\"\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let cfg = load(temp.path()).unwrap();

    // Then
    let home = dirs::home_dir().expect("home dir must be available");
    assert!(
        cfg.whisper.model_path.starts_with(&home),
        "whisper model_path {:?} should start with home {:?}",
        cfg.whisper.model_path,
        home
    );
    assert!(
        cfg.whisper.model_path.ends_with("foo"),
        "whisper model_path {:?} should end with 'foo'",
        cfg.whisper.model_path
    );
    assert!(
        cfg.llm.model_path.starts_with(&home),
        "llm model_path {:?} should start with home {:?}",
        cfg.llm.model_path,
        home
    );
    assert!(
        cfg.llm.model_path.ends_with("bar"),
        "llm model_path {:?} should end with 'bar'",
        cfg.llm.model_path
    );
}

#[test]
fn should_reject_unknown_key_in_config() {
    // Given
    let toml_str = "[hotkey]\nkey = \"F13\"\n[whisper]\nthrads = 0\n";
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), toml_str).unwrap();

    // When
    let result = load(temp.path());

    // Then
    assert!(result.is_err(), "expected Err for unknown field 'thrads'");
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("thrads") || msg.contains("whisper"),
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
