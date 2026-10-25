//! Statuses, sizes and times in the words the Download window shows them.

use super::RowStatus;

/// A row's status in words.
pub fn status_text(status: &RowStatus) -> String {
    match status {
        RowStatus::Present => "Present ✓".into(),
        RowStatus::Missing => "Missing".into(),
        RowStatus::Downloading { percent } => format!("Downloading {percent} %"),
        RowStatus::Verifying => "Verifying…".into(),
        RowStatus::Retrying => "No connection, retrying…".into(),
        RowStatus::MismatchRetrying => "Failed: checksum mismatch — downloading again".into(),
        RowStatus::Failed(failure) => format!("Failed: {failure}"),
        RowStatus::Cancelled => "Cancelled".into(),
    }
}

/// Decimal units as Finder shows them: `547 MB`, `3.3 GB`, `864 KB`.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = "B";
    for next in UNITS {
        if value < 1000.0 {
            break;
        }
        value /= 1000.0;
        unit = next;
    }
    if value < 10.0 && unit != "KB" {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.0} {unit}")
    }
}

/// `45 s left`, `2 min left`, `1 h 5 min left`.
pub fn time_left(seconds: u64) -> String {
    match seconds {
        0..60 => format!("{seconds} s left"),
        60..3600 => format!("{} min left", seconds.div_ceil(60)),
        _ => format!("{} h {} min left", seconds / 3600, seconds % 3600 / 60),
    }
}
