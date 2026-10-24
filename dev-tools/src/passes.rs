//! The pass records: one JSON file per stage (or test part) and key under `VOICEPUNCHER_STAGE_PASSES`
//! (`~/.cache/voicepuncher/stage-passes`), written by a stage that passed on unchanged inputs, read by every
//! worktree of the machine to reuse that stage.

use crate::clock;
use crate::context::cache_path;
use crate::process::{git_line, stdout};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The folder of the pass records.
pub fn folder() -> PathBuf {
    cache_path("VOICEPUNCHER_STAGE_PASSES", "stage-passes")
}

/// The pass record of a stage's key, when it has one.
pub fn found(passes: &Path, stage: &str, key: &str) -> Option<PathBuf> {
    let record = passes.join(stage).join(key);
    record.is_file().then_some(record)
}

/// A pass record's fields, or none when it cannot be read.
pub fn read(record: &Path) -> Value {
    std::fs::read_to_string(record)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}))
}

/// One line on a pass record: when, on which branch and commit it passed.
pub fn line(record: &Path) -> String {
    let fields = read(record);
    let text = |name: &str| fields.get(name).and_then(Value::as_str).map(str::to_string);
    let at = text("finished")
        .and_then(|finished| short_time(&finished))
        .unwrap_or_else(|| "?".into());
    let dirty = fields
        .get("dirty")
        .is_some_and(|dirty| dirty.as_bool().unwrap_or(false));
    format!(
        "passed {at} on {} at {}{}",
        text("branch").unwrap_or_else(|| "?".into()),
        text("commit").unwrap_or_else(|| "?".into()),
        if dirty { " (uncommitted edits)" } else { "" }
    )
}

/// `2026-10-23T10:00:00+02:00` as `2026-10-23 10:00`.
fn short_time(finished: &str) -> Option<String> {
    let (date, time) = finished.split_once('T')?;
    let digits = |text: &str| text.chars().all(|c| c.is_ascii_digit());
    let well_formed =
        date.len() == 10 && time.len() >= 5 && digits(&time[..2]) && digits(&time[3..5]);
    well_formed.then(|| format!("{date} {}", &time[..5]))
}

/// Writes the pass record of a stage under its key: commit, branch, whether the tree had uncommitted edits,
/// worktree, finish time and the seconds it took; by temp file and rename, so a reader never sees half a record.
pub fn record(
    passes: &Path,
    stage: &str,
    key: &str,
    seconds: f64,
    worktree: &Path,
) -> Result<(), String> {
    let folder = passes.join(stage);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let dirty = !stdout(crate::process::git(worktree).args(["status", "--porcelain"])).is_empty();
    let fields = json!({
        "stage": stage,
        "key": key,
        "commit": git_line(worktree, &["rev-parse", "--short", "HEAD"]),
        "branch": git_line(worktree, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "dirty": dirty,
        "worktree": worktree.to_string_lossy(),
        "finished": clock::iso(clock::now(), &clock::local_zone()),
        "seconds": clock::number(seconds),
    });
    let mut temp = tempfile::Builder::new()
        .prefix(".record.")
        .tempfile_in(&folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?;
    std::io::Write::write_all(&mut temp, format!("{fields}\n").as_bytes())
        .map_err(|e| e.to_string())?;
    temp.persist(folder.join(key)).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_describe_a_pass_record_in_one_line() {
        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("k");
        std::fs::write(&record, r#"{"branch":"feat/x","commit":"abc1234","dirty":true,"finished":"2026-10-23T10:00:00+02:00"}"#).unwrap();
        assert_eq!(
            line(&record),
            "passed 2026-10-23 10:00 on feat/x at abc1234 (uncommitted edits)"
        );
        assert_eq!(line(&dir.path().join("missing")), "passed ? on ? at ?");
    }

    #[test]
    fn should_write_a_record_that_reuse_finds() {
        let dir = tempfile::tempdir().unwrap();
        record(dir.path(), "test@app", "k1", 12.3, dir.path()).unwrap();
        let found = found(dir.path(), "test@app", "k1").unwrap();
        let fields = read(&found);
        assert_eq!(fields["stage"], "test@app");
        assert_eq!(fields["seconds"].to_string(), "12.3");
        assert_eq!(fields["commit"], Value::Null);
        assert!(super::found(dir.path(), "test@app", "k2").is_none());
    }

    #[test]
    fn should_write_the_fields_the_former_runner_wrote_in_its_order() {
        let dir = tempfile::tempdir().unwrap();
        record(dir.path(), "fmt", "k1", 4.0, dir.path()).unwrap();
        let text = std::fs::read_to_string(dir.path().join("fmt/k1")).unwrap();
        let names: Vec<&str> = text
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|name| text.contains(&format!("\"{name}\":")))
            .collect();
        assert_eq!(
            names,
            ["branch", "commit", "dirty", "finished", "key", "seconds", "stage", "worktree"]
        );
        assert!(text.ends_with("}\n") && text.lines().count() == 1, "{text}");
        assert!(text.contains("\"seconds\":4,"), "{text}");
    }
}
