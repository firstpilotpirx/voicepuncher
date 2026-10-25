//! Reporting the files a lookup found or missed.

use super::{Missing, Path, PathBuf};

pub(super) fn existing(path: PathBuf, var: &str) -> Result<PathBuf, Missing> {
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("{} not found (set by {var})", path.display()))
    }
}

pub(super) fn shown(found: &Result<PathBuf, Missing>) -> String {
    found
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

pub(super) fn first_problem<const N: usize>(
    found: [&Result<PathBuf, Missing>; N],
) -> Option<String> {
    found
        .into_iter()
        .find_map(|item| item.as_ref().err().cloned())
}

/// The most recently changed of `found`; of equally recent ones, the first by name.
pub(super) fn newest(found: Vec<PathBuf>) -> Option<PathBuf> {
    found.into_iter().max_by_key(|path| {
        let changed = path.metadata().and_then(|meta| meta.modified()).ok();
        (changed, std::cmp::Reverse(path.clone()))
    })
}

pub(super) fn file_name_starts(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(prefix))
}
