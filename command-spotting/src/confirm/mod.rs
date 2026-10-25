//! Confirms short utterances with the speech recognition service: what was said, in full,
//! decides whether the utterance is a command phrase.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use voicepuncher_copy_pool::{CopyPool, Finish, Ran};

mod pace;
mod recent_audio;
mod request;

use pace::Pace;
pub use recent_audio::RecentAudio;
use request::{multipart_body, wav, BOUNDARY};

/// How long a copy cut short may take to answer its health check before it counts as stalled.
const HEALTH_WAIT: Duration = Duration::from_millis(300);

/// A word the speech recognition service heard, with its times in the audio sent.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TimedWord {
    #[serde(rename = "word")]
    pub text: String,
    pub start: f32,
    pub end: f32,
}

/// What the speech recognition service heard: the text, and each word with its times.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Heard {
    pub text: String,
    pub words: Vec<TimedWord>,
}

impl Heard {
    pub fn word_texts(&self) -> Vec<&str> {
        self.words.iter().map(|w| w.text.as_str()).collect()
    }
}

#[derive(Debug, Deserialize)]
struct Transcription {
    text: String,
    #[serde(default)]
    segments: Vec<TranscribedSegment>,
}

#[derive(Debug, Deserialize)]
struct TranscribedSegment {
    #[serde(default)]
    words: Vec<TimedWord>,
}

fn heard_from(answer: &str) -> Result<Heard> {
    let transcription: Transcription =
        serde_json::from_str(answer).context("speech recognition service answer")?;
    Ok(Heard {
        text: transcription.text,
        words: whole_words(transcription.segments.into_iter().flat_map(|s| s.words)),
    })
}

/// The service reports pieces of words: a piece that does not start with a space
/// continues the word before it ("end" + "er", "today" + "'s").
fn whole_words(pieces: impl Iterator<Item = TimedWord>) -> Vec<TimedWord> {
    let mut words: Vec<TimedWord> = Vec::new();
    for piece in pieces {
        match words.last_mut() {
            Some(word) if !piece.text.starts_with(' ') => {
                word.text.push_str(&piece.text);
                word.end = piece.end;
            }
            _ => words.push(piece),
        }
    }
    words
}

/// What was said in a clip of 16 kHz mono audio, given up after `deadline`.
pub trait Hear: Send + Sync {
    fn hear(&self, samples: &[f32], deadline: Duration) -> Result<Heard>;
}

/// A client of the speech recognition service (`OpenAI` Audio Transcriptions API), one
/// confirmation per copy of the service.
pub struct UtteranceTranscriber {
    pool: CopyPool,
    pace: Pace,
}

/// One confirmation sent to a copy: what it was given and how long it took.
struct Sent {
    port: u16,
    /// How long the copy could decode before it was cut short.
    budget: Duration,
    /// How much of the deadline was left when it was sent.
    left: Duration,
    took: Duration,
}

impl UtteranceTranscriber {
    /// `base_url` like `http://127.0.0.1:8085/v1`: the copies are found from it.
    pub fn new(base_url: &str) -> Result<Self> {
        let pool = CopyPool::shared(base_url)
            .map_err(|e| anyhow!("speech recognition service address {base_url}: {e}"))?;
        Ok(Self { pool, pace: Pace::default() })
    }

    /// As `new`, with a decoding budget never below `floor`: for a caller whose result must
    /// not depend on how loaded the service is, such as tests sharing it with other runs.
    pub fn with_decoding_floor(base_url: &str, floor: Duration) -> Result<Self> {
        Ok(Self { pace: Pace::with_floor(floor), ..Self::new(base_url)? })
    }

    /// What was said in `samples` (16 kHz mono), with each word's times; the wait for a
    /// free copy and for its answer together never pass `deadline`. A copy still decoding
    /// after the budget the pace of the recent confirmations gives is cut short: when it
    /// answers its health check the speech decodes long and nothing was heard, else it
    /// stalled and, as a copy that refuses the connection, hands the confirmation to
    /// another copy while the deadline lasts.
    pub fn transcribe(
        &self,
        samples: &[f32],
        sample_rate: u32,
        deadline: Duration,
    ) -> Result<Heard> {
        let started = Instant::now();
        let body = Arc::new(multipart_body(&wav(samples, sample_rate)?));
        let mut tried: Vec<u16> = Vec::new();
        loop {
            let left = deadline.saturating_sub(started.elapsed());
            let lease = self.pool.lease_avoiding(left, &tried).map_err(|e| {
                log::warn!("confirmation given up unsent: {e}");
                anyhow!("speech recognition service: {e}")
            })?;
            let port = lease.port();
            tried.push(port);
            let left = deadline.saturating_sub(started.elapsed());
            let budget = self.pace.budget(deadline).min(left);
            let body = Arc::clone(&body);
            let asked = Instant::now();
            let ran = lease.run(left, move |url| post(url, &body, budget));
            let sent = Sent { port, budget, left, took: asked.elapsed() };
            if let Some(outcome) =
                self.settle(ran, &sent, deadline.saturating_sub(started.elapsed()))
            {
                return outcome;
            }
        }
    }

