//! Model download: which model files are present, and fetching the missing ones one at a
//! time — resumed from their part, verified by checksum, placed, extracted — with retries.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::core::application::Clock;
use crate::core::domain::{ModelFile, ModelKind};

mod events;
use crate::core::domain::conversions::saturating_u64;
pub use events::*;

/// The pauses before the tries after a connection failure.
pub const RETRY_PAUSES: [Duration; 3] = [
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
];

/// The models folder: what is there, and the part files downloads write to.
pub trait ModelStore: Send {
    /// Present under its placed name with the catalog size, or named by its variable.
    fn present(&self, file: &ModelFile) -> bool;
    /// The bytes of `NAME.part`; 0 without one.
    fn part_len(&self, file: &ModelFile) -> u64;
    fn delete_part(&self, file: &ModelFile) -> Result<(), String>;
    /// The part, hashed so far, to append to.
    fn open_part(&self, file: &ModelFile) -> Result<Box<dyn PartSink>, String>;
    /// Moves the part to the file's name.
    fn place(&self, file: &ModelFile) -> Result<(), String>;
    /// Extracts the placed archive into the folder and deletes it.
    fn extract(&self, file: &ModelFile) -> Result<(), String>;
}

/// A part file being written: appended to, hashed as it grows.
pub trait PartSink: Send {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
    /// Back to no bytes: the server sends the whole file.
    fn restart(&mut self) -> Result<(), String>;
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Of every byte written, as lowercase hex.
    fn sha256_hex(&mut self) -> String;
}

/// Why a fetch ended early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// No connection, a cut or a timeout: worth another try.
    Connection(String),
    /// The server refused the file.
    Server(String),
    /// The part cannot be written.
    Disk(String),
    Cancelled,
}

/// Streams a file from its address into a part, from the part's length on.
pub trait ModelFetcher: Send {
    /// `received` is told the part's length after every chunk.
    fn fetch(
        &self,
        url: &str,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError>;
}

/// The free space of the models folder; none when it cannot be told.
pub trait DiskSpace: Send {
    fn free_bytes(&self) -> Option<u64>;
}

/// Checks and downloads one set of model files.
pub struct ModelDownload {
    files: Vec<ModelFile>,
    store: Box<dyn ModelStore>,
    fetcher: Box<dyn ModelFetcher>,
    disk: Box<dyn DiskSpace>,
    clock: Arc<dyn Clock>,
    /// Replaces each address with `<mirror>/<name>`; none for the pinned addresses.
    mirror: Option<String>,
}

/// How one try of a file ended.
enum Try {
    Placed,
    Mismatch,
    Failed(DownloadFailure),
    Connection,
    Cancelled,
}

impl Try {
    /// How a try whose fetch ended early with `error` ended.
    fn of_fetch(file: &ModelFile, error: FetchError) -> Self {
        match error {
            FetchError::Cancelled => Self::Cancelled,
            FetchError::Connection(reason) => {
                log::warn!("models: {} — connection failed: {reason}", file.name);
                Self::Connection
            }
            FetchError::Server(reason) => Self::Failed(DownloadFailure::Server(reason)),
            FetchError::Disk(reason) => Self::Failed(DownloadFailure::Disk(reason)),
        }
    }
}

impl ModelDownload {
    pub fn new(
        files: Vec<ModelFile>,
        store: Box<dyn ModelStore>,
        fetcher: Box<dyn ModelFetcher>,
        disk: Box<dyn DiskSpace>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { files, store, fetcher, disk, clock, mirror: None }
    }

    /// Fetches every file from `<mirror>/<name>` instead of its pinned address.
    pub fn with_mirror(mut self, mirror: Option<String>) -> Self {
        self.mirror = mirror.filter(|m| !m.trim().is_empty());
        self
    }

    pub fn files(&self) -> &[ModelFile] {
        &self.files
    }

    pub fn check(&self) -> ModelsReport {
        let rows: Vec<ModelRow> = self
            .files
            .iter()
            .map(|file| ModelRow {
                file: *file,
                present: self.store.present(file),
                part: self.store.part_len(file),
            })
            .collect();
        let missing_bytes = rows
            .iter()
            .filter(|row| !row.present)
            .map(|row| row.file.size_bytes.saturating_sub(row.part))
            .sum();
        ModelsReport { rows, missing_bytes, free_bytes: self.disk.free_bytes() }
    }

    /// Fetches every missing file in catalog order; returns once each is placed, failed or
    /// the download was cancelled.
    pub fn download_missing(
        &self,
        observer: &mut dyn DownloadObserver,
        cancel: &AtomicBool,
    ) -> DownloadSummary {
        let report = self.check();
        let mut summary =
            DownloadSummary { failed: Vec::new(), cancelled: false, all_present: false };
        if !report.enough_space() {
            observer.event(&DownloadEvent::NotEnoughSpace {
                needed: report.missing_bytes,
                free: report.free_bytes.unwrap_or_default(),
            });
            return summary;
        }
        for row in report.rows.iter().filter(|row| !row.present) {
            if cancel.load(Ordering::SeqCst) {
                summary.cancelled = true;
                break;
            }
            match self.download(&row.file, observer, cancel) {
                Ok(()) => observer.event(&DownloadEvent::Done(row.file.name)),
                Err(None) => {
                    observer.event(&DownloadEvent::Cancelled(row.file.name));
                    summary.cancelled = true;
                    break;
                }
                Err(Some(failure)) => {
                    observer.event(&DownloadEvent::Failed {
                        name: row.file.name,
                        failure: failure.clone(),
                    });
                    summary.failed.push((row.file.name, failure));
                }
            }
        }
        summary.all_present = self.check().all_present();
        if summary.all_present {
            observer.event(&DownloadEvent::AllPresent);
        }
        summary
    }

