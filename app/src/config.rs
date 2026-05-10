use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::core::domain::{HotKey, RecordingMode};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub hotkey: HotkeyCfg,
    #[serde(default)]
    pub audio: AudioCfg,
    #[serde(default)]
    pub whisper: WhisperCfg,
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
pub struct WhisperCfg {
    #[serde(default)]
    pub model_path: PathBuf,
    #[serde(default = "WhisperCfg::default_language")]
    pub language: String,
    #[serde(default)]
    pub threads: usize,
    /// Beam size for beam search decoding. 1 = greedy (fastest). 5 = good quality/speed tradeoff.
    #[serde(default = "WhisperCfg::default_beam_size")]
    pub beam_size: i32,
    /// Beam search patience factor. Higher = more thorough, slower.
    #[serde(default = "WhisperCfg::default_patience")]
    pub patience: f32,
    /// Starting temperature for decoding. Raised by temperature_inc on fallback.
    #[serde(default)]
    pub temperature: f32,
    /// Temperature increment used when resampling due to failed quality checks.
    #[serde(default = "WhisperCfg::default_temperature_inc")]
    pub temperature_inc: f32,
    /// Entropy (compression ratio) threshold for fallback resampling.
    #[serde(default = "WhisperCfg::default_entropy_thold")]
    pub entropy_thold: f32,
    /// Log-probability threshold for fallback resampling.
    #[serde(default = "WhisperCfg::default_logprob_thold")]
    pub logprob_thold: f32,
    /// No-speech probability threshold. Higher = more aggressive silence detection.
    #[serde(default = "WhisperCfg::default_no_speech_thold")]
    pub no_speech_thold: f32,
    /// Optional initial prompt fed to the decoder (domain terms, punctuation style, etc.).
    #[serde(default)]
    pub initial_prompt: String,
    #[serde(default = "WhisperCfg::default_flash_attn")]
    pub flash_attn: bool,
    #[serde(default = "WhisperCfg::default_chunk_threshold_secs")]
    pub chunk_threshold_secs: u32,
    #[serde(default = "WhisperCfg::default_chunk_overlap_secs")]
    pub chunk_overlap_secs: u32,
    #[serde(default = "WhisperCfg::default_vad_rms_threshold")]
    pub vad_rms_threshold: f32,
    #[serde(default = "WhisperCfg::default_vad_lookback_secs")]
    pub vad_lookback_secs: u32,
}

impl WhisperCfg {
    fn default_language() -> String {
        "ru".into()
    }
    fn default_beam_size() -> i32 {
        1
    }
    fn default_patience() -> f32 {
        1.0
    }
    fn default_temperature_inc() -> f32 {
        0.0
    }
    fn default_flash_attn() -> bool {
        true
    }
    fn default_entropy_thold() -> f32 {
        2.4
    }
    fn default_logprob_thold() -> f32 {
        -1.0
    }
    fn default_no_speech_thold() -> f32 {
        0.6
    }
    fn default_chunk_threshold_secs() -> u32 {
        30
    }
    fn default_chunk_overlap_secs() -> u32 {
        2
    }
    fn default_vad_rms_threshold() -> f32 {
        0.01
    }
    fn default_vad_lookback_secs() -> u32 {
        5
    }
}

impl Default for WhisperCfg {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            language: Self::default_language(),
            threads: 0,
            beam_size: Self::default_beam_size(),
            patience: Self::default_patience(),
            temperature: 0.0,
            temperature_inc: Self::default_temperature_inc(),
            entropy_thold: Self::default_entropy_thold(),
            logprob_thold: Self::default_logprob_thold(),
            no_speech_thold: Self::default_no_speech_thold(),
            initial_prompt: String::new(),
            flash_attn: Self::default_flash_attn(),
            chunk_threshold_secs: Self::default_chunk_threshold_secs(),
            chunk_overlap_secs: Self::default_chunk_overlap_secs(),
            vad_rms_threshold: Self::default_vad_rms_threshold(),
            vad_lookback_secs: Self::default_vad_lookback_secs(),
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
         (3) Preserve the original language — do not translate. \
         (4) Fix technical terms to their correct script \
         (e.g. Docker, API in English even inside Russian text). \
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
    let mut cfg: Config =
        toml::from_str(&raw).with_context(|| format!("parse config: {}", path.display()))?;
    expand_tilde(&mut cfg.whisper.model_path);
    expand_tilde(&mut cfg.llm.model_path);
    Ok(cfg)
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
        writeln!(f, "[whisper]\nmodel_path = \"~/models/ggml.bin\"").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        let home = dirs::home_dir().unwrap();
        assert_eq!(cfg.whisper.model_path, home.join("models/ggml.bin"));
    }

    #[test]
    fn should_default_flash_attn_to_true() {
        assert!(WhisperCfg::default().flash_attn);
    }

    #[test]
    fn should_default_language_to_ru() {
        assert_eq!(WhisperCfg::default().language, "ru");
    }

    #[test]
    fn should_default_beam_size_to_one() {
        assert_eq!(WhisperCfg::default().beam_size, 1);
    }

    #[test]
    fn should_default_temperature_inc_to_zero() {
        assert_eq!(WhisperCfg::default().temperature_inc, 0.0_f32);
    }

    #[test]
    fn should_parse_flash_attn_from_toml() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"[whisper]\nflash_attn = false\n").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert!(!cfg.whisper.flash_attn);
    }

    #[test]
    fn should_default_flash_attn_when_absent_from_toml() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"[whisper]\nmodel_path = \"/tmp/model.bin\"\n").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert!(cfg.whisper.flash_attn);
    }

    #[test]
    fn should_default_chunk_threshold_secs_to_30() {
        // Given
        // When
        let cfg = WhisperCfg::default();
        // Then
        assert_eq!(cfg.chunk_threshold_secs, 30);
    }

    #[test]
    fn should_default_chunk_overlap_secs_to_2() {
        // Given
        // When
        let cfg = WhisperCfg::default();
        // Then
        assert_eq!(cfg.chunk_overlap_secs, 2);
    }

    #[test]
    fn should_default_vad_rms_threshold_to_point_zero_one() {
        // Given
        // When
        let cfg = WhisperCfg::default();
        // Then
        assert_eq!(cfg.vad_rms_threshold, 0.01_f32);
    }

    #[test]
    fn should_default_vad_lookback_secs_to_5() {
        // Given
        // When
        let cfg = WhisperCfg::default();
        // Then
        assert_eq!(cfg.vad_lookback_secs, 5);
    }

    #[test]
    fn should_parse_chunk_threshold_secs_from_toml() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"[whisper]\nchunk_threshold_secs = 60\n").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert_eq!(cfg.whisper.chunk_threshold_secs, 60);
    }

    #[test]
    fn should_default_chunk_fields_when_absent_from_toml() {
        // Given
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"[whisper]\nmodel_path = \"/tmp/m\"\n").unwrap();
        // When
        let cfg = load(f.path()).unwrap();
        // Then
        assert_eq!(cfg.whisper.chunk_threshold_secs, 30);
        assert_eq!(cfg.whisper.chunk_overlap_secs, 2);
        assert_eq!(cfg.whisper.vad_rms_threshold, 0.01_f32);
        assert_eq!(cfg.whisper.vad_lookback_secs, 5);
    }
}
