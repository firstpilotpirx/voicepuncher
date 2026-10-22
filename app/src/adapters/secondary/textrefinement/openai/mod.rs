use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use voicepuncher_copy_pool::{CopyPool, PoolError, ServiceAddress, TEXT_REFINEMENT_COPY_OFFSET};

use crate::adapters::secondary::http::agent;
use crate::adapters::secondary::http::failure::Failure;
use crate::config::RefinementCfg;
use crate::core::application::text_refiner::TextRefiner;
use crate::core::domain::{CleanedText, RawTranscript};

mod request;

use request::{chat_body, cleaned_answer};

const CHAT_COMPLETIONS_PATH: &str = "chat/completions";
const MODELS_PATH: &str = "models";
const STARTUP_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const START_HINT: &str = "start it with 'just start-model-servers'";
/// The pause before the one new attempt after the service dropped the connection.
const DROPPED_RETRY_PAUSE: Duration = Duration::from_millis(200);
/// How long a refinement waits for a copy to come back when no copy accepts connections:
/// the app restarts a stopped copy within seconds.
pub const RECOVERY_WAIT: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

/// Corrects a raw transcript through a local text refinement service that speaks the
/// `OpenAI` Chat Completions API (llama.cpp `llama-server`), run as one or several copies.
pub struct OpenAiTextRefiner {
    agent: ureq::Agent,
    pool: CopyPool,
    cfg: RefinementCfg,
    timeout: Duration,
    recovery_wait: Duration,
}

/// One failed attempt: why, and on which copy.
struct AttemptFailure {
    failure: Failure,
    error: String,
}

impl OpenAiTextRefiner {
    /// Checks that a copy of the service answers and offers the configured model, then
    /// returns a refiner for it. A service that offers a single model is accepted whatever
    /// its name.
    pub fn connect(cfg: RefinementCfg) -> Result<Self> {
        let base_url = cfg.base_url.trim_end_matches('/').to_owned();
        let address = ServiceAddress::parse(&base_url)
            .map_err(|e| anyhow!("text refinement service address {base_url}: {e}"))?
            .with_copy_offset(TEXT_REFINEMENT_COPY_OFFSET);
        let pool = CopyPool::shared_at(address.clone());
        // The model list needs no lease: llama-server answers it beside a running
        // refinement, so a client starting while every copy refines still connects.
        let copies = pool.snapshot();
        let Some(copy) = copies.iter().find(|c| !c.busy).or_else(|| copies.first()) else {
            bail!("text refinement service is not reachable at {base_url} — {START_HINT}");
        };
        let probe = agent::fresh_connections(STARTUP_CHECK_TIMEOUT);
        let offered = offered_models(
            &probe,
            &base_url,
            &address.with_port(copy.port),
            &cfg.api_key,
        )?;
        if offered.len() > 1 && !offered.contains(&cfg.model) {
            bail!(
                "text refinement service at {base_url} does not offer model '{}' — set \
                 [refinement].model to one of: {}",
                cfg.model,
                offered.join(", ")
            );
        }
        let timeout = Duration::from_secs(cfg.timeout_secs);
        Ok(Self {
            agent: agent::fresh_connections(timeout),
            pool,
            cfg,
            timeout,
            recovery_wait: RECOVERY_WAIT,
        })
    }

    /// The same refiner, waiting `wait` instead of [`RECOVERY_WAIT`] for a copy to come
    /// back.
    pub fn with_recovery_wait(self, wait: Duration) -> Self {
        Self { recovery_wait: wait, ..self }
    }
}

fn offered_models(
    agent: &ureq::Agent,
    base_url: &str,
    copy_url: &str,
    api_key: &str,
) -> Result<Vec<String>> {
    let copy_url = copy_url.trim_end_matches('/');
    let response = with_api_key(agent.get(&format!("{copy_url}/{MODELS_PATH}")), api_key)
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
        let json = self.post_with_retry(&body).map_err(|e| {
            anyhow!(
                "text refinement request to {} failed: {e}",
                self.cfg.base_url
            )
        })?;
        let text = cleaned_answer(&json, raw.as_str())?;
        CleanedText::new(text).map_err(|e| anyhow!(e))
    }
}

