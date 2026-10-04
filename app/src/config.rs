use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::core::domain::{HotKey, RecordingMode};

mod transcription;

pub use transcription::{ensure_loopback, TranscriptionCfg};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub hotkey: HotkeyCfg,
    #[serde(default)]
    pub audio: AudioCfg,
    #[serde(default)]
    pub transcription: TranscriptionCfg,
    #[serde(default)]
    pub llm: LlmCfg,
    #[serde(default)]
    pub paste: PasteCfg,
    #[serde(default)]
    pub sounds: SoundsCfg,
    #[serde(default)]
    pub logging: LoggingCfg,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCfg {
    #[serde(default = "AudioCfg::default_input_device")]
    pub input_device: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmCfg {
    #[serde(default)]
    pub model_path: PathBuf,
    #[serde(default = "LlmCfg::default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "LlmCfg::default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default = "LlmCfg::default_context_size")]
    pub context_size: u32,
}

impl LlmCfg {
    fn default_system_prompt() -> String {
        "You are a speech-to-text post-processor. Your ONLY job is to fix ASR recognition errors: \
         wrong words, technical terms transcribed in the wrong alphabet, \
         words accidentally merged or split. \
         CRITICAL RULES — NEVER BREAK THESE: \
         (1) NEVER shorten, cut, truncate, abbreviate, or summarize. \
         NEVER use ellipsis (...). NEVER omit any word. \
         Output the COMPLETE text — every single word, no exceptions. \
         (2) Do NOT rewrite, rephrase, restructure, or improve style. Fix only broken words. \
         (3) Preserve the original language exactly — do not translate between languages. \
         If the input contains multiple languages or code-switching (e.g. Russian + English), \
         preserve each language exactly as spoken. \
         (4) Fix technical terms to their correct script \
         (e.g. Docker, API, Kubernetes in English even inside Russian text). \
         (5) If nothing needs fixing, copy the input verbatim — every word, every number. \
         Output only the corrected text, nothing else."
            .into()
    }
    fn default_max_tokens() -> u32 {
        512
    }
    fn default_context_size() -> u32 {
        4096
    }
}

impl Default for LlmCfg {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            system_prompt: Self::default_system_prompt(),
            max_tokens: Self::default_max_tokens(),
            temperature: 0.0,
            context_size: Self::default_context_size(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasteCfg {
    #[serde(default = "PasteCfg::default_clipboard_restore_delay_ms")]
    pub clipboard_restore_delay_ms: u64,
}

impl PasteCfg {
    fn default_clipboard_restore_delay_ms() -> u64 {
        120
    }
}

impl Default for PasteCfg {
    fn default() -> Self {
        Self {
            clipboard_restore_delay_ms: Self::default_clipboard_restore_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundsCfg {
    #[serde(default = "SoundsCfg::default_start_sound")]
    pub start_sound: String,
    #[serde(default = "SoundsCfg::default_stop_sound")]
    pub stop_sound: String,
    #[serde(default = "SoundsCfg::default_cancel_sound")]
    pub cancel_sound: String,
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
}

impl Default for SoundsCfg {
    fn default() -> Self {
        Self {
            start_sound: Self::default_start_sound(),
            stop_sound: Self::default_stop_sound(),
            cancel_sound: Self::default_cancel_sound(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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
    let table: toml::Table =
        toml::from_str(&raw).with_context(|| format!("parse config: {}", path.display()))?;
    transcription::reject_removed_whisper_section(&table)
        .with_context(|| format!("outdated config: {}", path.display()))?;
    let mut cfg: Config =
        toml::from_str(&raw).with_context(|| format!("parse config: {}", path.display()))?;
    expand_tilde(&mut cfg.llm.model_path);
    validate(&cfg).with_context(|| format!("invalid config: {}", path.display()))?;
    Ok(cfg)
}

/// Checks rules that span the effective config; run again after CLI flags are overlaid.
pub fn validate(cfg: &Config) -> Result<()> {
    ensure_loopback(&cfg.transcription.base_url)
}

fn expand_tilde(p: &mut PathBuf) {
    if let Ok(s) = p.to_path_buf().into_os_string().into_string() {
        if let Some(rest) = s.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                *p = home.join(rest);
            }
        } else if s == "~" {
            if let Some(home) = dirs::home_dir() {
                *p = home;
            }
        }
    }
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
    fn should_expand_tilde_in_model_path() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "[llm]\nmodel_path = \"~/models/qwen.gguf\"").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        let home = dirs::home_dir().unwrap();
        assert_eq!(cfg.llm.model_path, home.join("models/qwen.gguf"));
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
}
