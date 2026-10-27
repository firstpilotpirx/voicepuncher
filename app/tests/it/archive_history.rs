//! The dictation history read back from an archive folder the archive wrote.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use voicepuncher::adapters::secondary::archiving::filesystem as archiving;
use voicepuncher::core::application as app;
use voicepuncher::core::domain::AudioBuffer16k;

use app::{DictationArchive, DictationHistory, HistoryList};
use archiving::FileSystemDictationHistory;

const NO_LIMIT: u64 = u64::MAX;
const ROWS: usize = 300;

fn at_second(second: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_792_406_525 + second)
}

fn entry(second: u64, pasted: Option<&str>) -> app::ArchiveEntry {
    app::ArchiveEntry {
        started_at: at_second(second),
        start: app::DictationStart::HotkeyHeld,
        audio: AudioBuffer16k::new(vec![0; 160]),
        chunk_texts: vec![],
        raw_transcript: pasted.map(str::to_owned),
        pasted_text: pasted.map(str::to_owned),
        outcome: app::DictationOutcome::Pasted,
        lost_chunks: 0,
        timings: vec![],
    }
}

fn keep(dir: &Path, entries: &[app::ArchiveEntry]) {
    let setup = archiving::ArchiveSetup {
        dictation_model: "model.bin".into(),
        app_version: "0.40.0".into(),
    };
    let mut archive =
        archiving::FileSystemDictationArchive::new(dir.to_path_buf(), NO_LIMIT, setup);
    for entry in entries {
        archive.keep(entry).unwrap();
    }
}

fn texts(dir: &Path, limit: usize) -> Vec<String> {
    let history = FileSystemDictationHistory::new(Some(dir.to_path_buf()));
    match history.latest(limit).unwrap() {
        HistoryList::Rows(rows) => rows.into_iter().map(|row| row.text).collect(),
        HistoryList::ArchiveOff => panic!("the archive reads as off"),
    }
}

#[test]
fn should_list_the_newest_entry_first_with_its_start() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    keep(
        dir.path(),
        &[entry(1, Some("first")), entry(2, Some("second"))],
    );
    // When
    let history = FileSystemDictationHistory::new(Some(dir.path().to_path_buf()));
    let list = history.latest(ROWS).unwrap();
    // Then
    let HistoryList::Rows(rows) = list else {
        panic!("the archive reads as off")
    };
    let read: Vec<_> = rows
        .iter()
        .map(|r| (r.text.as_str(), r.started_at))
        .collect();
    assert_eq!(
        read,
        vec![("second", at_second(2)), ("first", at_second(1))]
    );
}

#[test]
fn should_stop_at_the_limit_counting_only_entries_with_text() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let kept = [
        entry(1, Some("a")),
        entry(2, Some("b")),
        entry(3, None),
        entry(4, Some("d")),
    ];
    keep(dir.path(), &kept);
    // When
    let read = texts(dir.path(), 2);
    // Then
    assert_eq!(read, vec!["d", "b"]);
}

#[test]
fn should_skip_partial_damaged_and_unknown_format_entries() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    keep(
        dir.path(),
        &[entry(1, Some("kept")), entry(5, Some("newest"))],
    );
    for (name, json) in [
        ("2026-10-19T10-42-07.000Z", "{ not json"),
        (
            "2026-10-19T10-42-08.000Z",
            r#"{"format": 9, "started_at": "2026-10-19T10:42:08.000Z"}"#,
        ),
        (
            "2026-10-19T10-42-09.000Z.partial",
            r#"{"format": 2, "started_at": "2026-10-19T10:42:09.000Z", "pasted_text": "half"}"#,
        ),
    ] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::write(dir.path().join(name).join("entry.json"), json).unwrap();
    }
    // When
    let log = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Warn);
    let read = texts(dir.path(), ROWS);
    // Then
    assert_eq!(read, vec!["newest", "kept"]);
    let warnings = log.starting_with("history: skipped archive entry");
    let skipped = ["2026-10-19T10-42-07.000Z", "2026-10-19T10-42-08.000Z"].map(|name| {
        warnings
            .iter()
            .any(|w| w.starts_with(&format!("history: skipped archive entry {name}")))
    });
    assert_eq!(skipped, [true, true], "warnings: {warnings:?}");
    assert!(
        !warnings.iter().any(|w| w.contains(".partial")),
        "warnings: {warnings:?}"
    );
}

#[test]
fn should_read_a_missing_folder_as_empty() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    // When
    let read = texts(&dir.path().join("history"), ROWS);
    // Then
    assert!(read.is_empty(), "rows of a missing folder: {read:?}");
}

#[test]
fn should_read_no_folder_as_archive_off() {
    // Given
    let history = FileSystemDictationHistory::new(None);
    // When
    let list = history.latest(ROWS).unwrap();
    // Then
    assert_eq!(list, HistoryList::ArchiveOff);
}

#[test]
fn should_name_a_new_newest_entry() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    keep(dir.path(), &[entry(1, Some("a"))]);
    let history = FileSystemDictationHistory::new(Some(dir.path().to_path_buf()));
    let before = history.newest_entry();
    // When
    keep(dir.path(), &[entry(2, Some("b"))]);
    // Then
    assert_ne!(history.newest_entry(), before);
}
