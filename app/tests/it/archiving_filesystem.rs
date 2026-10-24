use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use voicepuncher::adapters::secondary::archiving::filesystem::{
    ArchiveSetup, FileSystemDictationArchive,
};
use voicepuncher::core::application::dictation_archive::{
    ArchiveEntry, DictationOutcome, DictationStart, StageTiming,
};
use voicepuncher::core::application::DictationArchive;
use voicepuncher::core::domain::AudioBuffer16k;

const ONE_SECOND_OF_SAMPLES: usize = 16_000;
const NO_LIMIT: u64 = u64::MAX;

fn at_second(second: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_792_406_525 + second)
}

fn entry(started_at: SystemTime, samples: Vec<i16>) -> ArchiveEntry {
    ArchiveEntry {
        started_at,
        start: DictationStart::Voice,
        audio: AudioBuffer16k::new(samples),
        chunk_texts: vec!["hello world".into()],
        raw_transcript: Some("hello world".into()),
        pasted_text: Some("Hello world.".into()),
        outcome: DictationOutcome::Pasted,
        lost_chunks: 0,
        timings: vec![StageTiming { stage: "transcribe", ms: 300 }],
    }
}

fn archive(dir: &Path, max_size_bytes: u64) -> FileSystemDictationArchive {
    let setup = ArchiveSetup {
        dictation_model: "ggml-large-v3-turbo-q5_0.bin".into(),
        app_version: "0.21.0".into(),
    };
    FileSystemDictationArchive::new(dir.to_path_buf(), max_size_bytes, setup)
}

fn folder_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|item| item.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn folder_size(folder: &Path) -> u64 {
    fs::read_dir(folder)
        .unwrap()
        .map(|item| item.unwrap().metadata().unwrap().len())
        .sum()
}

#[test]
fn should_write_readable_audio_and_parsable_entry() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let samples: Vec<i16> = (0..1_000).map(|i| i16::try_from(i).unwrap()).collect();
    // When
    archive(dir.path(), NO_LIMIT)
        .keep(&entry(at_second(0), samples.clone()))
        .unwrap();
    // Then
    let folder = dir.path().join("2026-10-19T10-42-05.000Z");
    let mut reader = hound::WavReader::open(folder.join("audio.wav")).unwrap();
    assert_eq!(reader.spec().sample_rate, 16_000);
    assert_eq!(reader.spec().channels, 1);
    let read: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
    assert_eq!(read, samples);
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(folder.join("entry.json")).unwrap()).unwrap();
    assert_eq!(json["format"], 2);
    assert_eq!(json["start"], "voice");
}

#[test]
fn should_suffix_a_taken_name() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let mut archive = archive(dir.path(), NO_LIMIT);
    // When
    for _ in 0..3 {
        archive.keep(&entry(at_second(0), vec![0; 10])).unwrap();
    }
    // Then
    assert_eq!(
        folder_names(dir.path()),
        vec![
            "2026-10-19T10-42-05.000Z",
            "2026-10-19T10-42-05.000Z-2",
            "2026-10-19T10-42-05.000Z-3",
        ]
    );
}

#[test]
fn should_remove_the_oldest_entries_first() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let samples = vec![0; ONE_SECOND_OF_SAMPLES];
    archive(dir.path(), NO_LIMIT)
        .keep(&entry(at_second(0), samples.clone()))
        .unwrap();
    let entry_size = folder_size(&dir.path().join("2026-10-19T10-42-05.000Z"));
    let mut full_archive = archive(dir.path(), entry_size * 3);
    for second in 1..3 {
        full_archive
            .keep(&entry(at_second(second), samples.clone()))
            .unwrap();
    }
    let mut archive = archive(dir.path(), entry_size * 2);
    // When
    archive.keep(&entry(at_second(3), samples)).unwrap();
    // Then
    assert_eq!(
        folder_names(dir.path()),
        vec!["2026-10-19T10-42-07.000Z", "2026-10-19T10-42-08.000Z"]
    );
}

#[test]
fn should_keep_the_new_entry_even_over_the_limit() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let mut archive = archive(dir.path(), 1);
    archive.keep(&entry(at_second(0), vec![0; 10])).unwrap();
    // When
    archive.keep(&entry(at_second(1), vec![0; 10])).unwrap();
    // Then
    assert_eq!(folder_names(dir.path()), vec!["2026-10-19T10-42-06.000Z"]);
}

#[test]
fn should_leave_partial_folders_and_files_when_pruning() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("2000-01-01T00-00-00.000Z.partial")).unwrap();
    fs::write(dir.path().join("1999-notes.txt"), "notes").unwrap();
    let mut archive = archive(dir.path(), 1);
    // When
    archive.keep(&entry(at_second(0), vec![0; 10])).unwrap();
    // Then
    assert_eq!(
        folder_names(dir.path()),
        vec![
            "1999-notes.txt",
            "2000-01-01T00-00-00.000Z.partial",
            "2026-10-19T10-42-05.000Z",
        ]
    );
}
