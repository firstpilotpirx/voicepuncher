use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};

use crate::core::domain::{HotKey, RecordingMode};

pub mod document;
mod loopback;
mod transcription;

pub use loopback::{ensure_loopback, ensure_loopback_websocket};
pub use transcription::TranscriptionCfg;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hotkey: HotkeyCfg,
    pub audio: AudioCfg,
    pub transcription: TranscriptionCfg,
    pub paste: PasteCfg,
    pub sounds: SoundsCfg,
    pub voice_control: VoiceControlCfg,
    pub archive: ArchiveCfg,
    pub logging: LoggingCfg,
    pub services: ServicesCfg,
    pub app: AppCfg,
}

/// What the app records about itself; the settings window shows no control for it.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppCfg {
    /// Whether the app already asked to start at login; it asks only once.
    pub launch_at_login_asked: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotkeyCfg {
    pub key: HotKey,
    pub min_press_ms: u64,
    pub recording_mode: RecordingMode,
}

impl Default for HotkeyCfg {
    fn default() -> Self {
        Self {
            key: HotKey::RightOption,
            min_press_ms: 200,
            recording_mode: RecordingMode::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioCfg {
    pub input_device: String,
    pub max_recording_seconds: u64,
    /// Retired: the dictation archive replaced it. Still accepted, and ignored, so old
    /// configs load.
    #[serde(skip_serializing)]
    debug_save_wav: IgnoredAny,
}

impl Default for AudioCfg {
    fn default() -> Self {
        Self {
            input_device: "default".into(),
            max_recording_seconds: 600,
            debug_save_wav: IgnoredAny,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PasteCfg {
    pub clipboard_restore_delay_ms: u64,
    pub focus_wait_seconds: u64,
}

impl Default for PasteCfg {
    fn default() -> Self {
        Self { clipboard_restore_delay_ms: 120, focus_wait_seconds: 300 }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoundsCfg {
    pub start_sound: String,
    pub stop_sound: String,
    pub cancel_sound: String,
    pub listening_on_sound: String,
    pub listening_off_sound: String,
    /// Voice control: "press enter" was carried out. `command_sound` is its former name.
    #[serde(alias = "command_sound")]
    pub enter_sound: String,
    /// Voice control: "clear all text" was carried out.
    pub clear_sound: String,
    /// Voice control: "paste text" was carried out.
    pub paste_sound: String,
    /// Voice control: "restart dictation" started the dictation over.
    pub restart_sound: String,
}

impl Default for SoundsCfg {
    fn default() -> Self {
        Self {
            start_sound: "Glass".into(),
            stop_sound: "Pop".into(),
            cancel_sound: "Basso".into(),
            listening_on_sound: "Tink".into(),
            listening_off_sound: "Bottle".into(),
            enter_sound: "Morse".into(),
            clear_sound: "Purr".into(),
            paste_sound: "Frog".into(),
            restart_sound: "Hero".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct VoiceControlCfg {
    pub enabled: bool,
    pub url: String,
}

impl Default for VoiceControlCfg {
    fn default() -> Self {
        Self { enabled: false, url: "ws://127.0.0.1:8082".into() }
    }
}

/// The dictation archive: where each dictation's audio and texts are kept, and how much.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchiveCfg {
    pub enabled: bool,
    pub dir: PathBuf,
    pub max_size_mb: u64,
}

const DEFAULT_ARCHIVE_DIR: &str = "~/.voicepuncher/history";
const DEFAULT_ARCHIVE_MAX_SIZE_MB: u64 = 2048;
const BYTES_PER_MEGABYTE: u64 = 1024 * 1024;
const HOME_DIR_PREFIX: &str = "~";

impl ArchiveCfg {
    /// The size limit of the archive in bytes.
    pub fn max_size_bytes(&self) -> u64 {
        self.max_size_mb.saturating_mul(BYTES_PER_MEGABYTE)
    }

    /// The archive folder with a leading `~` replaced by `home`.
    pub fn resolved_dir(&self, home: &Path) -> PathBuf {
        match self.dir.strip_prefix(HOME_DIR_PREFIX) {
            Ok(rest) => home.join(rest),
            Err(_) => self.dir.clone(),
        }
    }
}

impl Default for ArchiveCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: PathBuf::from(DEFAULT_ARCHIVE_DIR),
            max_size_mb: DEFAULT_ARCHIVE_MAX_SIZE_MB,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingCfg {
    pub level: String,
}

impl Default for LoggingCfg {
    fn default() -> Self {
        Self { level: "info".into() }
    }
}

/// The local services the app starts and watches itself, unless `manage` is off.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServicesCfg {
    pub manage: bool,
    pub dictation_copies: u32,
    pub command_copies: u32,
    pub command_spotting_copies: u32,
    pub models_dir: PathBuf,
    /// The speech recognition service for commands, which the command spotting service asks.
    pub commands_url: String,
}

/// The most copies of a service: its clients look on four ports.
pub const MAX_SERVICE_COPIES: u32 = 4;
const DEFAULT_MODELS_DIR: &str = "~/.voicepuncher/models";
/// Copies of a speech recognition service.
const DEFAULT_COPIES: u32 = 3;
/// Two copies of a service that answers one request at a time: one takes over while the
/// other is restarted.
const DEFAULT_SERVICE_COPIES: u32 = 2;

impl ServicesCfg {
    /// The models folder with a leading `~` replaced by `home`.
    pub fn resolved_models_dir(&self, home: &Path) -> PathBuf {
        match self.models_dir.strip_prefix(HOME_DIR_PREFIX) {
            Ok(rest) => home.join(rest),
            Err(_) => self.models_dir.clone(),
        }
    }
}

impl Default for ServicesCfg {
    fn default() -> Self {
        Self {
            manage: true,
            dictation_copies: DEFAULT_COPIES,
            command_copies: DEFAULT_COPIES,
            command_spotting_copies: DEFAULT_SERVICE_COPIES,
            models_dir: PathBuf::from(DEFAULT_MODELS_DIR),
            commands_url: "http://127.0.0.1:8085/v1".into(),
        }
    }
}

/// The configuration file a first launch writes: every option at its default, each with a
/// comment saying what it means.
pub const DEFAULT_FILE: &str = include_str!("config/default.toml");

/// Whether [`ensure_exists`] wrote the configuration file or found one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOutcome {
    Created,
    Existed,
}

/// Writes [`DEFAULT_FILE`] at `path` when nothing is there; an existing file is never touched.
pub fn ensure_exists(path: &Path) -> Result<FileOutcome> {
    if path.exists() {
        return Ok(FileOutcome::Existed);
    }
    document::ConfigDocument::parse(DEFAULT_FILE)?.save(path)?;
    Ok(FileOutcome::Created)
}

pub fn load(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read config: {}", path.display()))?;
    parse(&raw, &path.display().to_string())
}

/// The configuration `raw` holds, checked as a file is; `origin` names it in errors.
pub fn parse(raw: &str, origin: &str) -> Result<Config> {
    let current = without_retired_settings(raw, origin);
    let raw = current.as_deref().unwrap_or(raw);
    let cfg: Config = toml::from_str(raw).map_err(|error| explain(raw, error, origin))?;
    validate(&cfg).with_context(|| format!("invalid config: {origin}"))?;
    Ok(cfg)
}

/// Settings of the retired text refinement: ignored, so an old file keeps loading.
const RETIRED_SECTIONS: [&str; 2] = ["refinement", "llm"];
const RETIRED_SERVICES_KEY: &str = "refinement_copies";

/// `raw` without the retired text refinement settings, with one warning naming them; `None`
/// when it has none (or is no table, which the parse then explains).
fn without_retired_settings(raw: &str, origin: &str) -> Option<String> {
    let mut table = toml::from_str::<toml::Table>(raw).ok()?;
    let mut retired: Vec<String> = RETIRED_SECTIONS
        .into_iter()
        .filter(|section| table.remove(*section).is_some())
        .map(|section| format!("[{section}]"))
        .collect();
    let services = table
        .get_mut("services")
        .and_then(toml::Value::as_table_mut);
    if services.is_some_and(|services| services.remove(RETIRED_SERVICES_KEY).is_some()) {
        retired.push(format!("services.{RETIRED_SERVICES_KEY}"));
    }
    if retired.is_empty() {
        return None;
    }
    log::warn!(
        "config: {origin}: {} retired with text refinement, ignored",
        retired.join(", ")
    );
    toml::to_string(&table).ok()
}

/// Why `raw` is no configuration: the removed section or key it still sets, else `error`.
/// Only a file that fails is parsed a second time, as a table.
fn explain(raw: &str, error: toml::de::Error, origin: &str) -> anyhow::Error {
    let removed = toml::from_str::<toml::Table>(raw).ok().and_then(|table| {
        transcription::reject_removed_whisper_section(&table)
            .and_then(|()| transcription::reject_removed_transcription_keys(&table))
            .err()
    });
    match removed {
        Some(removed) => removed.context(format!("outdated config: {origin}")),
        None => anyhow::Error::new(error).context(format!("parse config: {origin}")),
    }
}

/// Checks rules that span the effective config; run again after CLI flags are overlaid.
pub fn validate(cfg: &Config) -> Result<()> {
    ensure_loopback("transcription.base_url", &cfg.transcription.base_url)?;
    ensure_loopback_websocket("voice_control.url", &cfg.voice_control.url)?;
    ensure_loopback("services.commands_url", &cfg.services.commands_url)?;
    ensure_copies("services.dictation_copies", cfg.services.dictation_copies)?;
    ensure_copies("services.command_copies", cfg.services.command_copies)?;
    ensure_copies(
        "services.command_spotting_copies",
        cfg.services.command_spotting_copies,
    )
}

fn ensure_copies(key: &str, copies: u32) -> Result<()> {
    if (1..=MAX_SERVICE_COPIES).contains(&copies) {
        Ok(())
    } else {
        anyhow::bail!(
            "{key} = {copies}: set 1 to {MAX_SERVICE_COPIES}, the most copies its clients look for"
        )
    }
}

#[cfg(test)]
mod default_file_tests;

#[cfg(test)]
mod tests {
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
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[hotkey]\nkey = \"F13\"").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert_eq!(cfg.hotkey.key, HotKey::F13);
        assert_eq!(cfg.hotkey.min_press_ms, 200);
        assert_eq!(cfg.audio.input_device, "default");
        assert_eq!(cfg.logging.level, "info");
    }

    #[test]
    fn should_wait_five_minutes_for_focus_when_the_setting_is_absent() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[paste]\nclipboard_restore_delay_ms = 120").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert_eq!(cfg.paste.focus_wait_seconds, 300);
        assert_eq!(Config::default().paste.focus_wait_seconds, 300);
    }

    #[test]
    fn should_keep_the_configured_focus_wait() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[paste]\nfocus_wait_seconds = 0").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert_eq!(cfg.paste.focus_wait_seconds, 0);
    }

    #[test]
    fn should_keep_voice_control_off_with_the_local_service_url_when_the_section_is_absent() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[hotkey]\nkey = \"F13\"").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert!(!cfg.voice_control.enabled);
        assert_eq!(cfg.voice_control.url, "ws://127.0.0.1:8082");
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
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[hotkey]\nkey = \"F13\"").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
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
        assert_eq!(t.temperature, 0.2_f32);
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
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[hotkey]\nkey = \"F13\"").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
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
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[services]\nmanage = false\ndictation_copies = 1").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
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
}
