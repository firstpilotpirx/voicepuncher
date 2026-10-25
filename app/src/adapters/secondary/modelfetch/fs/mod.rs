//! The models folder on disk.
//!
//! Which catalog files are present, the `NAME.part` files a download appends to and hashes, their
//! placing by rename, the archive's extraction with `/usr/bin/tar`, and the folder's free space.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::core::application::{DiskSpace, ModelStore, PartSink};
use crate::core::domain::{ModelFile, ModelKind};

const TAR: &str = "/usr/bin/tar";
const HASH_CHUNK: usize = 1 << 20;

/// The models folder, with the older folders the services also look in.
pub struct FsModelStore {
    folder: PathBuf,
    /// `~/.voicepuncher/models` and `~/.voicepuncher`, as the service catalog searches them.
    fallbacks: Vec<PathBuf>,
    env: HashMap<String, String>,
}

impl FsModelStore {
    pub fn new(folder: PathBuf, home: &Path, env: HashMap<String, String>) -> Self {
        let voicepuncher = home.join(".voicepuncher");
        let fallbacks = [voicepuncher.join("models"), voicepuncher]
            .into_iter()
            .filter(|dir| *dir != folder)
            .collect();
        Self { folder, fallbacks, env }
    }

    /// The models folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    fn part_path(&self, file: &ModelFile) -> PathBuf {
        self.folder.join(format!("{}.part", file.name))
    }

    fn named_by_variable(&self, file: &ModelFile) -> bool {
        self.env
            .get(file.variable)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .is_some_and(|value| Path::new(value).exists())
    }

    fn present_in(dir: &Path, file: &ModelFile) -> bool {
        match file.kind {
            ModelKind::File => dir
                .join(file.name)
                .metadata()
                .is_ok_and(|meta| meta.is_file() && meta.len() == file.size_bytes),
            ModelKind::TarBz2 { folder } => dir.join(folder).is_dir(),
        }
    }
}

impl ModelStore for FsModelStore {
    fn present(&self, file: &ModelFile) -> bool {
        self.named_by_variable(file)
            || std::iter::once(&self.folder)
                .chain(&self.fallbacks)
                .any(|dir| Self::present_in(dir, file))
    }

    fn part_len(&self, file: &ModelFile) -> u64 {
        self.part_path(file).metadata().map_or(0, |meta| meta.len())
    }

    fn delete_part(&self, file: &ModelFile) -> Result<(), String> {
        match std::fs::remove_file(self.part_path(file)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("{}: {e}", self.part_path(file).display())),
        }
    }

    fn open_part(&self, file: &ModelFile) -> Result<Box<dyn PartSink>, String> {
        let path = self.part_path(file);
        let shown = |e: std::io::Error| format!("{}: {e}", path.display());
        std::fs::create_dir_all(&self.folder).map_err(shown)?;
        let mut part = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(shown)?;
        let mut hasher = Sha256::new();
        let mut len = 0;
        let mut buffer = vec![0; HASH_CHUNK];
        part.seek(SeekFrom::Start(0)).map_err(shown)?;
        loop {
            let read = part.read(&mut buffer).map_err(shown)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            len += read as u64;
        }
        Ok(Box::new(FsPart { path, part, hasher, len }))
    }

    fn place(&self, file: &ModelFile) -> Result<(), String> {
        let target = self.folder.join(file.name);
        std::fs::rename(self.part_path(file), &target)
            .map_err(|e| format!("{}: {e}", target.display()))
    }

    fn extract(&self, file: &ModelFile) -> Result<(), String> {
        let ModelKind::TarBz2 { folder } = file.kind else {
            return Ok(());
        };
        let archive = self.folder.join(file.name);
        let output = Command::new(TAR)
            .arg("-xjf")
            .arg(&archive)
            .arg("-C")
            .arg(&self.folder)
            .output()
            .map_err(|e| format!("{TAR}: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "{TAR} could not extract {}: {}",
                archive.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        if !self.folder.join(folder).is_dir() {
            return Err(format!("{} holds no folder {folder}", archive.display()));
        }
        std::fs::remove_file(&archive).map_err(|e| format!("{}: {e}", archive.display()))
    }
}

/// A part file open for appending, with the hash of every byte in it.
struct FsPart {
    path: PathBuf,
    part: File,
    hasher: Sha256,
    len: u64,
}

impl PartSink for FsPart {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.part
            .write_all(bytes)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.hasher.update(bytes);
        self.len += bytes.len() as u64;
        Ok(())
    }

    fn restart(&mut self) -> Result<(), String> {
        self.part
            .set_len(0)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.hasher = Sha256::new();
        self.len = 0;
        Ok(())
    }

    fn len(&self) -> u64 {
        self.len
    }

    fn sha256_hex(&mut self) -> String {
        if let Err(e) = self.part.sync_all() {
            log::warn!("models: {} not synced: {e}", self.path.display());
        }
        self.hasher
            .clone()
            .finalize()
            .iter()
            .fold(String::new(), |mut text, byte| {
                let _ = write!(text, "{byte:02x}");
                text
            })
    }
}

/// The free space of the volume the models folder is on, or will be on.
pub struct FsDiskSpace {
    folder: PathBuf,
}

impl FsDiskSpace {
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }
}

impl DiskSpace for FsDiskSpace {
    fn free_bytes(&self) -> Option<u64> {
        let existing = self.folder.ancestors().find(|dir| dir.exists())?;
        let path = std::ffi::CString::new(existing.as_os_str().as_encoded_bytes()).ok()?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `path` is a valid C string and `stats` is written by `statvfs` on success.
        let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
        if result != 0 {
            return None;
        }
        // SAFETY: `statvfs` returned 0, so it filled `stats`.
        let stats = unsafe { stats.assume_init() };
        Some(u64::from(stats.f_bavail).saturating_mul(stats.f_frsize))
    }
}
