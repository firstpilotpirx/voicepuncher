//! A folder of entry folders: each entry written whole under a name that sorts by time, the
//! oldest removed once the folder passes its size limit. The dictation archive and the
//! command history keep their entries this way.

mod pruning;
mod utc_time;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub use pruning::prune;
pub use utc_time::UtcTime;

/// Marks a folder still being written; readers and pruning skip it.
pub const PARTIAL_SUFFIX: &str = ".partial";
/// The suffix number of the second entry that starts in the same millisecond.
const FIRST_DUPLICATE_NUMBER: u32 = 2;

/// Writes an entry folder named `base` (or `base-2` … when taken) inside `dir` with `write`,
/// which fills the folder it is given; the folder appears under its name only once written.
pub fn write_entry(
    dir: &Path,
    base: &str,
    write: impl FnOnce(&Path) -> Result<()>,
) -> Result<String> {
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let name = free_name(dir, base);
    let partial = dir.join(format!("{name}{PARTIAL_SUFFIX}"));
    fs::create_dir_all(&partial)
        .with_context(|| format!("failed to create {}", partial.display()))?;
    write(&partial)?;
    let folder: PathBuf = dir.join(&name);
    fs::rename(&partial, &folder)
        .with_context(|| format!("failed to move {} into place", folder.display()))?;
    Ok(name)
}

/// `base`, or `base-2`, `base-3` … when an entry of that name exists.
fn free_name(dir: &Path, base: &str) -> String {
    let taken = |name: &str| dir.join(name).exists();
    if !taken(base) {
        return base.to_owned();
    }
    (FIRST_DUPLICATE_NUMBER..=u32::MAX)
        .map(|number| format!("{base}-{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_owned())
}
