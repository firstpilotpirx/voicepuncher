use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::ser::{Serialize, SerializeMap, Serializer};

use super::utc_time::UtcTime;
use super::ArchiveSetup;
use crate::core::application::dictation_archive::{
    ArchiveEntry, DictationOutcome, DictationStart, RefinementOutcome, StageTiming,
};
use crate::core::domain::AudioBuffer16k;

/// Version of the `entry.json` layout; bump it when a field changes meaning or goes away.
const ENTRY_FORMAT: u32 = 1;
pub(super) const AUDIO_FILE: &str = "audio.wav";
pub(super) const ENTRY_FILE: &str = "entry.json";
const AUDIO_CHANNELS: u16 = 1;
const AUDIO_BITS_PER_SAMPLE: u16 = 16;

/// Writes `audio.wav` and `entry.json` of one entry into `folder`.
pub(super) fn write_entry(folder: &Path, entry: &ArchiveEntry, setup: &ArchiveSetup) -> Result<()> {
    write_audio(&folder.join(AUDIO_FILE), &entry.audio)?;
    let json = entry_json(entry, setup)?;
    let path = folder.join(ENTRY_FILE);
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))
}

fn write_audio(path: &Path, audio: &AudioBuffer16k) -> Result<()> {
    let spec = hound::WavSpec {
        channels: AUDIO_CHANNELS,
        sample_rate: AudioBuffer16k::SAMPLE_RATE_HZ,
        bits_per_sample: AUDIO_BITS_PER_SAMPLE,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("failed to create {}", path.display()))?;
    for &sample in audio.samples() {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn entry_json(entry: &ArchiveEntry, setup: &ArchiveSetup) -> Result<String> {
    let file = EntryFile {
        format: ENTRY_FORMAT,
        started_at: UtcTime::from(entry.started_at).iso_8601(),
        start: start_name(entry.start),
        audio: AudioFile {
            file: AUDIO_FILE,
            duration_ms: entry.audio.duration_ms(),
        },
        chunks: &entry.chunk_texts,
        raw_transcript: entry.raw_transcript.as_deref(),
        pasted_text: entry.pasted_text.as_deref(),
        refinement: entry.refinement.map(refinement_name),
        outcome: outcome_name(entry.outcome),
        lost_chunks: entry.lost_chunks,
        timings_ms: Timings(&entry.timings),
        dictation_model: &setup.dictation_model,
        refinement_model: setup.refinement_model.as_deref(),
        app_version: &setup.app_version,
    };
    Ok(serde_json::to_string_pretty(&file)?)
}

/// The `entry.json` layout, format 1.
#[derive(serde::Serialize)]
struct EntryFile<'a> {
    format: u32,
    started_at: String,
    start: &'static str,
    audio: AudioFile,
    chunks: &'a [String],
    raw_transcript: Option<&'a str>,
    pasted_text: Option<&'a str>,
    refinement: Option<&'static str>,
    outcome: &'static str,
    lost_chunks: usize,
    timings_ms: Timings<'a>,
    dictation_model: &'a str,
    refinement_model: Option<&'a str>,
    app_version: &'a str,
}

#[derive(serde::Serialize)]
struct AudioFile {
    file: &'static str,
    duration_ms: u64,
}

/// Stage timings as a JSON object whose keys keep the order the stages ran in.
struct Timings<'a>(&'a [StageTiming]);

impl Serialize for Timings<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for timing in self.0 {
            map.serialize_entry(timing.stage, &timing.ms)?;
        }
        map.end()
    }
}

fn start_name(start: DictationStart) -> &'static str {
    match start {
        DictationStart::HotkeyHeld => "hotkey_held",
        DictationStart::HotkeyToggled => "hotkey_toggled",
        DictationStart::Voice => "voice",
        DictationStart::RecordingFile => "recording_file",
    }
}

fn refinement_name(refinement: RefinementOutcome) -> &'static str {
    match refinement {
        RefinementOutcome::Refined => "refined",
        RefinementOutcome::Off => "off",
        RefinementOutcome::FellBack => "fell_back",
    }
}

fn outcome_name(outcome: DictationOutcome) -> &'static str {
    match outcome {
        DictationOutcome::Pasted => "pasted",
        DictationOutcome::LeftInClipboard => "left_in_clipboard",
        DictationOutcome::PasteFailed => "paste_failed",
        DictationOutcome::SpeechRecognitionFailed => "speech_recognition_failed",
        DictationOutcome::NoText => "no_text",
        DictationOutcome::OnlyCommandWords => "only_command_words",
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    fn sample_entry() -> ArchiveEntry {
        ArchiveEntry {
            started_at: UNIX_EPOCH + Duration::from_millis(1_792_406_525_123),
            start: DictationStart::HotkeyToggled,
            audio: AudioBuffer16k::new(vec![0; 24_000]),
            chunk_texts: vec!["hello".into(), "world".into()],
            raw_transcript: Some("hello world".into()),
            pasted_text: Some("Hello world.".into()),
            refinement: Some(RefinementOutcome::FellBack),
            outcome: DictationOutcome::LeftInClipboard,
            lost_chunks: 1,
            timings: vec![
                StageTiming {
                    stage: "transcribe",
                    ms: 420,
                },
                StageTiming {
                    stage: "refine",
                    ms: 90,
                },
            ],
        }
    }

    fn sample_setup() -> ArchiveSetup {
        ArchiveSetup {
            dictation_model: "ggml-large-v3-turbo-q5_0.bin".into(),
            refinement_model: None,
            app_version: "0.11.0".into(),
        }
    }

    #[test]
    fn should_write_every_entry_field_with_static_facts() {
        // Given
        let entry = sample_entry();
        let setup = sample_setup();
        // When
        let json = entry_json(&entry, &setup).unwrap();
        // Then
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let expected = serde_json::json!({
            "format": 1,
            "started_at": "2026-10-19T10:42:05.123Z",
            "start": "hotkey_toggled",
            "audio": {"file": "audio.wav", "duration_ms": 1500},
            "chunks": ["hello", "world"],
            "raw_transcript": "hello world",
            "pasted_text": "Hello world.",
            "refinement": "fell_back",
            "outcome": "left_in_clipboard",
            "lost_chunks": 1,
            "timings_ms": {"transcribe": 420, "refine": 90},
            "dictation_model": "ggml-large-v3-turbo-q5_0.bin",
            "refinement_model": null,
            "app_version": "0.11.0",
        });
        assert_eq!(value, expected);
    }

    #[test]
    fn should_keep_timings_in_stage_order() {
        // Given
        let entry = sample_entry();
        let setup = sample_setup();
        // When
        let json = entry_json(&entry, &setup).unwrap();
        // Then
        let transcribe = json.find("\"transcribe\"").unwrap();
        let refine = json.find("\"refine\"").unwrap();
        assert!(transcribe < refine);
    }

    #[test]
    fn should_write_null_for_missing_texts_and_refinement() {
        // Given
        let entry = ArchiveEntry {
            raw_transcript: None,
            pasted_text: None,
            refinement: None,
            outcome: DictationOutcome::SpeechRecognitionFailed,
            ..sample_entry()
        };
        // When
        let json = entry_json(&entry, &sample_setup()).unwrap();
        // Then
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["raw_transcript"].is_null());
        assert!(value["pasted_text"].is_null());
        assert!(value["refinement"].is_null());
        assert_eq!(value["outcome"], "speech_recognition_failed");
    }
}
