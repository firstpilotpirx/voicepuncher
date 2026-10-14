use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use url::Url;

use crate::adapters::secondary::http::failure::Failure;
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
/// Pauses before each new attempt when the service drops the connection before it
/// answers (a busy service resets connections it has no room for).
const RETRY_PAUSES: [Duration; 3] = [
    Duration::from_millis(200),
    Duration::from_millis(400),
    Duration::from_millis(800),
];
/// New attempts after a request the service did not answer in time or answered with an
/// error: the service aborts the request the app gave up on, and the next one goes to a
/// free copy.
const SLOW_RETRIES: u32 = 2;
const SLOW_RETRY_PAUSE: Duration = Duration::from_millis(200);
/// The wait for one request: this, plus half the length of its audio.
const DEADLINE_BASE: Duration = Duration::from_secs(5);
const DEADLINE_PER_AUDIO_SECOND: f32 = 0.5;

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
    /// Numbers the requests, for the log: `request=<n>`.
    requests: u64,
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
        let agent = request_agent(Duration::from_secs(cfg.timeout_secs));
        Ok(Self {
            agent,
            endpoint,
            cfg,
            requests: 0,
        })
    }

    fn request_text(&mut self, audio: &AudioBuffer16k) -> Result<String> {
        let wav = encode_wav(audio)?;
        let body = multipart_body(&self.cfg, &wav);
        self.requests += 1;
        let deadline = request_deadline(audio, Duration::from_secs(self.cfg.timeout_secs));
        log::debug!(
            "speech recognition: request={} audio={}ms deadline={}ms",
            self.requests,
            audio.duration_ms(),
            deadline.as_millis()
        );
        let response = self.post_with_retries(&body, deadline)?;
        let json = response
            .into_string()
            .context("read speech recognition response")?;
        let parsed: TranscriptionResponse = serde_json::from_str(&json)
            .with_context(|| format!("unexpected speech recognition response: {json}"))?;
        Ok(normalize_transcript(&parsed.text))
    }

    /// Posts `body`, again after a dropped connection, a timeout or an error answer, as
    /// often as each allows; every failed attempt is logged with how long it took.
    fn post_with_retries(&self, body: &[u8], deadline: Duration) -> Result<ureq::Response> {
        let (mut dropped, mut slow, mut attempt) = (0usize, 0u32, 0u32);
        loop {
            attempt += 1;
            let started = Instant::now();
            let result = self
                .agent
                .post(&self.endpoint)
                .timeout(deadline)
                .set("Content-Type", &request::content_type())
                .send_bytes(body);
            let error = match result {
                Ok(response) => return Ok(response),
                Err(e) => e,
            };
            let failure = Failure::of(&error);
            let pause = match failure {
                Failure::Dropped if dropped < RETRY_PAUSES.len() => {
                    dropped += 1;
                    RETRY_PAUSES[dropped - 1]
                }
                Failure::TimedOut | Failure::Busy(_) if slow < SLOW_RETRIES => {
                    slow += 1;
                    SLOW_RETRY_PAUSE
                }
                _ => {
                    log::warn!(
                        "speech recognition service {} (request={} attempt {attempt}, after {}ms, deadline {}ms): {error} — giving up",
                        failure.describe(),
                        self.requests,
                        started.elapsed().as_millis(),
                        deadline.as_millis()
                    );
                    bail!(
                        "speech recognition request to {} failed: {error}",
                        self.endpoint
                    )
                }
            };
            log::warn!(
                "speech recognition service {} (request={} attempt {attempt}, after {}ms, deadline {}ms): {error} — retrying in {}ms",
                failure.describe(),
                self.requests,
                started.elapsed().as_millis(),
                deadline.as_millis(),
                pause.as_millis()
            );
            thread::sleep(pause);
        }
    }
}

/// How long one request may take: a few seconds for a short recording, more for long
/// audio, never more than `timeout`.
fn request_deadline(audio: &AudioBuffer16k, timeout: Duration) -> Duration {
    let audio_secs = audio.duration_ms() as f32 / 1_000.0;
    (DEADLINE_BASE + Duration::from_secs_f32(audio_secs * DEADLINE_PER_AUDIO_SECOND)).min(timeout)
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
        if text.trim().is_empty() {
            log::info!("speech recognition service heard no speech");
            return Ok(None);
        }
        if is_whisper_hallucination(&text) {
            log::info!("dropping hallucinated transcript: {text:?}");
            return Ok(None);
        }
        RawTranscript::new(text).map(Some).map_err(|e| anyhow!(e))
    }
}

/// Opens a new connection for every request: the service closes an idle connection
/// after a few seconds, and after a pause of minutes the HTTP client fails a request
/// on such a connection instead of reconnecting. A new loopback connection costs
/// about 0.1 ms.
fn request_agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(timeout)
        .max_idle_connections(0)
        .max_idle_connections_per_host(0)
        .build()
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
    fn should_wait_a_few_seconds_for_a_short_recording() {
        let five_seconds = AudioBuffer16k::new(vec![0; 5 * 16_000]);
        assert_eq!(
            request_deadline(&five_seconds, Duration::from_secs(60)),
            Duration::from_millis(7_500)
        );
    }

    #[test]
    fn should_wait_longer_for_a_long_chunk() {
        let chunk = AudioBuffer16k::new(vec![0; 32 * 16_000]);
        assert_eq!(
            request_deadline(&chunk, Duration::from_secs(60)),
            Duration::from_secs(21)
        );
    }

    #[test]
    fn should_never_wait_longer_than_the_configured_timeout() {
        let chunk = AudioBuffer16k::new(vec![0; 32 * 16_000]);
        assert_eq!(
            request_deadline(&chunk, Duration::from_secs(10)),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn should_resolve_ipv6_loopback_address() {
        let addrs = socket_addrs("http://[::1]:8081/v1").unwrap();
        assert!(addrs
            .iter()
            .all(|a| a.ip().is_loopback() && a.port() == 8081));
    }
}
