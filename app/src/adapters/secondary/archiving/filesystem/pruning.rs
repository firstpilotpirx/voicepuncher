use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::PARTIAL_SUFFIX;

/// One entry folder found in the archive.
struct StoredEntry {
    name: String,
    path: PathBuf,
    size_bytes: u64,
}

/// Removes the oldest entry folders until the archive fits `max_size_bytes`; never `kept`.
pub(super) fn prune(dir: &Path, max_size_bytes: u64, kept: &str) -> Result<()> {
    let entries = stored_entries(dir)?;
    let mut total: u64 = entries.iter().map(|entry| entry.size_bytes).sum();
    for entry in entries.iter().filter(|entry| entry.name != kept) {
        if total <= max_size_bytes {
            break;
        }
        fs::remove_dir_all(&entry.path)
            .with_context(|| format!("failed to remove {}", entry.path.display()))?;
        total -= entry.size_bytes;
        log::info!(
            "archive: pruned {} ({} bytes) to fit {max_size_bytes} bytes",
            entry.name,
            entry.size_bytes
        );
    }
    Ok(())
}

/// Entry folders oldest first; `.partial` folders and plain files are not entries.
fn stored_entries(dir: &Path) -> Result<Vec<StoredEntry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if !item.file_type()?.is_dir() || name.ends_with(PARTIAL_SUFFIX) {
            continue;
        }
        let path = item.path();
        let size_bytes = folder_size(&path)?;
        entries.push(StoredEntry {
            name,
            path,
            size_bytes,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn folder_size(folder: &Path) -> Result<u64> {
    let mut size = 0;
    for item in fs::read_dir(folder)? {
        let item = item?;
        let metadata = item.metadata()?;
        size += if metadata.is_dir() {
            folder_size(&item.path())?
        } else {
            metadata.len()
        };
    }
    Ok(size)
}
