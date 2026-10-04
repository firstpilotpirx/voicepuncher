use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use url::Url;

use crate::config::TranscriptionCfg;
use crate::core::application::transcriber::Transcriber;
use crate::core::application::transcript_filter::is_whisper_hallucination;
use crate::core::domain::{AudioBuffer16k, RawTranscript};

mod request;

use request::{encode_wav, multipart_body, normalize_transcript};

const TRANSCRIPTIONS_PATH: &str = "audio/transcriptions";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const SLOW_REQUEST_LOG_MS: u128 = 600;
const START_HINT: &str = "start it with 'just start-model-servers'";

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Transcribes audio through a local speech recognition service that speaks the
/// OpenAI Audio Transcriptions API (whisper.cpp `whisper-server`, Speaches, LocalAI).
pub struct OpenAiTranscriber {
    agent: ureq::Agent,
    endpoint: String,
    cfg: TranscriptionCfg,
}

impl OpenAiTranscriber {
    /// Checks that the service accepts connections, then returns a transcriber for it.
    pub fn connect(cfg: TranscriptionCfg) -> Result<Self> {
        let addrs = socket_addrs(&cfg.base_url)?;
        let reachable = addrs
            .iter()
            .any(|addr| TcpStream::connect_timeout(addr, CONNECT_TIMEOUT).is_ok());
        if !reachable {
            bail!(
                "speech recognition service is not reachable at {} — {START_HINT}",
                cfg.base_url
            );
        }
        let endpoint = format!(
            "{}/{TRANSCRIPTIONS_PATH}",
            cfg.base_url.trim_end_matches('/')
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build();
        Ok(Self {
            agent,
            endpoint,
            cfg,
        })
    }

    fn request_text(&self, audio: &AudioBuffer16k) -> Result<String> {
        let wav = encode_wav(audio)?;
        let body = multipart_body(&self.cfg, &wav);
        let response = self
            .agent
            .post(&self.endpoint)
            .set("Content-Type", &request::content_type())
            .send_bytes(&body)
            .map_err(|e| {
                anyhow!(
                    "speech recognition request to {} failed: {e}",
                    self.endpoint
                )
            })?;
        let json = response
            .into_string()
            .context("read speech recognition response")?;
        let parsed: TranscriptionResponse = serde_json::from_str(&json)
            .with_context(|| format!("unexpected speech recognition response: {json}"))?;
        Ok(normalize_transcript(&parsed.text))
    }
}

fn socket_addrs(base_url: &str) -> Result<Vec<SocketAddr>> {
    let url = Url::parse(base_url)
        .with_context(|| format!("invalid speech recognition service address: {base_url}"))?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("speech recognition service address has no host: {base_url}"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| anyhow!("speech recognition service address has no port: {base_url}"))?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    Ok((host, port)
        .to_socket_addrs()
        .with_context(|| format!("resolve speech recognition service address {base_url}"))?
        .collect())
}

impl Transcriber for OpenAiTranscriber {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let started = Instant::now();
        let text = self.request_text(audio)?;
        let elapsed_ms = started.elapsed().as_millis();
        if elapsed_ms > SLOW_REQUEST_LOG_MS {
            log::info!(
                "transcription: audio={}ms request={elapsed_ms}ms",
                audio.duration_ms()
            );
        }
        if is_whisper_hallucination(&text) {
            log::info!("dropping hallucinated transcript: {text:?}");
            return Ok(None);
        }
        RawTranscript::new(text).map(Some).map_err(|e| anyhow!(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn cfg_for_port(port: u16) -> TranscriptionCfg {
        TranscriptionCfg {
            base_url: format!("http://127.0.0.1:{port}/v1"),
            ..TranscriptionCfg::default()
        }
    }

    #[test]
    fn should_connect_when_service_port_is_open() {
        // Given
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // When
        let transcriber = OpenAiTranscriber::connect(cfg_for_port(port)).unwrap();
        // Then
        assert_eq!(
            transcriber.endpoint,
            format!("http://127.0.0.1:{port}/v1/audio/transcriptions")
        );
    }

    #[test]
    fn should_report_address_and_start_command_when_port_is_closed() {
        // Given
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        // When
        let message = OpenAiTranscriber::connect(cfg_for_port(port))
            .err()
            .unwrap()
            .to_string();
        // Then
        assert!(
            message.contains(&format!("127.0.0.1:{port}/v1")),
            "{message}"
        );
        assert!(message.contains("just start-model-servers"), "{message}");
    }

    #[test]
    fn should_resolve_ipv6_loopback_address() {
        let addrs = socket_addrs("http://[::1]:8081/v1").unwrap();
        assert!(addrs
            .iter()
            .all(|a| a.ip().is_loopback() && a.port() == 8081));
    }
}
