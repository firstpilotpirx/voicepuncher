use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionCfg {
    /// Base URL of the OpenAI-compatible speech recognition service; must be on this machine.
    #[serde(default = "TranscriptionCfg::default_base_url")]
    pub base_url: String,
    /// Model name sent with every request; servers that host a single model ignore it.
    #[serde(default = "TranscriptionCfg::default_model")]
    pub model: String,
    /// Spoken language code (`ru`, `en`, …) or `auto` to let the service detect it.
    #[serde(default = "TranscriptionCfg::default_language")]
    pub language: String,
    /// Prompt that biases recognition toward domain terms and punctuation style.
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub temperature: f32,
    /// Upper bound for one transcription request, including upload and recognition.
    #[serde(default = "TranscriptionCfg::default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "TranscriptionCfg::default_chunk_threshold_secs")]
    pub chunk_threshold_secs: u32,
    #[serde(default = "TranscriptionCfg::default_chunk_overlap_secs")]
    pub chunk_overlap_secs: u32,
    #[serde(default = "TranscriptionCfg::default_vad_rms_threshold")]
    pub vad_rms_threshold: f32,
    #[serde(default = "TranscriptionCfg::default_vad_lookback_secs")]
    pub vad_lookback_secs: u32,
}

impl TranscriptionCfg {
    pub const AUTO_LANGUAGE: &'static str = "auto";

    fn default_base_url() -> String {
        "http://127.0.0.1:8081/v1".into()
    }
    fn default_model() -> String {
        "whisper-1".into()
    }
    fn default_language() -> String {
        Self::AUTO_LANGUAGE.into()
    }
    fn default_timeout_secs() -> u64 {
        60
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

impl Default for TranscriptionCfg {
    fn default() -> Self {
        Self {
            base_url: Self::default_base_url(),
            model: Self::default_model(),
            language: Self::default_language(),
            prompt: String::new(),
            temperature: 0.0,
            timeout_secs: Self::default_timeout_secs(),
            chunk_threshold_secs: Self::default_chunk_threshold_secs(),
            chunk_overlap_secs: Self::default_chunk_overlap_secs(),
            vad_rms_threshold: Self::default_vad_rms_threshold(),
            vad_lookback_secs: Self::default_vad_lookback_secs(),
        }
    }
}

const RENAMED_SECTION: &str = "whisper";

const MOVED_TO_SERVICE: &[(&str, &str)] = &[
    ("model_path", "choose the model when starting the speech recognition service (whisper-server -m <model>) and point [transcription].base_url at it"),
    ("threads", "set it when starting the service (whisper-server -t)"),
    ("beam_size", "set it when starting the service (whisper-server -bs)"),
    ("patience", "set it when starting the service"),
    ("temperature_inc", "set it when starting the service (whisper-server -nf disables the fallback)"),
    ("entropy_thold", "set it when starting the service (whisper-server -et)"),
    ("logprob_thold", "set it when starting the service (whisper-server -lpt)"),
    ("no_speech_thold", "set it when starting the service (whisper-server -nth)"),
    ("flash_attn", "set it when starting the service (whisper-server -fa / -nfa)"),
];

const RENAMED_KEYS: &[(&str, &str)] = &[("initial_prompt", "prompt")];

/// Explains how to migrate a config that still has the removed `[whisper]` section.
/// Returns `Ok(())` when the section is absent.
pub fn reject_removed_whisper_section(raw: &toml::Table) -> Result<()> {
    let Some(section) = raw.get(RENAMED_SECTION) else {
        return Ok(());
    };
    let keys = section
        .as_table()
        .ok_or_else(|| anyhow!("[{RENAMED_SECTION}] must be a table"))?
        .keys();
    let hints: Vec<String> = keys
        .map(|key| format!("  - whisper.{key}: {}", replacement_for(key)))
        .collect();
    bail!(
        "the [whisper] section was replaced by [transcription]; speech recognition now runs \
         in a separate service (start it with 'just start-model-servers'):\n{}",
        hints.join("\n")
    )
}

fn replacement_for(key: &str) -> String {
    if let Some((_, hint)) = MOVED_TO_SERVICE.iter().find(|(old, _)| *old == key) {
        return (*hint).to_owned();
    }
    let new_key = RENAMED_KEYS
        .iter()
        .find(|(old, _)| *old == key)
        .map_or(key, |(_, new)| *new);
    format!("move it to [transcription].{new_key}")
}

#[cfg(test)]
mod tests {
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
        assert_eq!(cfg.vad_rms_threshold, 0.01_f32);
        assert_eq!(cfg.vad_lookback_secs, 5);
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
            "vad_rms_threshold",
            "vad_lookback_secs",
        ] {
            let message = migration_error(&format!("[whisper]\n{key} = 1\n"));
            assert!(
                message.contains(&format!("[transcription].{key}")),
                "{message}"
            );
        }
    }
}
