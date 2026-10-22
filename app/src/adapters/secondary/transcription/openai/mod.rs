use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use voicepuncher_copy_pool::{CopyPool, Finish, Place, PoolError, Ran};

use crate::adapters::secondary::http::agent;
use crate::adapters::secondary::http::failure::Failure;
use crate::config::TranscriptionCfg;
use crate::core::application::transcriber::Transcriber;
use crate::core::application::transcript_filter::strip_whisper_hallucinations;
use crate::core::domain::{AudioBuffer16k, RawTranscript};

mod language;
mod request;

use language::{retry_language, TranscriptionResponse};
use request::{encode_wav, multipart_body, normalize_transcript};

const TRANSCRIPTIONS_PATH: &str = "audio/transcriptions";
const SLOW_REQUEST_LOG_MS: u128 = 600;
const START_HINT: &str = "start it with 'just start-model-servers'";
/// Pauses before each new attempt when the service refuses or drops the connection
/// before it answers (a busy service resets connections it has no room for).
const RETRY_PAUSES: [Duration; 3] = [
    Duration::from_millis(200),
    Duration::from_millis(400),
    Duration::from_millis(800),
];
/// New attempts after a request the service did not answer in time or answered with an
/// error; each goes to another copy than the one still working on the abandoned one.
const SLOW_RETRIES: u32 = 2;
const SLOW_RETRY_PAUSE: Duration = Duration::from_millis(200);
/// The wait for one request: this, plus half the length of its audio.
const DEADLINE_BASE: Duration = Duration::from_secs(5);
const DEADLINE_PER_AUDIO_SECOND: f32 = 0.5;
/// How long a request waits for a copy to come back once no copy accepts connections: the
/// app restarts a copy that died after 1 s, and loading its model takes a few seconds more.
pub const RECOVERY_WAIT: Duration = Duration::from_secs(30);

/// Transcribes audio through a local speech recognition service that speaks the
/// `OpenAI` Audio Transcriptions API (whisper.cpp `whisper-server`, Speaches, `LocalAI`),
/// one request per copy of the service.
pub struct OpenAiTranscriber {
    agent: ureq::Agent,
    pool: CopyPool,
    cfg: TranscriptionCfg,
    /// Numbers the requests, for the log: `request=<n>`.
    requests: u64,
    recovery_wait: Duration,
}

/// Why one attempt brought no answer.
struct AttemptFailure {
    failure: Failure,
    /// What the service did, for the log.
    what: String,
    error: String,
    /// Only the wait for a free copy ran out: the request still holds its place in the queue.
    waiting: bool,
}

impl AttemptFailure {
    fn of(error: &ureq::Error) -> Self {
        let failure = Failure::of(error);
        Self { failure, what: failure.describe(), error: error.to_string(), waiting: false }
    }

    fn of_pool(error: &PoolError) -> Self {
        let (failure, what) = match error {
            PoolError::NoFreeCopy { .. } => (Failure::TimedOut, "had no free copy in time"),
            PoolError::NoCopies { .. } => (Failure::Dropped, "accepts no connections"),
        };
        Self {
            failure,
            what: what.to_owned(),
            error: error.to_string(),
            waiting: matches!(error, PoolError::NoFreeCopy { .. }),
        }
    }
}

impl OpenAiTranscriber {
    /// Finds the copies of the service, then returns a transcriber for them.
    pub fn connect(cfg: TranscriptionCfg) -> Result<Self> {
        let pool = CopyPool::shared(&cfg.base_url).map_err(|e| {
            anyhow!(
                "invalid speech recognition service address {}: {e}",
                cfg.base_url
            )
        })?;
        if !pool.has_copies() {
            bail!(
                "speech recognition service is not reachable at {} — {START_HINT}",
                cfg.base_url
            );
        }
        let agent = agent::fresh_connections(Duration::from_secs(cfg.timeout_secs));
        Ok(Self { agent, pool, cfg, requests: 0, recovery_wait: RECOVERY_WAIT })
    }

    /// The same transcriber, waiting `wait` instead of [`RECOVERY_WAIT`] for a copy to come
    /// back.
    pub fn with_recovery_wait(self, wait: Duration) -> Self {
        Self { recovery_wait: wait, ..self }
    }

