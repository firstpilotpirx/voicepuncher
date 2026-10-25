//! What a model download reports: its failures, its events and the state of the models.

use super::{fmt, Duration, ModelFile};

/// Why a file could not be downloaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadFailure {
    NoInternet,
    ChecksumMismatch,
    Server(String),
    Disk(String),
}

impl fmt::Display for DownloadFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInternet => write!(f, "no internet connection"),
            Self::ChecksumMismatch => write!(f, "checksum mismatch"),
            Self::Server(reason) => write!(f, "server: {reason}"),
            Self::Disk(reason) => write!(f, "disk: {reason}"),
        }
    }
}

/// What a download tells its observer, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadEvent {
    /// Nothing is fetched: `needed` bytes are missing, `free` are free.
    NotEnoughSpace {
        needed: u64,
        free: u64,
    },
    Started(&'static str),
    /// The bytes of an existing part are hashed before the rest is fetched.
    Verifying(&'static str),
    Progress {
        name: &'static str,
        received: u64,
        total: u64,
        bytes_per_s: u64,
    },
    /// A connection failure; the next try comes after `pause`.
    Retrying {
        name: &'static str,
        attempt: usize,
        pause: Duration,
    },
    /// The checksum did not match; the file is fetched once more from its start.
    MismatchRetrying(&'static str),
    Done(&'static str),
    Failed {
        name: &'static str,
        failure: DownloadFailure,
    },
    /// Stopped by Cancel; the part is kept.
    Cancelled(&'static str),
    AllPresent,
}

/// Told every event of a download.
pub trait DownloadObserver {
    fn event(&mut self, event: &DownloadEvent);
}

impl<F: FnMut(&DownloadEvent)> DownloadObserver for F {
    fn event(&mut self, event: &DownloadEvent) {
        self(event);
    }
}

/// One file of a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRow {
    pub file: ModelFile,
    pub present: bool,
    /// The bytes of its part.
    pub part: u64,
}

/// Which files are present, how many bytes are still missing and how many are free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelsReport {
    pub rows: Vec<ModelRow>,
    pub missing_bytes: u64,
    pub free_bytes: Option<u64>,
}

impl ModelsReport {
    pub fn all_present(&self) -> bool {
        self.rows.iter().all(|row| row.present)
    }

    pub fn present_count(&self) -> usize {
        self.rows.iter().filter(|row| row.present).count()
    }

    pub fn enough_space(&self) -> bool {
        self.free_bytes
            .is_none_or(|free| free >= self.missing_bytes)
    }
}

/// How a download ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadSummary {
    pub failed: Vec<(&'static str, DownloadFailure)>,
    pub cancelled: bool,
    pub all_present: bool,
}
