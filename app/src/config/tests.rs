#[test]
fn should_not_record_the_login_question_as_asked_by_default() {
    // Given / When
    let cfg = Config::default();
    // Then
    assert!(!cfg.app.launch_at_login_asked);
}

use super::*;
use std::io::Write;

#[test]
fn should_load_defaults_when_only_hotkey_section_present() {
    // Given
    let file = "[hotkey]\nkey = \"F13\"";
    // When
    let cfg = loaded(file);
    // Then
    assert_eq!(cfg.hotkey.key, HotKey::F13);
    assert_eq!(cfg.hotkey.min_press_ms, 200);
    assert_eq!(cfg.audio.input_device, "default");
    assert_eq!(cfg.logging.level, "info");
}

#[test]
fn should_wait_five_minutes_for_focus_when_the_setting_is_absent() {
    // Given
    let file = "[paste]\nclipboard_restore_delay_ms = 120";
    // When
    let cfg = loaded(file);
    // Then
    assert_eq!(cfg.paste.focus_wait_seconds, 300);
    assert_eq!(Config::default().paste.focus_wait_seconds, 300);
}

#[test]
fn should_keep_the_configured_focus_wait() {
    // Given
    let file = "[paste]\nfocus_wait_seconds = 0";
    // When
    let cfg = loaded(file);
    // Then
    assert_eq!(cfg.paste.focus_wait_seconds, 0);
}

#[test]
fn should_keep_voice_control_off_with_the_local_service_url_when_the_section_is_absent() {
    // Given
    let file = "[hotkey]\nkey = \"F13\"";
    // When
    let cfg = loaded(file);
    // Then
    assert!(!cfg.voice_control.enabled);
    assert_eq!(cfg.voice_control.url, "ws://127.0.0.1:8082");
    assert!(cfg.voice_control.confirm_commands);
}

#[test]
fn should_keep_commands_unconfirmed_when_the_file_says_so() {
    // Given
    let file = "[voice_control]\nconfirm_commands = false";
    // When
    let cfg = loaded(file);
    // Then
    assert!(!cfg.voice_control.confirm_commands);
}

#[test]
fn should_keep_the_configured_voice_control_settings() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        f,
        "[voice_control]\nenabled = true\nurl = \"ws://localhost:9002\""
    )
    .unwrap();
    // When
    let cfg = load(f.path()).unwrap();
    // Then
    assert!(cfg.voice_control.enabled);
    assert_eq!(cfg.voice_control.url, "ws://localhost:9002");
}

#[test]
fn should_reject_a_command_spotting_service_on_another_machine() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "[voice_control]\nurl = \"ws://192.168.1.10:8082\"").unwrap();
    // When
    let message = format!("{:#}", load(f.path()).unwrap_err());
    // Then
    assert!(message.contains("voice_control.url"), "{message}");
    assert!(message.contains("not on this machine"), "{message}");
}

#[test]
fn should_default_the_voice_control_sounds() {
    // Given / When
    let sounds = SoundsCfg::default();
    // Then
    assert_eq!(sounds.listening_on_sound, "Tink");
    assert_eq!(sounds.listening_off_sound, "Bottle");
    assert_eq!(sounds.enter_sound, "Morse");
    assert_eq!(sounds.clear_sound, "Purr");
    assert_eq!(sounds.restart_sound, "Hero");
    assert_eq!(sounds.paste_sound, "Frog");
}

#[test]
fn should_give_every_confirmed_command_a_sound_no_other_cue_uses() {
    // Given
    let sounds = SoundsCfg::default();
    let all = [
        &sounds.start_sound,
        &sounds.stop_sound,
        &sounds.cancel_sound,
        &sounds.listening_on_sound,
        &sounds.listening_off_sound,
        &sounds.enter_sound,
        &sounds.clear_sound,
        &sounds.restart_sound,
        &sounds.paste_sound,
    ];
    // When
    let distinct: std::collections::HashSet<_> = all.iter().collect();
    // Then
    assert_eq!(distinct.len(), all.len(), "{all:?}");
}

#[test]
fn should_take_the_former_command_sound_as_the_enter_sound() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        f,
        "[sounds]\ncommand_sound = \"Ping\"\nclear_sound = \"Funk\"\npaste_sound = \"Blow\""
    )
    .unwrap();
    // When
    let config = load(f.path()).unwrap();
    // Then
    assert_eq!(config.sounds.enter_sound, "Ping");
    assert_eq!(config.sounds.clear_sound, "Funk");
    assert_eq!(config.sounds.paste_sound, "Blow");
}

#[test]
fn should_archive_into_the_history_folder_up_to_two_gigabytes_by_default() {
    // Given
    let file = "[hotkey]\nkey = \"F13\"";
    // When
    let cfg = loaded(file);
    // Then
    assert!(cfg.archive.enabled);
    assert_eq!(cfg.archive.dir, Path::new("~/.voicepuncher/history"));
    assert_eq!(cfg.archive.max_size_mb, 2048);
    assert_eq!(cfg.archive.max_size_bytes(), 2048 * 1024 * 1024);
}

#[test]
fn should_keep_the_configured_archive_settings() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        f,
        "[archive]\nenabled = false\ndir = \"/tmp/dictations\"\nmax_size_mb = 10"
    )
    .unwrap();
    // When
    let cfg = load(f.path()).unwrap();
    // Then
    assert!(!cfg.archive.enabled);
    assert_eq!(cfg.archive.dir, Path::new("/tmp/dictations"));
    assert_eq!(cfg.archive.max_size_bytes(), 10 * 1024 * 1024);
}

