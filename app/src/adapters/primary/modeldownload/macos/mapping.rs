//! What the Download window shows, as pure functions of the models check and the download's
//! events: each row's status in words, sizes, the total line with its rate and time left, the
//! free-space line and the buttons.

use std::path::{Path, PathBuf};

use crate::core::application::{DownloadEvent, DownloadFailure, ModelsReport};

pub const TITLE: &str = "Download speech models";
pub const OPEN_FOLDER: &str = "Open Folder";
pub const QUIT: &str = "Quit";
pub const DONE: &str = "Done";

/// Where one file stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowStatus {
    Present,
    Missing,
    Downloading {
        percent: u64,
    },
    Verifying,
    /// A connection failure; another try follows.
    Retrying,
    MismatchRetrying,
    Failed(DownloadFailure),
    Cancelled,
}

/// One file of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: &'static str,
    pub purpose: &'static str,
    pub size_bytes: u64,
    pub status: RowStatus,
    /// Bytes on disk: the whole file once present, else its part.
    pub received: u64,
    /// Missing when the window was opened, so it counts in the total.
    counted: bool,
}

/// The Download button's three faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainButton {
    Download,
    Cancel,
    Retry,
}

impl MainButton {
    pub fn title(self) -> &'static str {
        match self {
            MainButton::Download => "Download",
            MainButton::Cancel => "Cancel",
            MainButton::Retry => "Retry",
        }
    }
}

/// Everything the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowModel {
    pub rows: Vec<Row>,
    pub folder: PathBuf,
    pub free_bytes: Option<u64>,
    pub running: bool,
    /// Bytes per second of the file in progress.
    pub rate: u64,
    /// Download refused for space: needed and free bytes.
    pub refused: Option<(u64, u64)>,
}

impl WindowModel {
    /// The window as a fresh check shows it.
    pub fn new(report: &ModelsReport, folder: &Path) -> Self {
        let rows = report
            .rows
            .iter()
            .map(|row| Row {
                name: row.file.name,
                purpose: row.file.purpose,
                size_bytes: row.file.size_bytes,
                status: if row.present {
                    RowStatus::Present
                } else {
                    RowStatus::Missing
                },
                received: if row.present {
                    row.file.size_bytes
                } else {
                    row.part
                },
                counted: !row.present,
            })
            .collect();
        Self {
            rows,
            folder: folder.to_path_buf(),
            free_bytes: report.free_bytes,
            running: false,
            rate: 0,
            refused: None,
        }
    }

    /// A new check after a download or from the menu; the files still in the total stay in it.
    pub fn rechecked(&mut self, report: &ModelsReport) {
        let mut fresh = Self::new(report, &self.folder);
        for row in &mut fresh.rows {
            if let Some(old) = self.rows.iter().find(|old| old.name == row.name) {
                row.counted |= old.counted;
                if row.status == RowStatus::Missing
                    && matches!(old.status, RowStatus::Failed(_) | RowStatus::Cancelled)
                {
                    row.status = old.status.clone();
                }
            }
        }
        fresh.refused = self.refused;
        *self = fresh;
    }

    /// A download starts.
    pub fn started(&mut self) {
        self.running = true;
        self.refused = None;
        for row in &mut self.rows {
            if matches!(row.status, RowStatus::Failed(_) | RowStatus::Cancelled) {
                row.status = RowStatus::Missing;
            }
        }
    }

    /// The download returned.
    pub fn ended(&mut self) {
        self.running = false;
        self.rate = 0;
    }

