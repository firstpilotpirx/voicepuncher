use super::*;

fn migration_error(toml_str: &str) -> String {
    let raw: toml::Table = toml::from_str(toml_str).unwrap();
    format!("{:#}", reject_removed_whisper_section(&raw).unwrap_err())
}

#[test]
fn should_default_to_local_service_on_port_8081() {
    assert_eq!(
        TranscriptionCfg::default().base_url,
        "http://127.0.0.1:8081/v1"
    );
}

#[test]
fn should_default_language_to_auto() {
    assert_eq!(TranscriptionCfg::default().language, "auto");
}

#[test]
fn should_default_languages_to_russian_and_english() {
    assert_eq!(TranscriptionCfg::default().languages, ["ru", "en"]);
}

#[test]
fn should_load_known_language_codes() {
    let cfg: TranscriptionCfg = toml::from_str("languages = [\"en\", \"de\"]\n").unwrap();
    assert_eq!(cfg.languages, ["en", "de"]);
}

#[test]
fn should_fail_loading_an_unknown_language_code() {
    let result = toml::from_str::<TranscriptionCfg>("languages = [\"ru\", \"xx\"]\n");
    let message = result.unwrap_err().to_string();
    assert!(
        message.contains("unknown language code \"xx\""),
        "{message}"
    );
}

#[test]
fn should_fail_loading_an_empty_language_list() {
    let result = toml::from_str::<TranscriptionCfg>("languages = []\n");
    assert!(result.is_err());
}

#[test]
fn should_default_timeout_to_sixty_seconds() {
    assert_eq!(TranscriptionCfg::default().timeout_secs, 60);
}

#[test]
fn should_default_chunking_fields() {
    // Given / When
    let cfg = TranscriptionCfg::default();
    // Then
    assert_eq!(cfg.chunk_threshold_secs, 30);
    assert_eq!(cfg.chunk_overlap_secs, 2);
    assert_eq!(cfg.vad_lookback_secs, 5);
}

#[test]
fn should_refuse_the_removed_fixed_silence_level() {
    let raw: toml::Table = toml::from_str("[transcription]\nvad_rms_threshold = 0.01\n").unwrap();
    let message = format!("{:#}", reject_removed_transcription_keys(&raw).unwrap_err());
    assert!(
        message.contains("transcription.vad_rms_threshold"),
        "{message}"
    );
    assert!(message.contains("background noise"), "{message}");
    assert!(message.contains("delete"), "{message}");
}

#[test]
fn should_accept_a_transcription_section_without_removed_keys() {
    let raw: toml::Table = toml::from_str("[transcription]\nlanguage = \"en\"\n").unwrap();
    assert!(reject_removed_transcription_keys(&raw).is_ok());
}

#[test]
fn should_tell_to_delete_the_fixed_silence_level_of_the_whisper_section() {
    let message = migration_error("[whisper]\nvad_rms_threshold = 0.01\n");
    assert!(
        message.contains("whisper.vad_rms_threshold: delete it"),
        "{message}"
    );
}

#[test]
fn should_accept_config_without_whisper_section() {
    let raw: toml::Table = toml::from_str("[transcription]\nlanguage = \"en\"\n").unwrap();
    assert!(reject_removed_whisper_section(&raw).is_ok());
}

#[test]
fn should_name_service_and_address_setting_for_model_path() {
    // Given / When
    let message = migration_error("[whisper]\nmodel_path = \"/m.bin\"\n");
    // Then
    assert!(message.contains("whisper.model_path"), "{message}");
    assert!(
        message.contains("starting the speech recognition service"),
        "{message}"
    );
    assert!(message.contains("[transcription].base_url"), "{message}");
}

#[test]
fn should_name_service_flag_for_every_decoder_key() {
    for (key, _) in MOVED_TO_SERVICE {
        let message = migration_error(&format!("[whisper]\n{key} = 1\n"));
        assert!(message.contains(&format!("whisper.{key}")), "{message}");
        assert!(message.contains("starting the"), "{key}: {message}");
    }
}

#[test]
fn should_name_prompt_as_replacement_for_initial_prompt() {
    let message = migration_error("[whisper]\ninitial_prompt = \"Docker\"\n");
    assert!(message.contains("[transcription].prompt"), "{message}");
}

#[test]
fn should_name_same_key_in_transcription_for_kept_keys() {
    for key in [
        "language",
        "temperature",
        "chunk_threshold_secs",
        "chunk_overlap_secs",
        "vad_lookback_secs",
    ] {
        let message = migration_error(&format!("[whisper]\n{key} = 1\n"));
        assert!(
            message.contains(&format!("[transcription].{key}")),
            "{message}"
        );
    }
}