    /// Transcribes `audio`, again in the user's first language when the service heard a
    /// language the user does not speak.
    fn request_text(&mut self, audio: &AudioBuffer16k) -> Result<String> {
        let wav = encode_wav(audio)?;
        let mut response = self.request_transcript(multipart_body(&self.cfg, &wav), audio)?;
        if let Some(language) = retry_language(&response, &self.cfg).map(str::to_owned) {
            log::info!(
                "speech recognition: transcript detected as {}, not one of {} — requesting again in {language}",
                response.language.as_deref().unwrap_or_default(),
                self.cfg.languages.join(", ")
            );
            let fixed = TranscriptionCfg { language, ..self.cfg.clone() };
            response = self.request_transcript(multipart_body(&fixed, &wav), audio)?;
        }
        Ok(normalize_transcript(&response.text))
    }

    fn request_transcript(
        &mut self,
        body: Vec<u8>,
        audio: &AudioBuffer16k,
    ) -> Result<TranscriptionResponse> {
        self.requests += 1;
        let deadline = request_deadline(audio, Duration::from_secs(self.cfg.timeout_secs));
        log::debug!(
            "speech recognition: request={} audio={}ms deadline={}ms",
            self.requests,
            audio.duration_ms(),
            deadline.as_millis()
        );
        let json = self.post_with_retries(Arc::new(body), deadline)?;
        serde_json::from_str(&json)
            .with_context(|| format!("unexpected speech recognition response: {json}"))
    }

    /// Posts `body`, again after a refused or dropped connection, a timeout or an error
    /// answer, as often as each allows, each time to a copy not tried yet when one is
    /// free; every failed attempt is logged with how long it took. A request whose wait
    /// for a free copy ran out asks again at once and keeps its place in the queue. Once
    /// the dropped connections are used up and no copy accepts connections, the request
    /// waits once for a copy to come back and starts its dropped connections over.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn post_with_retries(&self, body: Arc<Vec<u8>>, deadline: Duration) -> Result<String> {
        let (mut dropped, mut slow, mut attempt) = (0usize, 0u32, 0u32);
        let mut waited_for_copy = false;
        let mut tried: Vec<u16> = Vec::new();
        let mut place = self.pool.place();
        loop {
            attempt += 1;
            let started = Instant::now();
            let failed = match self.attempt(&body, deadline, attempt, &mut tried, &mut place) {
                Ok(text) => return Ok(text),
                Err(failed) => failed,
            };
            let pause = match failed.failure {
                Failure::Dropped if dropped < RETRY_PAUSES.len() => {
                    dropped += 1;
                    Some(RETRY_PAUSES[dropped - 1])
                }
                Failure::Dropped if !waited_for_copy && !self.pool.has_copies() => {
                    waited_for_copy = true;
                    self.wait_for_copy().then(|| {
                        dropped = 0;
                        Duration::ZERO
                    })
                }
                Failure::TimedOut | Failure::Busy(_) if slow < SLOW_RETRIES => {
                    slow += 1;
                    Some(if failed.waiting {
                        Duration::ZERO
                    } else {
                        SLOW_RETRY_PAUSE
                    })
                }
                _ => None,
            };
            let Some(pause) = pause else {
                log::warn!(
                    "speech recognition service {} (request={} attempt {attempt}, after {}ms, deadline {}ms): {} — giving up",
                    failed.what,
                    self.requests,
                    started.elapsed().as_millis(),
                    deadline.as_millis(),
                    failed.error
                );
                bail!(
                    "speech recognition request to {} failed on copies {}: {}",
                    self.cfg.base_url,
                    ports_list(&tried),
                    failed.error
                )
            };
            log::warn!(
                "speech recognition service {} (request={} attempt {attempt}, after {}ms, deadline {}ms): {} — retrying in {}ms",
                failed.what,
                self.requests,
                started.elapsed().as_millis(),
                deadline.as_millis(),
                failed.error,
                pause.as_millis()
            );
            thread::sleep(pause);
        }
    }

    /// Waits for a copy to accept connections again; logs the wait and how it ended.
    fn wait_for_copy(&self) -> bool {
        log::warn!(
            "speech recognition: no copy accepts connections — waiting up to {} s for one to come back (request={})",
            self.recovery_wait.as_secs_f32(),
            self.requests
        );
        let started = Instant::now();
        let back = self.pool.await_copy(self.recovery_wait);
        if back {
            log::info!(
                "speech recognition: a copy is back after {}ms (request={})",
                started.elapsed().as_millis(),
                self.requests
            );
        }
        back
    }

    /// One attempt on a free copy, waiting at most `deadline` in `place` for the copy and
    /// again for its answer; a copy given up on stays held until it answered.
    #[expect(clippy::too_many_arguments, reason = "size baseline")]
    fn attempt(
        &self,
        body: &Arc<Vec<u8>>,
        deadline: Duration,
        attempt: u32,
        tried: &mut Vec<u16>,
        place: &mut Place,
    ) -> Result<String, AttemptFailure> {
        let lease = place
            .lease_avoiding(deadline, tried)
            .map_err(|e| AttemptFailure::of_pool(&e))?;
        let port = lease.port();
        if !tried.contains(&port) {
            tried.push(port);
        }
        log::debug!(
            "speech recognition: request={} attempt {attempt} on copy {port}",
            self.requests
        );
        let (agent, body) = (self.agent.clone(), Arc::clone(body));
        let ran = lease.run(deadline, move |url| post(&agent, url, &body));
        match ran {
            Ran::Answered(answer) => answer,
            Ran::GaveUp => Err(AttemptFailure {
                failure: Failure::TimedOut,
                what: Failure::TimedOut.describe(),
                error: format!("copy {port} sent no answer in {}ms", deadline.as_millis()),
                waiting: false,
            }),
        }
    }
}

