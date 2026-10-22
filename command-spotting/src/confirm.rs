//! Confirms short utterances with the speech recognition service: what was said, in full,
//! decides whether the utterance is a command phrase.

use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use voicepuncher_copy_pool::{CopyPool, Finish, Ran};

const BOUNDARY: &str = "voicepuncher-command-spotting-3f9a";
const LANGUAGE: &str = "en";
// No prompt: with the command phrases as the prompt (whisper-server runs with `-mc 0`, which
// drops a prompt unless the request sets its own `max_context`), Russian speech such as
// "четыре", "пять" or "очисти" was heard as "clear all text." or "press enter.", the user's
// recorded speech gave four wrong commands instead of one, and the slowest confirmations of
// Russian speech took 1.7 s instead of 0.2 s (restore-recognition-prompt, design.md § Results).

/// The temperature step of a retry when a decoding fails (stuck repeating, or too unsure):
/// 0 turns the retries off. The service's default 0.2 retries at 0.2, 0.4, … 1.0, and
/// speech in another language that gets stuck repeating in English used those up for
/// seconds; a retry at 0.55 still took up to 3 s, since it got stuck as well.
const TEMPERATURE_STEP: &str = "0";

/// The longest a copy may decode a confirmation once it was sent: an ordinary one is answered
/// in at most 0.3 s, while speech that gets stuck repeating decodes whisper's whole 219-token
/// limit for 1.7–1.8 s and is never a command. Closing the connection stops the copy's
/// decoding at once (fast-confirmation-of-repeating-russian-speech, design.md).
const DECODING_BUDGET: Duration = Duration::from_secs(1);
/// How long a copy cut short may take to answer its health check before it counts as stalled.
const HEALTH_WAIT: Duration = Duration::from_millis(300);

/// The last seconds of a connection's audio, by absolute sample position.
pub struct RecentAudio {
    samples: VecDeque<f32>,
    first: usize,
    capacity: usize,
}

impl RecentAudio {
    pub fn new(capacity: usize) -> Self {
        Self { samples: VecDeque::with_capacity(capacity), first: 0, capacity }
    }

    pub fn push(&mut self, frame: &[f32]) {
        self.samples.extend(frame.iter().copied());
        let excess = self.samples.len().saturating_sub(self.capacity);
        self.samples.drain(..excess);
        self.first += excess;
    }

    /// The samples from `start` to `end` that are still held.
    pub fn between(&self, start: usize, end: usize) -> Vec<f32> {
        let from = start.saturating_sub(self.first).min(self.samples.len());
        let to = end.saturating_sub(self.first).min(self.samples.len());
        self.samples.range(from..to.max(from)).copied().collect()
    }
}

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
}

impl UtteranceTranscriber {
    /// `base_url` like `http://127.0.0.1:8085/v1`: the copies are found from it.
    pub fn new(base_url: &str) -> Result<Self> {
        let pool = CopyPool::shared(base_url)
            .map_err(|e| anyhow!("speech recognition service address {base_url}: {e}"))?;
        Ok(Self { pool })
    }

    /// What was said in `samples` (16 kHz mono), with each word's times; the wait for a
    /// free copy and for its answer together never pass `deadline`. A copy still decoding
    /// after `DECODING_BUDGET` is cut short: when it answers its health check the speech
    /// decodes long and nothing was heard, else it stalled and, as a copy that refuses the
    /// connection, hands the confirmation to another copy while the deadline lasts.
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
            let budget = DECODING_BUDGET.min(left);
            let sent = Arc::clone(&body);
            let ran = lease.run(left, move |url| post(url, &sent, budget));
            match ran {
                Ran::Answered(Ok(Posted::Answer(answer))) => return heard_from(&answer),
                Ran::Answered(Ok(Posted::Refused(e))) => {
                    log::warn!(
                        "confirmation refused by copy {port}: sending it to another copy ({e:#})"
                    );
                }
                Ran::Answered(Ok(Posted::TimedOut(url))) if budget < left => {
                    match cut_short(&url, budget, deadline.saturating_sub(started.elapsed())) {
                        Some(nothing) => return Ok(nothing),
                        None => continue,
                    }
                }
                Ran::Answered(Ok(Posted::TimedOut(url))) => bail!(
                    "speech recognition copy {url} sent no answer in {} ms",
                    deadline.as_millis()
                ),
                Ran::Answered(Err(e)) => return Err(e),
                Ran::GaveUp => bail!(
                    "speech recognition copy {port} sent no answer in {} ms",
                    deadline.as_millis()
                ),
            }
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
        Err(ureq::Error::Transport(t))
            if t.kind() == ureq::ErrorKind::Io
                && std::error::Error::source(&t)
                    .and_then(|e| e.downcast_ref::<std::io::Error>())
                    .is_some_and(is_timeout) =>
        {
            (Ok(Posted::TimedOut(url.to_owned())), Finish::Answered)
        }
        Err(error) => {
            let refused = matches!(
                &error,
                ureq::Error::Transport(t) if t.kind() == ureq::ErrorKind::ConnectionFailed
            );
            let error = anyhow::Error::from(error)
                .context(format!("speech recognition service at {endpoint}"));
            if refused {
                (Ok(Posted::Refused(error)), Finish::Refused)
            } else {
                (Err(error), Finish::Answered)
            }
        }
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
        self.transcribe(samples, crate::engine::SAMPLE_RATE, deadline)
    }
}

fn wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
    for sample in samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
    }
    writer.finalize()?;
    Ok(cursor.into_inner())
}

fn multipart_body(wav: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(wav.len() + 512);
    for (name, value) in [
        ("language", LANGUAGE),
        ("temperature", "0"),
        ("temperature_inc", TEMPERATURE_STEP),
        ("response_format", "verbose_json"),
        ("timestamp_granularities[]", "word"),
    ] {
        body.extend_from_slice(
            format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"utterance.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(wav);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

#[cfg(test)]
mod tests;