    /// What the confirmation `sent` came to, or none when it goes to another copy; `left`
    /// is what remains of the deadline.
    fn settle(
        &self,
        ran: Ran<Result<Posted>>,
        sent: &Sent,
        left: Duration,
    ) -> Option<Result<Heard>> {
        match ran {
            Ran::Answered(Ok(Posted::Answer(answer))) => Some(
                heard_from(&answer).inspect(|heard| self.pace.note_answer(sent.took, &heard.text)),
            ),
            Ran::Answered(Ok(Posted::Refused(e))) => {
                log::warn!(
                    "confirmation refused by copy {}: sending it to another copy ({e:#})",
                    sent.port
                );
                None
            }
            Ran::Answered(Ok(Posted::TimedOut(url))) if sent.budget < sent.left => {
                let nothing = cut_short(&url, sent.budget, left);
                if nothing.is_some() {
                    self.pace.note_cut_short();
                }
                nothing.map(Ok)
            }
            Ran::Answered(Ok(Posted::TimedOut(url))) => Some(Err(anyhow!(
                "speech recognition copy {url} sent no answer in {} ms",
                sent.left.as_millis()
            ))),
            Ran::Answered(Err(e)) => Some(Err(e)),
            Ran::GaveUp => Some(Err(anyhow!(
                "speech recognition copy {} sent no answer in {} ms",
                sent.port,
                sent.left.as_millis()
            ))),
        }
    }
}

/// What a copy sent back for a request.
enum Posted {
    Answer(String),
    /// No answer within the request's time; the connection is closed, which stops the copy.
    TimedOut(String),
    /// The copy refused the connection: another copy may take the request.
    Refused(anyhow::Error),
}

/// Posts `body` to the copy at `url` and reads the whole answer, within `timeout`.
fn post(url: &str, body: &[u8], timeout: Duration) -> (Result<Posted>, Finish) {
    let endpoint = format!("{}/audio/transcriptions", url.trim_end_matches('/'));
    let sent = ureq::post(&endpoint)
        .timeout(timeout)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .send_bytes(body);
    match sent {
        Ok(response) => match response.into_string() {
            Ok(answer) => (Ok(Posted::Answer(answer)), Finish::Answered),
            Err(e) if is_timeout(&e) => (Ok(Posted::TimedOut(url.to_owned())), Finish::Answered),
            Err(e) => (Err(e.into()), Finish::Answered),
        },
        Err(error) if is_io_timeout(&error) => {
            (Ok(Posted::TimedOut(url.to_owned())), Finish::Answered)
        }
        Err(error) => failed(error, &endpoint),
    }
}

/// Whether `error` is a transport timeout: the copy took longer than the request's time.
fn is_io_timeout(error: &ureq::Error) -> bool {
    let ureq::Error::Transport(t) = error else {
        return false;
    };
    t.kind() == ureq::ErrorKind::Io
        && std::error::Error::source(t)
            .and_then(|e| e.downcast_ref::<std::io::Error>())
            .is_some_and(is_timeout)
}

/// A request that failed at `endpoint`: refused when the copy refused the connection.
fn failed(error: ureq::Error, endpoint: &str) -> (Result<Posted>, Finish) {
    let refused = matches!(
        &error,
        ureq::Error::Transport(t) if t.kind() == ureq::ErrorKind::ConnectionFailed
    );
    let error =
        anyhow::Error::from(error).context(format!("speech recognition service at {endpoint}"));
    if refused {
        (Ok(Posted::Refused(error)), Finish::Refused)
    } else {
        (Err(error), Finish::Answered)
    }
}

/// A confirmation cut short after `budget` on the copy at `url`: nothing was heard when the
/// copy is live, none when it stalled and the confirmation goes to another copy.
fn cut_short(url: &str, budget: Duration, left: Duration) -> Option<Heard> {
    let live = answers_health(url, HEALTH_WAIT.min(left));
    let next = if live {
        "still decoding, nothing heard"
    } else {
        "the copy does not answer, sending it to another copy"
    };
    log::warn!(
        "confirmation cut short after {}ms on copy {url}: {next}",
        budget.as_millis()
    );
    live.then(Heard::default)
}

/// Whether the copy at `url` (`http://host:port/v1`) answers its health check within `wait`:
/// a live copy stops decoding once its request is closed and answers at once, a stalled one
/// answers nothing.
fn answers_health(url: &str, wait: Duration) -> bool {
    let authority_end = url
        .find("://")
        .and_then(|scheme| url[scheme + 3..].find('/').map(|end| scheme + 3 + end))
        .unwrap_or(url.len());
    ureq::get(&format!("{}/health", &url[..authority_end]))
        .timeout(wait)
        .call()
        .is_ok()
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

impl Hear for UtteranceTranscriber {
    fn hear(&self, samples: &[f32], deadline: Duration) -> Result<Heard> {
        self.transcribe(samples, crate::audio::SAMPLE_RATE, deadline)
    }
}

#[cfg(test)]
mod tests;
