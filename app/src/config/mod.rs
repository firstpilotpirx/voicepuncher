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
    /// Whether the speech recognition service for commands confirms a command before it is
    /// carried out; off, the keyword spotter alone decides and Whisper's verdict is logged.
    pub confirm_commands: bool,
}

impl Default for VoiceControlCfg {
    fn default() -> Self {
        Self { enabled: false, url: "ws://127.0.0.1:8082".into(), confirm_commands: true }
    }
}

/// The dictation archive: where each dictation's audio and texts are kept, and how much.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchiveCfg {
    pub enabled: bool,
    pub dir: PathBuf,
    pub max_size_mb: u64,
    /// Keep every voice command in the command history, written by the command spotting
    /// services the app starts.
    pub keep_commands: bool,
}

const DEFAULT_ARCHIVE_DIR: &str = "~/.voicepuncher/history";
const DEFAULT_ARCHIVE_MAX_SIZE_MB: u64 = 2048;
/// The command history's folder, under the home folder beside the dictation archive's.
const COMMAND_HISTORY_DIR: &str = ".voicepuncher/command-history";
const BYTES_PER_MEGABYTE: u64 = 1024 * 1024;
const HOME_DIR_PREFIX: &str = "~";

impl ArchiveCfg {
    /// The size limit of the archive in bytes.
    pub fn max_size_bytes(&self) -> u64 {
        self.max_size_mb.saturating_mul(BYTES_PER_MEGABYTE)
    }

    /// The command history's folder under `home`; none when commands are not kept.
    pub fn command_history_dir(&self, home: &Path) -> Option<PathBuf> {
        self.keep_commands.then(|| home.join(COMMAND_HISTORY_DIR))
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
            keep_commands: true,
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
pub const DEFAULT_FILE: &str = include_str!("default.toml");

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
mod tests;
