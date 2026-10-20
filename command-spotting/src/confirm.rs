//! Confirms short utterances with the speech recognition service: what was said, in full,
//! decides whether the utterance is a command phrase.

use std::collections::VecDeque;
use std::io::Cursor;
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

/// The last seconds of a connection's audio, by absolute sample position.
pub struct RecentAudio {
    samples: VecDeque<f32>,
    first: usize,
    capacity: usize,
}

impl RecentAudio {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            first: 0,
            capacity,
        }
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

/// A client of the speech recognition service (OpenAI Audio Transcriptions API), one
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
    /// free copy and for its answer together never pass `deadline`.
    pub fn transcribe(
        &self,
        samples: &[f32],
        sample_rate: u32,
        deadline: Duration,
    ) -> Result<Heard> {
        let started = Instant::now();
        let body = multipart_body(&wav(samples, sample_rate)?);
        let lease = self.pool.lease(deadline).map_err(|e| {
            log::warn!("confirmation given up unsent: {e}");
            anyhow!("speech recognition service: {e}")
        })?;
        let port = lease.port();
        let left = deadline.saturating_sub(started.elapsed());
        let ran = lease.run(left, move |url| post(url, &body, left));
        match ran {
            Ran::Answered(answer) => heard_from(&answer?),
            Ran::GaveUp => bail!(
                "speech recognition copy {port} sent no answer in {} ms",
                deadline.as_millis()
            ),
        }
    }
}