#[test]
fn should_resolve_the_archive_folder_under_the_home_folder() {
    // Given
    let archive = ArchiveCfg::default();
    // When
    let dir = archive.resolved_dir(Path::new("/Users/someone"));
    // Then
    assert_eq!(dir, Path::new("/Users/someone/.voicepuncher/history"));
}

#[test]
fn should_keep_an_absolute_archive_folder_as_it_is() {
    // Given
    let archive = ArchiveCfg { dir: "/tmp/dictations".into(), ..ArchiveCfg::default() };
    // When
    let dir = archive.resolved_dir(Path::new("/Users/someone"));
    // Then
    assert_eq!(dir, Path::new("/tmp/dictations"));
}

#[test]
fn should_reject_unknown_key_in_config() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "[hotkey]\nunknown_field = true").unwrap();
    // When
    let result = load(f.path());
    // Then
    assert!(result.is_err());
}

#[test]
fn should_parse_transcription_section_from_toml() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        f,
        "[transcription]\nbase_url = \"http://localhost:9000/v1\"\nmodel = \"large\"\n\
         language = \"en\"\nprompt = \"Docker\"\ntemperature = 0.2\ntimeout_secs = 5\n\
         chunk_threshold_secs = 60"
    )
    .unwrap();
    // When
    let cfg = load(f.path()).unwrap();
    // Then
    let t = &cfg.transcription;
    assert_eq!(t.base_url, "http://localhost:9000/v1");
    assert_eq!(t.model, "large");
    assert_eq!(t.language, "en");
    assert_eq!(t.prompt, "Docker");
    assert!(
        (t.temperature - 0.2_f32).abs() < 1e-6,
        "{} is not {}",
        t.temperature,
        0.2_f32
    );
    assert_eq!(t.timeout_secs, 5);
    assert_eq!(t.chunk_threshold_secs, 60);
    assert_eq!(t.chunk_overlap_secs, 2);
}

#[test]
fn should_reject_remote_transcription_service_at_load() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        f,
        "[transcription]\nbase_url = \"http://192.168.1.10:8081/v1\""
    )
    .unwrap();
    // When
    let message = format!("{:#}", load(f.path()).unwrap_err());
    // Then
    assert!(message.contains("not on this machine"), "{message}");
}

#[test]
fn should_report_removed_whisper_section_with_replacement() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "[whisper]\nmodel_path = \"/m.bin\"").unwrap();
    // When
    let message = format!("{:#}", load(f.path()).unwrap_err());
    // Then
    assert!(message.contains("[transcription]"), "{message}");
    assert!(message.contains("whisper.model_path"), "{message}");
}

#[test]
fn should_reject_remote_service_set_after_load() {
    // Given
    let mut cfg = Config::default();
    cfg.transcription.base_url = "https://example.com/v1".into();
    // When / Then
    assert!(validate(&cfg).is_err());
}

#[test]
fn should_accept_default_service_addresses() {
    assert!(validate(&Config::default()).is_ok());
}

#[test]
fn should_manage_three_copies_of_each_speech_recognition_service_by_default() {
    // Given
    let file = "[hotkey]\nkey = \"F13\"";
    // When
    let cfg = loaded(file);
    // Then
    assert!(cfg.services.manage);
    assert_eq!(
        (cfg.services.dictation_copies, cfg.services.command_copies),
        (3, 3)
    );
    assert_eq!(
        cfg.services.models_dir,
        PathBuf::from("~/.voicepuncher/models")
    );
    assert_eq!(cfg.services.commands_url, "http://127.0.0.1:8085/v1");
}

#[test]
fn should_run_two_copies_of_the_command_spotting_service_by_default() {
    let cfg = Config::default();
    assert_eq!(cfg.services.command_spotting_copies, 2);
}

#[test]
fn should_reject_more_command_spotting_copies_than_the_client_looks_for() {
    for copies in [0, 5] {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[services]\ncommand_spotting_copies = {copies}").unwrap();
        // When
        let message = format!("{:#}", load(f.path()).unwrap_err());
        // Then
        assert!(
            message.contains("services.command_spotting_copies"),
            "{message}"
        );
    }
}

#[test]
fn should_leave_the_services_to_the_scripts_when_told() {
    // Given
    let file = "[services]\nmanage = false\ndictation_copies = 1";
    // When
    let cfg = loaded(file);
    // Then
    assert!(!cfg.services.manage);
    assert_eq!(cfg.services.dictation_copies, 1);
}

#[test]
fn should_reject_more_copies_than_the_pool_looks_for() {
    for copies in [0, 5] {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[services]\ncommand_copies = {copies}").unwrap();
        // When
        let message = format!("{:#}", load(f.path()).unwrap_err());
        // Then
        assert!(message.contains("services.command_copies"), "{message}");
        assert!(message.contains("1 to 4"), "{message}");
    }
}

#[test]
fn should_reject_a_speech_recognition_service_for_commands_on_another_machine() {
    // Given
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "[services]\ncommands_url = \"http://10.0.0.2:8085/v1\"").unwrap();
    // When
    let message = format!("{:#}", load(f.path()).unwrap_err());
    // Then
    assert!(message.contains("services.commands_url"), "{message}");
}

#[test]
fn should_find_the_models_folder_under_home() {
    let services = ServicesCfg::default();
    assert_eq!(
        services.resolved_models_dir(Path::new("/Users/me")),
        PathBuf::from("/Users/me/.voicepuncher/models")
    );
}

/// The configuration a file holding `text` and a newline loads.
fn loaded(text: &str) -> Config {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "{text}").unwrap();
    load(f.path()).unwrap()
}
