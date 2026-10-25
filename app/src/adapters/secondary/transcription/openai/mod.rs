use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use voicepuncher_copy_pool::{CopyPool, Finish, PoolError, Ran};

use crate::adapters::secondary::http::agent;
use crate::adapters::secondary::http::failure::Failure;
use crate::config::TranscriptionCfg;
use crate::core::application::transcriber::Transcriber;
use crate::core::application::transcript_filter::strip_whisper_hallucinations;
use crate::core::domain::{AudioBuffer16k, RawTranscript};

mod deadline;
mod language;
mod request;
mod retries;

pub use deadline::longest_wait;
use deadline::request_deadline;
use language::{retry_language, TranscriptionResponse};
use request::{encode_wav, multipart_body, normalize_transcript};
use retries::{Retries, Tries};

const TRANSCRIPTIONS_PATH: &str = "audio/transcriptions";
const SLOW_REQUEST_LOG_MS: u128 = 600;
const START_HINT: &str = "start it with 'just start-model-servers'";
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
    /// A deadline every request takes instead of the one its audio length gives; test clients.
    request_deadline: Option<Duration>,
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
        Ok(Self {
            agent,
            pool,
            cfg,
            requests: 0,
            recovery_wait: RECOVERY_WAIT,
            request_deadline: None,
        })
    }

    /// The same transcriber, waiting `wait` instead of [`RECOVERY_WAIT`] for a copy to come
    /// back.
    pub fn with_recovery_wait(self, wait: Duration) -> Self {
        Self { recovery_wait: wait, ..self }
    }

    /// The same transcriber, giving every request `deadline` whatever its audio length: for
    /// a test whose copies are busy with other work.
    pub fn with_request_deadline(self, deadline: Duration) -> Self {
        Self { request_deadline: Some(deadline), ..self }
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
        let deadline = request_deadline(
            audio,
            Duration::from_secs(self.cfg.timeout_secs),
            self.request_deadline,
        );
        log::debug!(
            "speech recognition: request={} audio={}ms deadline={}ms",
            self.requests,
            audio.duration_ms(),
            deadline.as_millis()
        );
        let json = self.post_with_retries(&Arc::new(body), deadline)?;
        serde_json::from_str(&json)
            .with_context(|| format!("unexpected speech recognition response: {json}"))
    }

    /// Posts `body`, again after a refused or dropped connection, a timeout or an error
    /// answer, as often as each allows, each time to a copy not tried yet when one is
    /// free; every failed attempt is logged with how long it took. A request whose wait
    /// for a free copy ran out asks again at once and keeps its place in the queue. Once
    /// the dropped connections are used up and no copy accepts connections, the request
    /// waits once for a copy to come back and starts its dropped connections over.
    fn post_with_retries(&self, body: &Arc<Vec<u8>>, deadline: Duration) -> Result<String> {
        let mut tries = Tries::new(self.pool.place());
        let mut retries = Retries::default();
        loop {
            tries.attempt += 1;
            let started = Instant::now();
            let failed = match self.attempt(body, deadline, &mut tries) {
                Ok(text) => return Ok(text),
                Err(failed) => failed,
            };
            let pause = retries.pause(&failed, self);
            let failure = format!(
                "speech recognition service {} (request={} attempt {}, after {}ms, deadline {}ms): {}",
                failed.what,
                self.requests,
                tries.attempt,
                started.elapsed().as_millis(),
                deadline.as_millis(),
                failed.error
            );
            let Some(pause) = pause else {
                log::warn!("{failure} — giving up");
                bail!(
                    "speech recognition request to {} failed on copies {}: {}",
                    self.cfg.base_url,
                    ports_list(&tries.tried),
                    failed.error
                )
            };
            log::warn!("{failure} — retrying in {}ms", pause.as_millis());
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
    fn attempt(
        &self,
        body: &Arc<Vec<u8>>,
        deadline: Duration,
        tries: &mut Tries,
    ) -> Result<String, AttemptFailure> {
        let lease = tries
            .place
            .lease_avoiding(deadline, &tries.tried)
            .map_err(|e| AttemptFailure::of_pool(&e))?;
        let port = lease.port();
        if !tries.tried.contains(&port) {
            tries.tried.push(port);
        }
        log::debug!(
            "speech recognition: request={} attempt {} on copy {port}",
            self.requests,
            tries.attempt
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
mod tests;
