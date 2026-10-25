use std::time::Duration;

use super::*;
use crate::core::application::ModelRow;
use crate::core::domain::{ModelFile, CATALOG};

const MB: u64 = 1_000_000;
const GB: u64 = 1_000 * MB;
const FOLDER: &str = "/Users/me/.voicepuncher/models";

fn file(name: &'static str, size_bytes: u64) -> ModelFile {
    ModelFile { name, size_bytes, ..CATALOG[0] }
}

fn report(rows: &[(ModelFile, bool)], free: Option<u64>) -> ModelsReport {
    let rows: Vec<ModelRow> = rows
        .iter()
        .map(|(file, present)| ModelRow { file: *file, present: *present, part: 0 })
        .collect();
    let missing_bytes = rows
        .iter()
        .filter(|r| !r.present)
        .map(|r| r.file.size_bytes)
        .sum();
    ModelsReport { rows, missing_bytes, free_bytes: free }
}

fn two_missing(free: Option<u64>) -> WindowModel {
    WindowModel::new(
        &report(
            &[
                (file("a.bin", GB), false),
                (file("b.bin", 2 * GB), false),
                (file("c.bin", 300 * MB), true),
            ],
            free,
        ),
        Path::new(FOLDER),
    )
}

#[test]
fn should_size_files_in_decimal_units() {
    assert_eq!(human_size(574_041_195), "574 MB");
    assert_eq!(human_size(3_578_079_573), "3.6 GB");
    assert_eq!(human_size(643_854), "644 KB");
    assert_eq!(human_size(17_626_723), "18 MB");
    assert_eq!(human_size(512), "512 B");
}

#[test]
fn should_tell_the_time_left_in_seconds_minutes_and_hours() {
    assert_eq!(time_left(45), "45 s left");
    assert_eq!(time_left(100), "2 min left");
    assert_eq!(time_left(3900), "1 h 5 min left");
}

#[test]
fn should_word_every_row_status() {
    assert_eq!(status_text(&RowStatus::Present), "Present ✓");
    assert_eq!(status_text(&RowStatus::Missing), "Missing");
    assert_eq!(
        status_text(&RowStatus::Downloading { percent: 42 }),
        "Downloading 42 %"
    );
    assert_eq!(status_text(&RowStatus::Verifying), "Verifying…");
    assert_eq!(
        status_text(&RowStatus::Failed(DownloadFailure::NoInternet)),
        "Failed: no internet connection"
    );
    assert_eq!(
        status_text(&RowStatus::MismatchRetrying),
        "Failed: checksum mismatch — downloading again"
    );
    assert_eq!(status_text(&RowStatus::Cancelled), "Cancelled");
}

#[test]
fn should_list_missing_and_present_files_before_a_download() {
    // Given / When
    let model = two_missing(Some(120 * GB));
    // Then
    let statuses: Vec<&RowStatus> = model.rows.iter().map(|r| &r.status).collect();
    assert_eq!(
        statuses,
        [
            &RowStatus::Missing,
            &RowStatus::Missing,
            &RowStatus::Present
        ]
    );
    assert_eq!(model.total_line(), "0 B of 3.0 GB");
    assert_eq!(model.space_line(), ("Free space: 120 GB".into(), false));
    assert_eq!(model.main_button(), MainButton::Download);
    assert!(model.main_enabled());
    assert!(!model.done_enabled());
    assert!(
        model.intro().contains("3.3 GB in total"),
        "{}",
        model.intro()
    );
    assert!(model.intro().contains(FOLDER), "{}", model.intro());
}

#[test]
fn should_show_the_percent_the_total_the_rate_and_the_time_left_while_downloading() {
    // Given
    let mut model = two_missing(Some(120 * GB));
    model.started();
    // When
    model.apply(&DownloadEvent::Done("a.bin"));
    model.apply(&DownloadEvent::Started("b.bin"));
    model.apply(&DownloadEvent::Progress {
        name: "b.bin",
        received: 840 * MB,
        total: 2 * GB,
        bytes_per_s: 18 * MB,
    });
    // Then
    assert_eq!(model.rows[0].status, RowStatus::Present);
    assert_eq!(model.rows[1].status, RowStatus::Downloading { percent: 42 });
    assert_eq!(
        model.total_line(),
        "1.8 GB of 3.0 GB · 18 MB/s · 2 min left"
    );
    assert_eq!(model.main_button(), MainButton::Cancel);
    assert!(model.main_enabled());
    assert!(!model.done_enabled());
}

#[test]
fn should_turn_the_free_space_line_red_when_the_disk_is_too_full() {
    // Given
    let mut model = two_missing(Some(GB));
    // Then
    assert_eq!(
        model.space_line(),
        ("Not enough space: needs 3.0 GB, 1.0 GB free".into(), true)
    );
    assert!(!model.main_enabled());
    // When the download is refused
    model.apply(&DownloadEvent::NotEnoughSpace { needed: 3 * GB, free: GB });
    assert!(model.space_line().1);
}

#[test]
fn should_offer_retry_after_a_failed_file() {
    // Given
    let mut model = two_missing(Some(120 * GB));
    model.started();
    // When
    model.apply(&DownloadEvent::Retrying {
        name: "a.bin",
        attempt: 1,
        pause: Duration::from_secs(2),
    });
    model.apply(&DownloadEvent::Failed { name: "a.bin", failure: DownloadFailure::NoInternet });
    model.ended();
    // Then
    assert_eq!(
        model.rows[0].status,
        RowStatus::Failed(DownloadFailure::NoInternet)
    );
    assert_eq!(model.main_button(), MainButton::Retry);
    assert!(model.main_enabled());
}

#[test]
fn should_enable_done_once_every_file_is_present() {
    // Given
    let mut model = two_missing(Some(120 * GB));
    model.started();
    // When
    model.apply(&DownloadEvent::Done("a.bin"));
    model.apply(&DownloadEvent::Done("b.bin"));
    model.apply(&DownloadEvent::AllPresent);
    model.ended();
    // Then
    assert!(model.done_enabled());
    assert!(!model.main_enabled());
    assert_eq!(model.total_line(), "3.0 GB of 3.0 GB");
}

#[test]
fn should_open_from_the_menu_with_every_row_present_and_done_enabled() {
    // Given every model present
    let model = WindowModel::new(
        &report(
            &[(file("a.bin", GB), true), (file("b.bin", GB), true)],
            None,
        ),
        Path::new(FOLDER),
    );
    // Then
    assert!(model.rows.iter().all(|r| r.status == RowStatus::Present));
    assert!(model.done_enabled());
    assert!(
        (model.total_fraction() - 1.0).abs() < 1e-6,
        "{} is not {}",
        model.total_fraction(),
        1.0
    );
}

#[test]
fn should_keep_a_failure_shown_across_a_recheck() {
    // Given
    let mut model = two_missing(Some(120 * GB));
    model.apply(&DownloadEvent::Failed {
        name: "a.bin",
        failure: DownloadFailure::ChecksumMismatch,
    });
    // When
    model.rechecked(&report(
        &[
            (file("a.bin", GB), false),
            (file("b.bin", 2 * GB), true),
            (file("c.bin", 300 * MB), true),
        ],
        Some(120 * GB),
    ));
    // Then
    assert_eq!(
        model.rows[0].status,
        RowStatus::Failed(DownloadFailure::ChecksumMismatch)
    );
    assert_eq!(model.total_line(), "2.0 GB of 3.0 GB");
}