    pub fn apply(&mut self, event: &DownloadEvent) {
        match event {
            DownloadEvent::NotEnoughSpace { needed, free } => {
                self.refused = Some((*needed, *free));
                self.free_bytes = Some(*free);
            }
            DownloadEvent::Started(name) => {
                self.rate = 0;
                self.set(name, RowStatus::Downloading { percent: 0 });
            }
            DownloadEvent::Verifying(name) => self.set(name, RowStatus::Verifying),
            DownloadEvent::Progress {
                name,
                received,
                total,
                bytes_per_s,
            } => {
                self.rate = *bytes_per_s;
                if let Some(row) = self.row(name) {
                    row.received = *received;
                    row.status = RowStatus::Downloading {
                        percent: percent(*received, *total),
                    };
                }
            }
            DownloadEvent::Retrying { name, .. } => self.set(name, RowStatus::Retrying),
            DownloadEvent::MismatchRetrying(name) => {
                if let Some(row) = self.row(name) {
                    row.received = 0;
                    row.status = RowStatus::MismatchRetrying;
                }
            }
            DownloadEvent::Done(name) => {
                if let Some(row) = self.row(name) {
                    row.received = row.size_bytes;
                    row.status = RowStatus::Present;
                }
            }
            DownloadEvent::Failed { name, failure } => {
                self.set(name, RowStatus::Failed(failure.clone()))
            }
            DownloadEvent::Cancelled(name) => self.set(name, RowStatus::Cancelled),
            DownloadEvent::AllPresent => {}
        }
    }

    fn row(&mut self, name: &str) -> Option<&mut Row> {
        self.rows.iter_mut().find(|row| row.name == name)
    }

    fn set(&mut self, name: &str, status: RowStatus) {
        if let Some(row) = self.row(name) {
            row.status = status;
        }
    }

    pub fn all_present(&self) -> bool {
        self.rows.iter().all(|row| row.status == RowStatus::Present)
    }

    fn total_bytes(&self) -> u64 {
        self.rows
            .iter()
            .filter(|r| r.counted)
            .map(|r| r.size_bytes)
            .sum()
    }

    fn received_bytes(&self) -> u64 {
        self.rows
            .iter()
            .filter(|r| r.counted)
            .map(|r| r.received.min(r.size_bytes))
            .sum()
    }

    fn missing_bytes(&self) -> u64 {
        self.total_bytes() - self.received_bytes()
    }

    /// The share of the total received, from 0 to 1; 1 when nothing was missing.
    pub fn total_fraction(&self) -> f64 {
        match self.total_bytes() {
            0 => 1.0,
            total => self.received_bytes() as f64 / total as f64,
        }
    }

    /// The line under the table's intro.
    pub fn intro(&self) -> String {
        let total: u64 = self.rows.iter().map(|r| r.size_bytes).sum();
        format!(
            "voicepuncher works offline. It needs these files once, {} in total. They go to {}.",
            human_size(total),
            self.folder.display()
        )
    }

    /// `1.2 GB of 3.3 GB · 18 MB/s · 2 min left`; the rate and time only while running.
    pub fn total_line(&self) -> String {
        let mut line = format!(
            "{} of {}",
            human_size(self.received_bytes()),
            human_size(self.total_bytes())
        );
        if self.running && self.rate > 0 {
            line.push_str(&format!(
                " · {}/s · {}",
                human_size(self.rate),
                time_left(self.missing_bytes() / self.rate)
            ));
        }
        line
    }

    /// The free-space line, and whether it is the red one of a disk too full.
    pub fn space_line(&self) -> (String, bool) {
        let needed = self.missing_bytes();
        match (self.refused, self.free_bytes) {
            (Some((needed, free)), _) => (not_enough(needed, free), true),
            (None, Some(free)) if free < needed => (not_enough(needed, free), true),
            (None, Some(free)) => (format!("Free space: {}", human_size(free)), false),
            (None, None) => (String::new(), false),
        }
    }

    pub fn main_button(&self) -> MainButton {
        if self.running {
            MainButton::Cancel
        } else if self
            .rows
            .iter()
            .any(|row| matches!(row.status, RowStatus::Failed(_)))
        {
            MainButton::Retry
        } else {
            MainButton::Download
        }
    }

    /// Cancel always; Download and Retry while a file is missing and the space suffices.
    pub fn main_enabled(&self) -> bool {
        self.running || (!self.all_present() && !self.space_line().1)
    }

    pub fn done_enabled(&self) -> bool {
        !self.running && self.all_present()
    }
}

fn not_enough(needed: u64, free: u64) -> String {
    format!(
        "Not enough space: needs {}, {} free",
        human_size(needed),
        human_size(free)
    )
}

fn percent(received: u64, total: u64) -> u64 {
    (received.saturating_mul(100))
        .checked_div(total)
        .unwrap_or(100)
        .min(100)
}

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

#[cfg(test)]
mod tests;