impl OpenAiTextRefiner {
    /// Posts `body`, once more after a refused or dropped connection, and after a timeout
    /// when another copy may answer, each time to a copy not tried yet when one is free.
    /// When no copy accepts connections, waits once for one to come back first.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    fn post_with_retry(&self, body: &str) -> Result<String, String> {
        let (mut retried, mut waited) = (false, false);
        let mut tried: Vec<u16> = Vec::new();
        loop {
            let failed = match self.attempt(body, &mut tried) {
                Ok(json) => return Ok(json),
                Err(failed) => failed,
            };
            let attempt = tried.len();
            if !waited && !self.pool.has_copies() {
                waited = true;
                if self.wait_for_copy() {
                    tried.clear();
                    continue;
                }
                return Err(failed.error);
            }
            let pause = match failed.failure {
                _ if retried => None,
                Failure::Dropped => Some(DROPPED_RETRY_PAUSE),
                Failure::TimedOut if self.has_untried_copy(&tried) => Some(Duration::ZERO),
                _ => None,
            };
            let Some(pause) = pause else {
                return Err(failed.error);
            };
            retried = true;
            log::warn!(
                "text refinement service {} (refine attempt {attempt}): {} — retrying in {}ms",
                failed.failure.describe(),
                failed.error,
                pause.as_millis()
            );
            thread::sleep(pause);
        }
    }

    /// One attempt on a free copy, preferring one not in `tried`.
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn attempt(&self, body: &str, tried: &mut Vec<u16>) -> Result<String, AttemptFailure> {
        let lease = self
            .pool
            .lease_avoiding(self.timeout, tried)
            .map_err(|e| AttemptFailure {
                failure: match e {
                    PoolError::NoCopies { .. } => Failure::Dropped,
                    PoolError::NoFreeCopy { .. } => Failure::TimedOut,
                },
                error: e.to_string(),
            })?;
        let port = lease.port();
        tried.push(port);
        let endpoint = format!(
            "{}/{CHAT_COMPLETIONS_PATH}",
            lease.url().trim_end_matches('/')
        );
        let sent = with_api_key(self.agent.post(&endpoint), &self.cfg.api_key)
            .set("Content-Type", "application/json")
            .send_string(body);
        match sent {
            Ok(response) => {
                lease.finished();
                response.into_string().map_err(|e| AttemptFailure {
                    failure: Failure::Other,
                    error: format!("read the answer of copy {port}: {e}"),
                })
            }
            Err(error) => {
                let refused = matches!(
                    &error,
                    ureq::Error::Transport(t) if t.kind() == ureq::ErrorKind::ConnectionFailed
                );
                if refused {
                    lease.refused();
                } else {
                    lease.finished();
                }
                Err(AttemptFailure {
                    failure: Failure::of(&error),
                    error: format!("copy {port}: {error}"),
                })
            }
        }
    }

    fn has_untried_copy(&self, tried: &[u16]) -> bool {
        self.pool
            .snapshot()
            .iter()
            .any(|copy| !tried.contains(&copy.port))
    }

    /// Waits for a copy to accept connections again; logs the wait and how it ended.
    fn wait_for_copy(&self) -> bool {
        log::warn!(
            "text refinement: no copy accepts connections — waiting up to {} s for one to come back",
            self.recovery_wait.as_secs_f32()
        );
        let started = Instant::now();
        let back = self.pool.await_copy(self.recovery_wait);
        if back {
            log::info!(
                "text refinement: a copy is back after {}ms",
                started.elapsed().as_millis()
            );
        }
        back
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    /// Answers every request with `status` and `body`; returns the service address.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
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
        let refiner = OpenAiTextRefiner::connect(cfg(&url, "local"));
        // Then
        assert!(refiner.is_ok());
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
