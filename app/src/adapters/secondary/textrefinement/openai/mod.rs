use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::config::RefinementCfg;
use crate::core::application::text_refiner::TextRefiner;
use crate::core::domain::{CleanedText, RawTranscript};

mod request;

use request::{chat_body, cleaned_answer};

const CHAT_COMPLETIONS_PATH: &str = "chat/completions";
const MODELS_PATH: &str = "models";
const STARTUP_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const START_HINT: &str = "start it with 'just start-model-servers'";

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

/// Corrects a raw transcript through a local text refinement service that speaks the
/// OpenAI Chat Completions API (llama.cpp `llama-server`).
pub struct OpenAiTextRefiner {
    agent: ureq::Agent,
    endpoint: String,
    cfg: RefinementCfg,
}

impl OpenAiTextRefiner {
    /// Checks that the service answers and offers the configured model, then returns a
    /// refiner for it. A service that offers a single model is accepted whatever its name.
    pub fn connect(cfg: RefinementCfg) -> Result<Self> {
        let base_url = cfg.base_url.trim_end_matches('/').to_owned();
        let probe = ureq::AgentBuilder::new()
            .timeout(STARTUP_CHECK_TIMEOUT)
            .build();
        let offered = offered_models(&probe, &base_url, &cfg.api_key)?;
        if offered.len() > 1 && !offered.contains(&cfg.model) {
            bail!(
                "text refinement service at {base_url} does not offer model '{}' — set \
                 [refinement].model to one of: {}",
                cfg.model,
                offered.join(", ")
            );
        }
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build();
        Ok(Self {
            agent,
            endpoint: format!("{base_url}/{CHAT_COMPLETIONS_PATH}"),
            cfg,
        })
    }
}

fn offered_models(agent: &ureq::Agent, base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let response = with_api_key(agent.get(&format!("{base_url}/{MODELS_PATH}")), api_key)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => anyhow!(
                "text refinement service at {base_url} is not ready (HTTP {code}) — wait until \
                 it has loaded its model, or {START_HINT}"
            ),
            ureq::Error::Transport(_) => {
                anyhow!("text refinement service is not reachable at {base_url} — {START_HINT}")
            }
        })?;
    let unexpected =
        || format!("text refinement service at {base_url} did not answer with a model list");
    let json = response.into_string().with_context(unexpected)?;
    let list: ModelList = serde_json::from_str(&json).with_context(unexpected)?;
    Ok(list.data.into_iter().map(|m| m.id).collect())
}

fn with_api_key(request: ureq::Request, api_key: &str) -> ureq::Request {
    if api_key.is_empty() {
        request
    } else {
        request.set("Authorization", &format!("Bearer {api_key}"))
    }
}

impl TextRefiner for OpenAiTextRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        let body = chat_body(&self.cfg, raw.as_str());
        let response = with_api_key(self.agent.post(&self.endpoint), &self.cfg.api_key)
            .set("Content-Type", "application/json")
            .send_string(&body)
            .map_err(|e| anyhow!("text refinement request to {} failed: {e}", self.endpoint))?;
        let json = response
            .into_string()
            .context("read text refinement response")?;
        let text = cleaned_answer(&json, raw.as_str())?;
        CleanedText::new(text).map_err(|e| anyhow!(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    /// Answers every request with `status` and `body`; returns the service address.
    fn spawn_models_stub(status: u16, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 2 {
                    line.clear();
                }
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        format!("http://127.0.0.1:{port}/v1")
    }

    fn cfg(base_url: &str, model: &str) -> RefinementCfg {
        RefinementCfg {
            base_url: base_url.to_owned(),
            model: model.to_owned(),
            ..RefinementCfg::default()
        }
    }

    fn connect_error(base_url: &str, model: &str) -> String {
        format!(
            "{:#}",
            OpenAiTextRefiner::connect(cfg(base_url, model))
                .err()
                .unwrap()
        )
    }

    const ONE_MODEL: &str = r#"{"object":"list","data":[{"id":"/models/qwen.gguf"}]}"#;
    const TWO_MODELS: &str = r#"{"object":"list","data":[{"id":"qwen2.5"},{"id":"llama3"}]}"#;

    #[test]
    fn should_connect_to_single_model_service_whatever_the_model_name() {
        // Given
        let url = spawn_models_stub(200, ONE_MODEL);
        // When
        let refiner = OpenAiTextRefiner::connect(cfg(&url, "local")).unwrap();
        // Then
        assert_eq!(refiner.endpoint, format!("{url}/chat/completions"));
    }

    #[test]
    fn should_connect_when_multi_model_service_offers_configured_model() {
        let url = spawn_models_stub(200, TWO_MODELS);
        assert!(OpenAiTextRefiner::connect(cfg(&url, "llama3")).is_ok());
    }

    #[test]
    fn should_list_offered_models_when_configured_model_is_missing() {
        // Given
        let url = spawn_models_stub(200, TWO_MODELS);
        // When
        let message = connect_error(&url, "mistral");
        // Then
        assert!(message.contains("'mistral'"), "{message}");
        assert!(message.contains("qwen2.5, llama3"), "{message}");
    }

    #[test]
    fn should_report_address_and_start_command_when_service_is_not_running() {
        // Given
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let url = format!("http://127.0.0.1:{port}/v1");
        // When
        let message = connect_error(&url, "local");
        // Then
        assert!(
            message.contains("text refinement service is not reachable"),
            "{message}"
        );
        assert!(message.contains(&url), "{message}");
        assert!(message.contains("just start-model-servers"), "{message}");
    }

    #[test]
    fn should_report_address_when_service_is_not_ready() {
        let url = spawn_models_stub(503, r#"{"error":"Loading model"}"#);
        let message = connect_error(&url, "local");
        assert!(message.contains(&url), "{message}");
    }

    #[test]
    fn should_report_address_when_answer_is_not_a_model_list() {
        let url = spawn_models_stub(200, "<html>hello</html>");
        let message = connect_error(&url, "local");
        assert!(message.contains(&url), "{message}");
    }
}
