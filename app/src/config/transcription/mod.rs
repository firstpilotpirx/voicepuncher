use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    /// The user's spoken languages; with `auto`, a transcript in another language is requested
    /// again with the first of them fixed.
    #[serde(
        default = "TranscriptionCfg::default_languages",
        deserialize_with = "known_languages"
    )]
    pub languages: Vec<String>,
    /// Prompt that biases recognition toward domain terms and punctuation style, used in every
    /// 30 s window of every chunk.
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
    fn default_languages() -> Vec<String> {
        DEFAULT_LANGUAGES
            .iter()
            .map(|&code| code.to_owned())
            .collect()
    }

    /// Every language code `languages` accepts, with the name whisper gives it.
    pub fn language_names() -> &'static [(&'static str, &'static str)] {
        LANGUAGE_NAMES
    }

    /// The name the speech recognition service gives the language of `code` ("ru" →
    /// "russian"), for a code `languages` accepts.
    pub fn language_name(code: &str) -> Option<&'static str> {
        LANGUAGE_NAMES
            .iter()
            .find(|(known, _)| *known == code)
            .map(|(_, name)| *name)
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
            languages: Self::default_languages(),
            prompt: String::new(),
            temperature: 0.0,
            timeout_secs: Self::default_timeout_secs(),
            chunk_threshold_secs: Self::default_chunk_threshold_secs(),
            chunk_overlap_secs: Self::default_chunk_overlap_secs(),
            vad_lookback_secs: Self::default_vad_lookback_secs(),
        }
    }
}

const DEFAULT_LANGUAGES: [&str; 2] = ["ru", "en"];

/// The language codes `languages` accepts, with the name whisper gives each language.
const LANGUAGE_NAMES: &[(&str, &str)] = &[
    ("ru", "russian"),
    ("en", "english"),
    ("uk", "ukrainian"),
    ("be", "belarusian"),
    ("kk", "kazakh"),
    ("de", "german"),
    ("fr", "french"),
    ("es", "spanish"),
    ("it", "italian"),
    ("pt", "portuguese"),
    ("nl", "dutch"),
    ("pl", "polish"),
    ("cs", "czech"),
    ("tr", "turkish"),
    ("zh", "chinese"),
    ("ja", "japanese"),
    ("ko", "korean"),
];

fn known_languages<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<String>, D::Error> {
    let codes = Vec::<String>::deserialize(deserializer)?;
    if codes.is_empty() {
        return Err(serde::de::Error::custom(
            "languages must list at least one language code",
        ));
    }
    if let Some(unknown) = codes
        .iter()
        .find(|code| TranscriptionCfg::language_name(code).is_none())
    {
        let known: Vec<&str> = LANGUAGE_NAMES.iter().map(|(code, _)| *code).collect();
        return Err(serde::de::Error::custom(format!(
            "unknown language code \"{unknown}\" in languages; known codes: {}",
            known.join(", ")
        )));
    }
    Ok(codes)
}

const RENAMED_SECTION: &str = "whisper";

const MOVED_TO_SERVICE: &[(&str, &str)] = &[
    ("model_path", "choose the model when starting the speech recognition service (whisper-server -m <model>) and point [transcription].base_url at it"),
    ("threads", "set it when starting the service (whisper-server -t)"),
    ("beam_size", "set it when starting the service (whisper-server -bs)"),
    ("patience", "set it when starting the service"),
    ("temperature_inc", "remove it; no flag sets it when starting the service: whisper-server 1.9.5 ignores -nf and keeps its own temperature fallback"),
    ("entropy_thold", "set it when starting the service (whisper-server -et)"),
    ("logprob_thold", "set it when starting the service (whisper-server -lpt)"),
    ("no_speech_thold", "set it when starting the service (whisper-server -nth)"),
    ("flash_attn", "set it when starting the service (whisper-server -fa / -nfa)"),
];

const RENAMED_KEYS: &[(&str, &str)] = &[("initial_prompt", "prompt")];

const SECTION: &str = "transcription";

/// Keys the app no longer reads, and what to do instead.
const REMOVED_KEYS: &[(&str, &str)] = &[(
    "vad_rms_threshold",
    "delete it: chunks are now cut in pauses judged against the recording's own background noise",
)];

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

/// Explains how to migrate a `[transcription]` section that still sets a removed key.
/// Returns `Ok(())` when it sets none.
pub fn reject_removed_transcription_keys(raw: &toml::Table) -> Result<()> {
    let Some(section) = raw.get(SECTION).and_then(toml::Value::as_table) else {
        return Ok(());
    };
    let hints: Vec<String> = REMOVED_KEYS
        .iter()
        .filter(|(key, _)| section.contains_key(*key))
        .map(|(key, hint)| format!("  - {SECTION}.{key}: {hint}"))
        .collect();
    if hints.is_empty() {
        return Ok(());
    }
    bail!("settings no longer used:\n{}", hints.join("\n"))
}

fn replacement_for(key: &str) -> String {
    if let Some((_, hint)) = REMOVED_KEYS.iter().find(|(old, _)| *old == key) {
        return (*hint).to_owned();
    }
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
mod tests;
