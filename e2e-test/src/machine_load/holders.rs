//! The live holders of the machine-wide test locks, from the kit's holder files.

use std::fs;
use std::path::{Path, PathBuf};

/// One run holding a lock, read from the kit's holder file (`key=value` lines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockHolder {
    pub lock: String,
    pub pid: u32,
    pub dir: String,
    pub command: String,
    pub mode: String,
}

/// The lock directories of this machine: the machine lock's and every class lock beside it
/// (`<machine lock dir>-*`).
pub fn lock_dirs() -> Vec<PathBuf> {
    let base = std::env::var("MACHINE_LOCK_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME").map(|home| Path::new(&home).join(".cache/machine-lock"))
        });
    let Ok(base) = base else { return Vec::new() };
    let stem = base
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let mut dirs: Vec<PathBuf> = base
        .parent()
        .and_then(|parent| fs::read_dir(parent).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&stem))
        })
        .collect();
    dirs.sort();
    dirs
}

/// Every holder of the locks under `dirs` whose process `alive` confirms.
pub fn live_holders(dirs: &[PathBuf], alive: impl Fn(u32) -> bool) -> Vec<LockHolder> {
    let mut holders: Vec<LockHolder> = dirs
        .iter()
        .flat_map(|dir| {
            let lock = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            fs::read_dir(dir.join("holders"))
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| fs::read_to_string(entry.path()).ok())
                .filter_map(move |content| parse_holder(&lock, &content))
        })
        .filter(|holder| alive(holder.pid))
        .collect();
    holders.sort_by(|a, b| (&a.lock, a.pid).cmp(&(&b.lock, b.pid)));
    holders
}

fn parse_holder(lock: &str, content: &str) -> Option<LockHolder> {
    let field = |key: &str| {
        content
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|rest| rest.strip_prefix('='))
            })
            .unwrap_or_default()
            .to_string()
    };
    Some(LockHolder {
        lock: lock.to_string(),
        pid: field("pid").parse().ok()?,
        dir: field("dir"),
        command: field("cmd"),
        mode: field("mode"),
    })
}