/// Posts `body` to the copy at `url` and reads the whole answer.
fn post(agent: &ureq::Agent, url: &str, body: &[u8]) -> (Result<String, AttemptFailure>, Finish) {
    let endpoint = format!("{}/{TRANSCRIPTIONS_PATH}", url.trim_end_matches('/'));
    let sent = agent
        .post(&endpoint)
        .set("Content-Type", &request::content_type())
        .send_bytes(body);
    match sent {
        Ok(response) => {
            let read = response.into_string().map_err(|e| AttemptFailure {
                failure: Failure::Other,
                what: "sent an unreadable answer".to_owned(),
                error: e.to_string(),
                waiting: false,
            });
            (read, Finish::Answered)
        }
        Err(error) => {
            let refused = matches!(
                &error,
                ureq::Error::Transport(t) if t.kind() == ureq::ErrorKind::ConnectionFailed
            );
            let finish = if refused {
                Finish::Refused
            } else {
                Finish::Answered
            };
            (Err(AttemptFailure::of(&error)), finish)
        }
    }
}

fn ports_list(ports: &[u16]) -> String {
    let ports: Vec<String> = ports.iter().map(u16::to_string).collect();
    ports.join(", ")
}

/// The longest one recording or chunk may take with every retry, each attempt at its
/// longest deadline `timeout`: what a stopped dictation may wait for its last chunk.
pub fn longest_wait(timeout: Duration) -> Duration {
    timeout * (1 + SLOW_RETRIES) + SLOW_RETRY_PAUSE * SLOW_RETRIES + RETRY_PAUSES.iter().sum()
}

/// How long one request may take: a few seconds for a short recording, more for long
/// audio, never more than `timeout`.
fn request_deadline(audio: &AudioBuffer16k, timeout: Duration) -> Duration {
    let audio_secs = audio.duration_ms() as f32 / 1_000.0;
    (DEADLINE_BASE + Duration::from_secs_f32(audio_secs * DEADLINE_PER_AUDIO_SECOND)).min(timeout)
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
        let Some(speech) = strip_whisper_hallucinations(&text) else {
            log::info!("dropping hallucinated transcript: {text:?}");
            return Ok(None);
        };
        if speech.split_whitespace().count() < text.split_whitespace().count() {
            log::info!("removed hallucination from transcript: {text:?} → {speech:?}");
        }
        RawTranscript::new(speech).map(Some).map_err(|e| anyhow!(e))
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
        let ports: Vec<u16> = transcriber.pool.snapshot().iter().map(|c| c.port).collect();
        assert_eq!(ports, [port]);
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
    fn should_wait_every_attempt_and_pause_at_the_longest() {
        assert_eq!(
            longest_wait(Duration::from_secs(60)),
            Duration::from_millis(181_800)
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
}