    /// Places `file`; `Err(None)` when cancelled.
    fn download(
        &self,
        file: &ModelFile,
        observer: &mut dyn DownloadObserver,
        cancel: &AtomicBool,
    ) -> Result<(), Option<DownloadFailure>> {
        observer.event(&DownloadEvent::Started(file.name));
        self.drop_overlong_part(file)?;
        let mut mismatches = 0;
        let mut connection_failures = 0;
        loop {
            match self.try_once(file, observer, cancel) {
                Try::Placed => return Ok(()),
                Try::Cancelled => return Err(None),
                Try::Failed(failure) => return Err(Some(failure)),
                Try::Mismatch => {
                    mismatches += 1;
                    self.drop_mismatched(file, mismatches, observer)?;
                }
                Try::Connection => {
                    connection_failures += 1;
                    self.pause_to_retry(file, connection_failures, observer, cancel)?;
                }
            }
        }
    }

    /// Deletes a part longer than the file it is part of: it cannot be resumed.
    fn drop_overlong_part(&self, file: &ModelFile) -> Result<(), Option<DownloadFailure>> {
        if self.store.part_len(file) > file.size_bytes {
            self.store.delete_part(file).map_err(disk)?;
        }
        Ok(())
    }

    /// Deletes the part that failed its checksum; `Err` after the second mismatch.
    fn drop_mismatched(
        &self,
        file: &ModelFile,
        mismatches: usize,
        observer: &mut dyn DownloadObserver,
    ) -> Result<(), Option<DownloadFailure>> {
        self.store.delete_part(file).map_err(disk)?;
        if mismatches > 1 {
            return Err(Some(DownloadFailure::ChecksumMismatch));
        }
        observer.event(&DownloadEvent::MismatchRetrying(file.name));
        Ok(())
    }

    /// Pauses before the try after connection failure number `failures`; `Err` once the
    /// pauses are used up or the download was cancelled meanwhile.
    fn pause_to_retry(
        &self,
        file: &ModelFile,
        failures: usize,
        observer: &mut dyn DownloadObserver,
        cancel: &AtomicBool,
    ) -> Result<(), Option<DownloadFailure>> {
        let Some(&pause) = RETRY_PAUSES.get(failures - 1) else {
            return Err(Some(DownloadFailure::NoInternet));
        };
        observer.event(&DownloadEvent::Retrying { name: file.name, attempt: failures, pause });
        self.clock.sleep(pause);
        if cancel.load(Ordering::SeqCst) {
            return Err(None);
        }
        Ok(())
    }

    fn try_once(
        &self,
        file: &ModelFile,
        observer: &mut dyn DownloadObserver,
        cancel: &AtomicBool,
    ) -> Try {
        if self.store.part_len(file) > 0 {
            observer.event(&DownloadEvent::Verifying(file.name));
        }
        let mut sink = match self.store.open_part(file) {
            Ok(sink) => sink,
            Err(e) => return Try::Failed(DownloadFailure::Disk(e)),
        };
        if let Err(error) = self.fetch_into(file, sink.as_mut(), observer, cancel) {
            return Try::of_fetch(file, error);
        }
        observer.event(&DownloadEvent::Verifying(file.name));
        let hash = sink.sha256_hex();
        drop(sink);
        if hash != file.sha256 {
            log::warn!(
                "models: {} — sha256 {hash}, expected {}",
                file.name,
                file.sha256
            );
            return Try::Mismatch;
        }
        self.put_in_place(file)
    }

    /// Streams `file` into `sink` from its length on, telling `observer` the progress.
    fn fetch_into(
        &self,
        file: &ModelFile,
        sink: &mut dyn PartSink,
        observer: &mut dyn DownloadObserver,
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let started = self.clock.now();
        let from = sink.len();
        let clock = Arc::clone(&self.clock);
        let mut tell = |received: u64| {
            observer.event(&DownloadEvent::Progress {
                name: file.name,
                received,
                total: file.size_bytes,
                bytes_per_s: rate(received.saturating_sub(from), started, clock.now()),
            });
        };
        self.fetcher
            .fetch(&self.address(file), sink, &mut tell, cancel)
    }

    /// Moves the verified part to its name and extracts an archive.
    fn put_in_place(&self, file: &ModelFile) -> Try {
        if let Err(e) = self.store.place(file) {
            return Try::Failed(DownloadFailure::Disk(e));
        }
        if let ModelKind::TarBz2 { .. } = file.kind {
            if let Err(e) = self.store.extract(file) {
                return Try::Failed(DownloadFailure::Disk(e));
            }
        }
        Try::Placed
    }

    fn address(&self, file: &ModelFile) -> String {
        match &self.mirror {
            Some(mirror) => format!("{}/{}", mirror.trim_end_matches('/'), file.name),
            None => file.url.to_string(),
        }
    }
}

fn disk(reason: String) -> DownloadFailure {
    DownloadFailure::Disk(reason)
}

fn rate(bytes: u64, started: Instant, now: Instant) -> u64 {
    let seconds = now.saturating_duration_since(started).as_secs_f64();
    if seconds <= 0.0 {
        return 0;
    }
    saturating_u64(bytes as f64 / seconds)
}

#[cfg(test)]
mod tests;
