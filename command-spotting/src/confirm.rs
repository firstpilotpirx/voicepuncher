//! Confirms short utterances with the speech recognition service: what was said, in full,
//! decides whether the utterance is a command phrase.

use std::collections::VecDeque;
use std::io::Cursor;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

const BOUNDARY: &str = "voicepuncher-command-spotting-3f9a";
const LANGUAGE: &str = "en";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The commands, as a hint that steers the speech recognition service towards them.
const PROMPT: &str =
    "start dictation. stop dictation. cancel dictation. press enter. clear text. clear all text.";

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

#[derive(Debug, Deserialize)]
struct Transcription {
    text: String,
}

/// A client of the speech recognition service (OpenAI Audio Transcriptions API).
pub struct UtteranceTranscriber {
    endpoint: String,
}

impl UtteranceTranscriber {
    /// `base_url` like `http://127.0.0.1:8081/v1`.
    pub fn new(base_url: &str) -> Self {
        Self {
            endpoint: format!("{}/audio/transcriptions", base_url.trim_end_matches('/')),
        }
    }

    /// What was said in `samples` (16 kHz mono).
    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> Result<String> {
        let body = multipart_body(&wav(samples, sample_rate)?);
        let response = ureq::post(&self.endpoint)
            .timeout(REQUEST_TIMEOUT)
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .send_bytes(&body)
            .with_context(|| format!("speech recognition service at {}", self.endpoint))?
            .into_string()?;
        let transcription: Transcription =
            serde_json::from_str(&response).context("speech recognition service answer")?;
        Ok(transcription.text)
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
        ("prompt", PROMPT),
        ("temperature", "0"),
        ("response_format", "json"),
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
        assert!(body.contains("press enter"));
        assert!(body.ends_with(&format!("--{BOUNDARY}--\r\n")));
    }
}
