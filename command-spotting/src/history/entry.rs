//! One command history entry and the files it is kept as: `audio.wav` and `entry.json`.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::Serialize;
use voicepuncher_entry_folders::UtcTime;

use super::Outcome;
use crate::audio::SAMPLE_RATE;
use crate::conversions::saturating_i16;
use crate::protocol::Verdict;
use crate::streaming::trace::Trace;

/// Version of the `entry.json` layout; bump it when a field changes meaning or goes away.
const ENTRY_FORMAT: u32 = 1;
pub const AUDIO_FILE: &str = "audio.wav";
pub const ENTRY_FILE: &str = "entry.json";

/// A phrase the keyword spotter spotted, the tokens it decoded and where, in seconds of the
/// entry's audio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpottedPhrase {
    pub phrase: String,
    pub tokens: String,
    pub start: f32,
    pub end: f32,
}

/// A hearing of the speech recognition service for commands: what it was asked as, the audio
/// it was given and what it heard; no text when it gave no answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hearing {
    pub kind: String,
    pub start: f32,
    pub end: f32,
    pub text: Option<String>,
}

/// Commands the service reported to the app, said together at `start..end`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reported {
    pub commands: Vec<String>,
    pub start: f32,
    pub end: f32,
    /// The speech recognition service heard them; else they are the keyword spotter's alone.
    pub heard: bool,
    /// What the speech recognition service made of them, heard after they went out without
    /// confirmation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whisper: Option<Verdict>,
}

/// One utterance in which either model heard a command.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub started_at: SystemTime,
    /// Tells apart entries of other processes and connections that start in the same instant.
    pub source: String,
    /// 16 kHz mono, as the microphone gave it.
    pub audio: Vec<f32>,
    pub spotted: Vec<SpottedPhrase>,
    pub hearings: Vec<Hearing>,
    /// What the segment's own hearing heard, else the last hearing.
    pub transcript: String,
    pub reported: Vec<Reported>,
    pub outcome: Outcome,
    /// What the streaming recognizers heard of it; none from the keyword spotter.
    pub streaming: Option<Trace>,
}

impl Entry {
    /// The name its folder starts with: its start, then its source.
    pub fn folder_base(&self) -> String {
        format!(
            "{}-{}",
            UtcTime::from(self.started_at).folder_name(),
            self.source
        )
    }
}

#[derive(Serialize)]
struct EntryFile<'a> {
    format: u32,
    started_at: String,
    outcome: Outcome,
    audio: AudioFile,
    spotted: &'a [SpottedPhrase],
    transcript: &'a str,
    reported: &'a [Reported],
    hearings: &'a [Hearing],
    #[serde(skip_serializing_if = "Option::is_none")]
    streaming: Option<&'a Trace>,
    service_version: &'static str,
}

#[derive(Serialize)]
struct AudioFile {
    file: &'static str,
    duration_ms: u64,
}

/// Writes `audio.wav` and `entry.json` of `entry` into `folder`.
pub fn write_files(folder: &Path, entry: &Entry) -> Result<()> {
    write_wav(&folder.join(AUDIO_FILE), &entry.audio)?;
    let file = EntryFile {
        format: ENTRY_FORMAT,
        started_at: UtcTime::from(entry.started_at).iso_8601(),
        outcome: entry.outcome,
        audio: AudioFile { file: AUDIO_FILE, duration_ms: duration_ms(entry.audio.len()) },
        spotted: &entry.spotted,
        transcript: &entry.transcript,
        reported: &entry.reported,
        hearings: &entry.hearings,
        streaming: entry.streaming.as_ref(),
        service_version: env!("CARGO_PKG_VERSION"),
    };
    let path = folder.join(ENTRY_FILE);
    fs::write(&path, serde_json::to_string_pretty(&file)?)
        .with_context(|| format!("failed to write {}", path.display()))
}

/// Writes `samples` (16 kHz mono, -1..1) to `path` as a 16-bit WAV file.
pub fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("failed to create {}", path.display()))?;
    for sample in samples {
        writer.write_sample(saturating_i16(
            sample.clamp(-1.0, 1.0) * f32::from(i16::MAX),
        ))?;
    }
    writer.finalize()?;
    Ok(())
}

fn duration_ms(samples: usize) -> u64 {
    u64::try_from(samples).unwrap_or(u64::MAX) * 1_000 / u64::from(SAMPLE_RATE)
}
