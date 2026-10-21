use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::core::domain::{HotKey, RecordingMode};

pub mod document;
mod loopback;
mod refinement;
mod transcription;

pub use loopback::{ensure_loopback, ensure_loopback_websocket};
pub use refinement::{RefinementCfg, RefinementExample};
pub use transcription::TranscriptionCfg;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub hotkey: HotkeyCfg,
    #[serde(default)]
    pub audio: AudioCfg,
    #[serde(default)]
    pub transcription: TranscriptionCfg,
    #[serde(default)]
    pub refinement: RefinementCfg,
    #[serde(default)]
    pub paste: PasteCfg,
    #[serde(default)]
    pub sounds: SoundsCfg,
    #[serde(default)]
    pub voice_control: VoiceControlCfg,
    #[serde(default)]
    pub archive: ArchiveCfg,
    #[serde(default)]
    pub logging: LoggingCfg,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyCfg {
    #[serde(default = "HotkeyCfg::default_key")]
    pub key: HotKey,
    #[serde(default = "HotkeyCfg::default_min_press_ms")]
    pub min_press_ms: u64,
    #[serde(default)]
    pub recording_mode: RecordingMode,
}

impl HotkeyCfg {
    fn default_key() -> HotKey {
        HotKey::RightOption
    }
    fn default_min_press_ms() -> u64 {
        200
    }
}

