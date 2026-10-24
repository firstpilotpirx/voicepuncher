//! The files a key covers — tracked and untracked but not ignored, as the working tree holds them — and the
//! `git` object hash of each.

use crate::process::git;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Stdio;

/// The repository's files, sorted: tracked and untracked but not ignored, regular files only.
pub fn listed(root: &Path) -> Result<Vec<String>, String> {
    let out = git(root)
        .args(["ls-files", "-z", "-c", "-o", "--exclude-standard"])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    let mut files: Vec<String> = out
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .filter(|name| root.join(name).is_file())
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// The object hash of each file, from one `git hash-object --stdin-paths`.
pub fn hashed(root: &Path, files: &[String]) -> Result<BTreeMap<String, String>, String> {
    if files.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut child = git(root)
        .args(["hash-object", "--stdin-paths"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git hash-object: {e}"))?;
    let mut input = child.stdin.take().ok_or("git hash-object: no stdin")?;
    let paths: String = files
        .iter()
        .flat_map(|file| [file.as_str(), "\n"])
        .collect();
    // Written beside the read, so a long list never fills both pipes
    let writer = std::thread::spawn(move || input.write_all(paths.as_bytes()));
    let mut text = String::new();
    child
        .stdout
        .take()
        .ok_or("git hash-object: no stdout")?
        .read_to_string(&mut text)
        .map_err(|e| format!("git hash-object: {e}"))?;
    let _ = writer.join();
    let _ = child.wait();
    let hashes: Vec<&str> = text.lines().collect();
    if hashes.len() != files.len() {
        return Err(format!(
            "git hash-object: {} hashes for {} files",
            hashes.len(),
            files.len()
        ));
    }
    Ok(files
        .iter()
        .cloned()
        .zip(hashes.into_iter().map(str::to_string))
        .collect())
}

/// A file's hash for a model stage: a Cargo file without the `version = "…"` lines of the workspace's own
/// packages, which no model stage reads.
pub fn unversioned(root: &Path, path: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
    let lock = path == "Cargo.lock" || path.ends_with("/Cargo.lock");
    let mut kept = String::new();
    let mut own = false;
    for line in text.split_inclusive('\n') {
        let bare = line.strip_suffix('\n').unwrap_or(line);
        if lock {
            own = own_package(bare).unwrap_or(own);
        }
        if (!lock || own) && is_version_line(bare) {
            continue;
        }
        kept.push_str(line);
    }
    Ok(hex(&Sha256::digest(kept.as_bytes())))
}

/// Whether a `name = …` line of a lock names one of the workspace's packages; nothing on another line.
fn own_package(line: &str) -> Option<bool> {
    let name = line.strip_prefix("name = ")?;
    let quoted = name
        .strip_prefix("\"voicepuncher")
        .and_then(|rest| rest.strip_suffix('"'));
    Some(quoted.is_some_and(|suffix| {
        suffix.is_empty()
            || suffix.strip_prefix('-').is_some_and(|tail| {
                !tail.is_empty() && tail.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            })
    }))
}

fn is_version_line(line: &str) -> bool {
    line.strip_prefix("version = \"")
        .and_then(|rest| rest.strip_suffix('"'))
        .is_some_and(|version| !version.contains('"'))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_the_workspace_packages_of_a_lock() {
        assert_eq!(own_package("name = \"voicepuncher\""), Some(true));
        assert_eq!(own_package("name = \"voicepuncher-copy-pool\""), Some(true));
        assert_eq!(own_package("name = \"voicepuncher-\""), Some(false));
        assert_eq!(own_package("name = \"serde\""), Some(false));
        assert_eq!(own_package("version = \"1.0.0\""), None);
    }

    #[test]
    fn should_drop_only_the_version_lines_of_the_own_packages() {
        let dir = tempfile::tempdir().unwrap();
        let lock = "[[package]]\nname = \"voicepuncher\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n";
        std::fs::write(dir.path().join("Cargo.lock"), lock).unwrap();
        let bumped = lock.replacen("1.0.0", "1.1.0", 1);
        let before = unversioned(dir.path(), "Cargo.lock").unwrap();
        std::fs::write(dir.path().join("Cargo.lock"), bumped).unwrap();
        assert_eq!(unversioned(dir.path(), "Cargo.lock").unwrap(), before);
        std::fs::write(
            dir.path().join("Cargo.lock"),
            lock.replace(
                "\"serde\"\nversion = \"1.0.0\"",
                "\"serde\"\nversion = \"1.0.1\"",
            ),
        )
        .unwrap();
        assert_ne!(unversioned(dir.path(), "Cargo.lock").unwrap(), before);
    }
}
