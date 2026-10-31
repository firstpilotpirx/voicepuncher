use std::time::{Duration, UNIX_EPOCH};

use super::*;
use crate::history::{write_files, Outcome, ENTRY_FILE};
use crate::streaming::trace::tests::{press_enter, traced};

fn kept() -> Kept {
    Kept { opened: UNIX_EPOCH + Duration::from_secs(1_792_406_525), source: "1-1-1".into() }
}

#[test]
fn should_keep_an_entry_with_the_timeline_from_the_start_of_its_audio() {
    // Given
    let trace = traced();
    // When
    let entry = entry(
        &trace,
        &[(1, press_enter(1.5, 1.8))],
        vec![0.1; 16_000],
        kept(),
    );
    // Then
    assert_eq!(entry.outcome, Outcome::Agreed);
    assert!((entry.reported[0].start - 0.5).abs() < 1e-4);
    let streaming = entry.streaming.expect("the trace");
    assert!((streaming.reports[0].at - 1.0).abs() < 1e-4);
    assert!((streaming.onset - 0.5).abs() < 1e-4);
    assert_eq!(entry.spotted[0].tokens, "ru: пресс энтер");
}

#[test]
fn should_keep_an_utterance_without_a_command_as_none() {
    // Given
    let mut trace = Trace::new(1.0, 1.5, &["en"]);
    trace.heard(0, 1.9, "hello there");
    // When
    let entry = entry(&trace, &[], vec![0.1; 16_000], kept());
    // Then
    assert_eq!(entry.outcome, Outcome::None);
    assert_eq!(entry.transcript, "en: hello there");
}

#[test]
fn should_write_the_trace_into_the_entry_file() {
    // Given
    let dir = tempfile::tempdir().expect("a temporary folder");
    let entry = entry(&traced(), &[], vec![0.1; 16_000], kept());
    // When
    write_files(dir.path(), &entry).expect("written");
    // Then
    let json = std::fs::read_to_string(dir.path().join(ENTRY_FILE)).expect("entry.json");
    let file: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(file["outcome"], "missed");
    assert_eq!(file["streaming"]["reports"][0]["delay_ms"], 300);
    assert_eq!(file["streaming"]["recognizers"][1]["name"], "ru");
}