/// Posts `body` to the copy at `url` and reads the whole answer, within `timeout`.
fn post(url: &str, body: &[u8], timeout: Duration) -> (Result<String>, Finish) {
    let endpoint = format!("{}/audio/transcriptions", url.trim_end_matches('/'));
    let sent = ureq::post(&endpoint)
        .timeout(timeout)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .send_bytes(body);
    match sent {
        Ok(response) => (
            response.into_string().map_err(anyhow::Error::from),
            Finish::Answered,
        ),
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
            let error = anyhow::Error::from(error)
                .context(format!("speech recognition service at {endpoint}"));
            (Err(error), finish)
        }
    }
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
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use voicepuncher_copy_pool::CopyPool;

    /// A fake copy on its own port: answers every request with `answer` and counts them.
    fn spawn_copy(answer: &'static str) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&requests);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut first = [0u8; 1];
                if stream.peek(&mut first).unwrap_or(0) == 0 {
                    continue;
                }
                counter.fetch_add(1, Ordering::SeqCst);
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0usize;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse().unwrap();
                        }
                    }
                }
                reader.read_exact(&mut vec![0u8; length]).unwrap();
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                    answer.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        (url, requests)
    }

    const PRESS_ENTER: &str = r#"{"text":" press enter","segments":[]}"#;
    const DEADLINE: Duration = Duration::from_millis(300);
    const SAMPLE_RATE: u32 = 16_000;

    #[test]
    fn should_give_a_confirmation_up_unsent_when_every_copy_stays_busy_past_its_deadline() {
        // Given
        let lines = crate::test_log::captured();
        let (url, requests) = spawn_copy(PRESS_ENTER);
        let transcriber = UtteranceTranscriber::new(&url).unwrap();
        let held = CopyPool::shared(&url).unwrap().lease(DEADLINE).unwrap();
        // When
        let started = Instant::now();
        let result = transcriber.transcribe(&[0.0; 1_600], SAMPLE_RATE, DEADLINE);
        // Then
        assert!(result.is_err());
        let given_up = lines.containing("confirmation given up unsent");
        assert!(
            given_up.iter().any(|line| line.contains(&url)),
            "{given_up:?}"
        );
        assert!(
            started.elapsed() < DEADLINE * 2,
            "waited {:?}",
            started.elapsed()
        );
        assert_eq!(requests.load(Ordering::SeqCst), 0);
        drop(held);
    }

    #[test]
    fn should_send_a_confirmation_to_the_copy_that_frees_before_its_deadline() {
        // Given
        let (url, requests) = spawn_copy(PRESS_ENTER);
        let transcriber = UtteranceTranscriber::new(&url).unwrap();
        let held = CopyPool::shared(&url).unwrap().lease(DEADLINE).unwrap();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            held.finished();
        });
        // When
        let heard = transcriber
            .transcribe(&[0.0; 1_600], SAMPLE_RATE, Duration::from_secs(2))
            .unwrap();
        // Then
        assert_eq!(heard.text, " press enter");
        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn should_hold_only_the_last_samples() {
        // Given
        let mut recent = RecentAudio::new(4);
        // When
        recent.push(&[1.0, 2.0, 3.0]);
        recent.push(&[4.0, 5.0, 6.0]);
        // Then samples 2..6 are held
        assert_eq!(recent.between(0, 10), [3.0, 4.0, 5.0, 6.0]);
        assert_eq!(recent.between(3, 5), [4.0, 5.0]);
        assert!(recent.between(8, 10).is_empty());
    }

    #[test]
    fn should_put_the_fields_before_the_audio_in_the_request() {
        // When
        let body = String::from_utf8_lossy(&multipart_body(b"RIFF")).into_owned();
        // Then
        let language = body.find("name=\"language\"").unwrap();
        let file = body.find("name=\"file\"").unwrap();
        assert!(language < file);
        assert!(body.contains("name=\"language\"\r\n\r\nen\r\n"));
        assert!(body.contains("name=\"temperature\"\r\n\r\n0\r\n"));
        assert!(body.contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
        assert!(body.contains("name=\"timestamp_granularities[]\"\r\n\r\nword\r\n"));
        assert!(body.ends_with(&format!("--{BOUNDARY}--\r\n")));
    }

    #[test]
    fn should_ask_for_no_retry_of_a_confirmation() {
        // When
        let body = String::from_utf8_lossy(&multipart_body(b"RIFF")).into_owned();
        // Then
        assert!(body.contains("name=\"temperature\"\r\n\r\n0\r\n"));
        assert!(body.contains("name=\"temperature_inc\"\r\n\r\n0\r\n"));
    }

    #[test]
    fn should_send_no_prompt_with_a_confirmation() {
        // When
        let body = String::from_utf8_lossy(&multipart_body(b"RIFF")).into_owned();
        // Then
        assert!(!body.contains("name=\"prompt\""));
        assert!(!body.contains("name=\"max_context\""));
        assert!(!body.contains("name=\"carry_initial_prompt\""));
    }

    #[test]
    fn should_read_the_words_and_their_times() {
        // Given
        let answer = r#"{"text":" we meet stop","segments":[
            {"start":0.0,"end":1.0,"words":[{"word":" we","start":0.0,"end":0.2},{"word":" meet","start":0.2,"end":0.6}]},
            {"start":1.0,"end":1.4,"words":[{"word":" stop","start":1.0,"end":1.4}]}]}"#;
        // When
        let heard = heard_from(answer).unwrap();
        // Then
        assert_eq!(heard.text, " we meet stop");
        assert_eq!(heard.word_texts(), [" we", " meet", " stop"]);
        assert_eq!((heard.words[2].start, heard.words[2].end), (1.0, 1.4));
    }

    #[test]
    fn should_join_the_pieces_of_a_word() {
        // Given
        let answer = r#"{"text":" top ender.","segments":[{"words":[
            {"word":" top","start":0.0,"end":0.3},{"word":" end","start":0.3,"end":0.5},
            {"word":"er","start":0.5,"end":0.7},{"word":".","start":0.7,"end":0.7}]}]}"#;
        // When
        let heard = heard_from(answer).unwrap();
        // Then
        assert_eq!(heard.word_texts(), [" top", " ender."]);
        assert_eq!((heard.words[1].start, heard.words[1].end), (0.3, 0.7));
    }
}
