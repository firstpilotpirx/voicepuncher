use std::time::{Duration, UNIX_EPOCH};

use super::*;
use crate::history::{Hearing, Outcome, Reported, SpottedPhrase, AUDIO_FILE, ENTRY_FILE};

fn entry(second: u64) -> Entry {
    Entry {
        started_at: UNIX_EPOCH + Duration::from_secs(1_792_406_525 + second),
        source: "1-0".to_owned(),
        audio: vec![0.25; 16_000],
        spotted: vec![SpottedPhrase {
            phrase: "press enter".to_owned(),
            tokens: "▁PRESS ▁EN TER".to_owned(),
            start: 0.3,
            end: 0.8,
        }],
        hearings: vec![Hearing {
            kind: "Segment".to_owned(),
            start: 0.0,
            end: 1.0,
            text: Some(" Press enter.".to_owned()),
        }],
        transcript: " Press enter.".to_owned(),
        reported: vec![Reported {
            commands: vec!["press enter".to_owned()],
            start: 0.3,
            end: 0.8,
            heard: true,
            whisper: None,
        }],
        outcome: Outcome::Agreed,
    }
}

/// The entry folders in `dir`, by name.
fn names(dir: &std::path::Path) -> Vec<String> {
    let entries: std::collections::BTreeSet<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    entries.into_iter().collect()
}

#[test]
fn should_keep_playable_audio_and_both_models_results() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let cfg = HistoryCfg { dir: dir.path().to_owned(), max_size_bytes: u64::MAX };
    // When
    let name = keep(&cfg, &entry(0)).unwrap();
    // Then
    let folder = dir.path().join(&name);
    let wav = hound::WavReader::open(folder.join(AUDIO_FILE)).unwrap();
    assert_eq!((wav.spec().sample_rate, wav.len()), (16_000, 16_000));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join(ENTRY_FILE)).unwrap()).unwrap();
    assert_eq!(json["outcome"], "agreed");
    assert_eq!(json["spotted"][0]["phrase"], "press enter");
    assert_eq!(json["transcript"], " Press enter.");
    assert_eq!(json["reported"][0]["commands"][0], "press enter");
    assert_eq!(json["started_at"], "2026-10-19T10:42:05.000Z");
}

#[test]
fn should_remove_the_oldest_entries_but_never_the_new_one() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let cfg = HistoryCfg { dir: dir.path().to_owned(), max_size_bytes: 40_000 };
    // When
    let kept: Vec<String> = (0..3)
        .map(|second| keep(&cfg, &entry(second)).unwrap())
        .collect();
    // Then
    assert_eq!(names(dir.path()), vec![kept[2].clone()]);
}

#[test]
fn should_write_in_the_background_and_warn_when_the_folder_cannot_be_written() {
    // Given
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Warn);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-folder");
    std::fs::write(&file, "").unwrap();
    let history =
        CommandHistory::start(HistoryCfg { dir: file.clone(), max_size_bytes: u64::MAX }).unwrap();
    // When
    history.keep(entry(0));
    history.flush();
    std::fs::remove_file(&file).unwrap();
    history.keep(entry(1));
    history.flush();
    // Then
    assert_eq!(lines.containing("was not kept").len(), 1);
    assert_eq!(names(&file).len(), 1);
}
