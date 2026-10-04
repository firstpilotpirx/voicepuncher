use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementCfg {
    /// Base URL of the OpenAI-compatible text refinement service; must be on this machine.
    #[serde(default = "RefinementCfg::default_base_url")]
    pub base_url: String,
    /// Model name sent with every request; a service that hosts a single model ignores it.
    #[serde(default = "RefinementCfg::default_model")]
    pub model: String,
    /// Sent as a bearer token when not empty (`llama-server --api-key`).
    #[serde(default)]
    pub api_key: String,
    /// Instruction for the model; the raw transcript is sent apart from it as delimited data.
    #[serde(default = "RefinementCfg::default_system_prompt")]
    pub system_prompt: String,
    #[serde(default)]
    pub temperature: f32,
    /// Upper bound for the cleaned text's length in tokens.
    #[serde(default = "RefinementCfg::default_max_tokens")]
    pub max_tokens: u32,
    /// Upper bound for one refinement request.
    #[serde(default = "RefinementCfg::default_timeout_secs")]
    pub timeout_secs: u64,
}

impl RefinementCfg {
    fn default_base_url() -> String {
        "http://127.0.0.1:8080/v1".into()
    }
    fn default_model() -> String {
        "local".into()
    }
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
    fn default_timeout_secs() -> u64 {
        30
    }
}

impl Default for RefinementCfg {
    fn default() -> Self {
        Self {
            base_url: Self::default_base_url(),
            model: Self::default_model(),
            api_key: String::new(),
            system_prompt: Self::default_system_prompt(),
            temperature: 0.0,
            max_tokens: Self::default_max_tokens(),
            timeout_secs: Self::default_timeout_secs(),
        }
    }
}

const REMOVED_SECTION: &str = "llm";

const MOVED_TO_SERVICE: &[(&str, &str)] = &[
    ("model_path", "choose the model when starting the text refinement service (llama-server -m <model>) and point [refinement].base_url at it"),
    ("context_size", "set it when starting the service (llama-server -c)"),
];

/// Explains how to migrate a config that still has the removed `[llm]` section.
/// Returns `Ok(())` when the section is absent.
pub fn reject_removed_llm_section(raw: &toml::Table) -> Result<()> {
    let Some(section) = raw.get(REMOVED_SECTION) else {
        return Ok(());
    };
    let keys = section
        .as_table()
        .ok_or_else(|| anyhow!("[{REMOVED_SECTION}] must be a table"))?
        .keys();
    let hints: Vec<String> = keys
        .map(|key| format!("  - llm.{key}: {}", replacement_for(key)))
        .collect();
    bail!(
        "the [llm] section was replaced by [refinement]; text refinement now runs in a \
         separate service (start it with 'just start-model-servers'):\n{}",
        hints.join("\n")
    )
}

fn replacement_for(key: &str) -> String {
    MOVED_TO_SERVICE
        .iter()
        .find(|(old, _)| *old == key)
        .map_or_else(
            || format!("move it to [refinement].{key}"),
            |(_, hint)| (*hint).to_owned(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migration_error(toml_str: &str) -> String {
        let raw: toml::Table = toml::from_str(toml_str).unwrap();
        format!("{:#}", reject_removed_llm_section(&raw).unwrap_err())
    }

    #[test]
    fn should_accept_config_without_llm_section() {
        let raw: toml::Table = toml::from_str("[refinement]\nmodel = \"qwen\"\n").unwrap();
        assert!(reject_removed_llm_section(&raw).is_ok());
    }

    #[test]
    fn should_name_service_and_address_setting_for_model_path() {
        // Given / When
        let message = migration_error("[llm]\nmodel_path = \"/m.gguf\"\n");
        // Then
        assert!(message.contains("llm.model_path"), "{message}");
        assert!(
            message.contains("starting the text refinement service"),
            "{message}"
        );
        assert!(message.contains("[refinement].base_url"), "{message}");
    }

    #[test]
    fn should_name_service_flag_for_context_size() {
        let message = migration_error("[llm]\ncontext_size = 4096\n");
        assert!(message.contains("llm.context_size"), "{message}");
        assert!(message.contains("llama-server -c"), "{message}");
    }

    #[test]
    fn should_name_same_key_in_refinement_for_kept_keys() {
        for key in ["system_prompt", "max_tokens", "temperature"] {
            let message = migration_error(&format!("[llm]\n{key} = 1\n"));
            assert!(
                message.contains(&format!("[refinement].{key}")),
                "{message}"
            );
        }
    }

    #[test]
    fn should_default_to_local_service_on_port_8080() {
        assert_eq!(
            RefinementCfg::default().base_url,
            "http://127.0.0.1:8080/v1"
        );
    }

    #[test]
    fn should_default_model_api_key_and_sampling() {
        // Given / When
        let cfg = RefinementCfg::default();
        // Then
        assert_eq!(cfg.model, "local");
        assert_eq!(cfg.api_key, "");
        assert_eq!(cfg.temperature, 0.0_f32);
        assert_eq!(cfg.max_tokens, 512);
        assert_eq!(cfg.timeout_secs, 30);
    }

    #[test]
    fn should_default_system_prompt_to_correction_only_instructions() {
        let prompt = RefinementCfg::default().system_prompt;
        assert!(prompt.starts_with("You are a speech-to-text post-processor."));
        assert!(prompt.contains("do not translate between languages"));
    }
}