impl Default for HotkeyCfg {
    fn default() -> Self {
        Self {
            key: Self::default_key(),
            min_press_ms: Self::default_min_press_ms(),
            recording_mode: RecordingMode::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCfg {
    #[serde(default = "AudioCfg::default_input_device")]
    pub input_device: String,
    /// Retired: the dictation archive replaced it. Still accepted so old configs load.
    #[serde(default)]
    pub debug_save_wav: bool,
    #[serde(default = "AudioCfg::default_max_recording_seconds")]
    pub max_recording_seconds: u64,
}

impl AudioCfg {
    fn default_input_device() -> String {
        "default".into()
    }
    fn default_max_recording_seconds() -> u64 {
        600
    }
}

impl Default for AudioCfg {
    fn default() -> Self {
        Self {
            input_device: Self::default_input_device(),
            debug_save_wav: false,
            max_recording_seconds: Self::default_max_recording_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PasteCfg {
    #[serde(default = "PasteCfg::default_clipboard_restore_delay_ms")]
    pub clipboard_restore_delay_ms: u64,
    #[serde(default = "PasteCfg::default_focus_wait_seconds")]
    pub focus_wait_seconds: u64,
}

impl PasteCfg {
    fn default_clipboard_restore_delay_ms() -> u64 {
        120
    }
    fn default_focus_wait_seconds() -> u64 {
        300
    }
}

impl Default for PasteCfg {
    fn default() -> Self {
        Self {
            clipboard_restore_delay_ms: Self::default_clipboard_restore_delay_ms(),
            focus_wait_seconds: Self::default_focus_wait_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SoundsCfg {
    #[serde(default = "SoundsCfg::default_start_sound")]
    pub start_sound: String,
    #[serde(default = "SoundsCfg::default_stop_sound")]
    pub stop_sound: String,
    #[serde(default = "SoundsCfg::default_cancel_sound")]
    pub cancel_sound: String,
    #[serde(default = "SoundsCfg::default_listening_on_sound")]
    pub listening_on_sound: String,
    #[serde(default = "SoundsCfg::default_listening_off_sound")]
    pub listening_off_sound: String,
    /// Voice control: "press enter" was carried out. `command_sound` is its former name.
    #[serde(default = "SoundsCfg::default_enter_sound", alias = "command_sound")]
    pub enter_sound: String,
    /// Voice control: "clear all text" was carried out.
    #[serde(default = "SoundsCfg::default_clear_sound")]
    pub clear_sound: String,
    /// Voice control: "paste text" was carried out.
    #[serde(default = "SoundsCfg::default_paste_sound")]
    pub paste_sound: String,
    /// Voice control: "restart dictation" started the dictation over.
    #[serde(default = "SoundsCfg::default_restart_sound")]
    pub restart_sound: String,
}

impl SoundsCfg {
    fn default_start_sound() -> String {
        "Glass".into()
    }

    fn default_stop_sound() -> String {
        "Pop".into()
    }

    fn default_cancel_sound() -> String {
        "Basso".into()
    }

    fn default_listening_on_sound() -> String {
        "Tink".into()
    }

    fn default_listening_off_sound() -> String {
        "Bottle".into()
    }

    fn default_enter_sound() -> String {
        "Morse".into()
    }

    fn default_clear_sound() -> String {
        "Purr".into()
    }

    fn default_paste_sound() -> String {
        "Frog".into()
    }

    fn default_restart_sound() -> String {
        "Hero".into()
    }
}

impl Default for SoundsCfg {
    fn default() -> Self {
        Self {
            start_sound: Self::default_start_sound(),
            stop_sound: Self::default_stop_sound(),
            cancel_sound: Self::default_cancel_sound(),
            listening_on_sound: Self::default_listening_on_sound(),
            listening_off_sound: Self::default_listening_off_sound(),
            enter_sound: Self::default_enter_sound(),
            clear_sound: Self::default_clear_sound(),
            paste_sound: Self::default_paste_sound(),
            restart_sound: Self::default_restart_sound(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceControlCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "VoiceControlCfg::default_url")]
    pub url: String,
}

impl VoiceControlCfg {
    fn default_url() -> String {
        "ws://127.0.0.1:8082".into()
    }
}

impl Default for VoiceControlCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            url: Self::default_url(),
        }
    }
}

/// The dictation archive: where each dictation's audio and texts are kept, and how much.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCfg {
    #[serde(default = "ArchiveCfg::default_enabled")]
    pub enabled: bool,
    #[serde(default = "ArchiveCfg::default_dir")]
    pub dir: PathBuf,
    #[serde(default = "ArchiveCfg::default_max_size_mb")]
    pub max_size_mb: u64,
}

const DEFAULT_ARCHIVE_DIR: &str = "~/.voicepuncher/history";
const DEFAULT_ARCHIVE_MAX_SIZE_MB: u64 = 2048;
const BYTES_PER_MEGABYTE: u64 = 1024 * 1024;
const HOME_DIR_PREFIX: &str = "~";

impl ArchiveCfg {
    fn default_enabled() -> bool {
        true
    }

    fn default_dir() -> PathBuf {
        PathBuf::from(DEFAULT_ARCHIVE_DIR)
    }

    fn default_max_size_mb() -> u64 {
        DEFAULT_ARCHIVE_MAX_SIZE_MB
    }

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
            enabled: Self::default_enabled(),
            dir: Self::default_dir(),
            max_size_mb: Self::default_max_size_mb(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingCfg {
    #[serde(default = "LoggingCfg::default_level")]
    pub level: String,
}

impl LoggingCfg {
    fn default_level() -> String {
        "info".into()
    }
}

impl Default for LoggingCfg {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
        }
    }
}

pub fn load(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read config: {}", path.display()))?;
    parse(&raw, &path.display().to_string())
}

/// The configuration `raw` holds, checked as a file is; `origin` names it in errors.
pub fn parse(raw: &str, origin: &str) -> Result<Config> {
    let table: toml::Table =
        toml::from_str(raw).with_context(|| format!("parse config: {origin}"))?;
    transcription::reject_removed_whisper_section(&table)
        .and_then(|()| transcription::reject_removed_transcription_keys(&table))
        .and_then(|()| refinement::reject_removed_llm_section(&table))
        .with_context(|| format!("outdated config: {origin}"))?;
    let cfg: Config = toml::from_str(raw).with_context(|| format!("parse config: {origin}"))?;
    validate(&cfg).with_context(|| format!("invalid config: {origin}"))?;
    Ok(cfg)
}

/// Checks rules that span the effective config; run again after CLI flags are overlaid.
pub fn validate(cfg: &Config) -> Result<()> {
    ensure_loopback("transcription.base_url", &cfg.transcription.base_url)?;
    ensure_loopback("refinement.base_url", &cfg.refinement.base_url)?;
    ensure_loopback_websocket("voice_control.url", &cfg.voice_control.url)
}

#[cfg(test)]
mod tests {
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
        let archive = ArchiveCfg {
            dir: "/tmp/dictations".into(),
            ..ArchiveCfg::default()
        };
        // When
        let dir = archive.resolved_dir(Path::new("/Users/someone"));
        // Then
        assert_eq!(dir, Path::new("/tmp/dictations"));
    }

    #[test]
    fn should_still_accept_the_retired_debug_save_wav_key() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[audio]\ndebug_save_wav = true").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert!(cfg.audio.debug_save_wav);
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
    fn should_report_removed_llm_section_with_replacement() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[llm]\nmodel_path = \"~/models/qwen.gguf\"").unwrap();
        // When
        let message = format!("{:#}", load(f.path()).unwrap_err());
        // Then
        assert!(message.contains("[refinement]"), "{message}");
        assert!(message.contains("llm.model_path"), "{message}");
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
    fn should_parse_refinement_section_from_toml() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            f,
            "[refinement]\nbase_url = \"http://localhost:9090/v1\"\nmodel = \"qwen\"\n\
             api_key = \"secret\"\nsystem_prompt = \"Fix it.\"\ntemperature = 0.1\n\
             max_tokens = 256\ntimeout_secs = 5"
        )
        .unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        let r = &cfg.refinement;
        assert_eq!(r.base_url, "http://localhost:9090/v1");
        assert_eq!(r.model, "qwen");
        assert_eq!(r.api_key, "secret");
        assert_eq!(r.system_prompt, "Fix it.");
        assert_eq!(r.temperature, 0.1_f32);
        assert_eq!(r.max_tokens, 256);
        assert_eq!(r.timeout_secs, 5);
    }

    #[test]
    fn should_keep_refinement_off_by_default() {
        // Given / When
        let cfg = Config::default();
        // Then
        assert!(!cfg.refinement.enabled);
    }

    #[test]
    fn should_turn_refinement_on_from_toml() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[refinement]\nenabled = true").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert!(cfg.refinement.enabled);
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
    fn should_reject_remote_refinement_service_at_load() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[refinement]\nbase_url = \"https://api.openai.com/v1\"").unwrap();
        // When
        let message = format!("{:#}", load(f.path()).unwrap_err());
        // Then
        assert!(message.contains("refinement.base_url"), "{message}");
        assert!(message.contains("not on this machine"), "{message}");
    }

    #[test]
    fn should_reject_remote_refinement_service_set_after_load() {
        // Given
        let mut cfg = Config::default();
        cfg.refinement.base_url = "http://192.168.1.10:8080/v1".into();
        // When
        let message = format!("{:#}", validate(&cfg).unwrap_err());
        // Then
        assert!(message.contains("refinement.base_url"), "{message}");
    }

    #[test]
    fn should_accept_default_service_addresses() {
        assert!(validate(&Config::default()).is_ok());
    }
}
