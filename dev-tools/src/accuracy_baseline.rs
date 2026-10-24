//! `dev-tools accuracy-baseline`: rewrites `e2e-test/accuracy_baseline.json` from this worktree's last
//! journaled accuracy run of each sample size, with that run's commit; a size it never measured keeps its entry.

use crate::Outcome;
use serde_json::{json, Map, Value};

/// The baseline file, relative to the worktree.
const FILE: &str = "e2e-test/accuracy_baseline.json";

pub fn command(_args: &[String]) -> Outcome {
    let worktree =
        std::env::current_dir().map_err(|error| format!("current directory: {error}"))?;
    let file = worktree.join(FILE);
    let old = std::fs::read_to_string(&file).unwrap_or_else(|_| r#"{"wer":{}}"#.into());
    let journal = std::env::var_os("VOICEPUNCHER_STAGE_JOURNAL")
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    let (new, changes) = accept(&old, &journal, &worktree.to_string_lossy())?;
    if changes.is_empty() {
        return Err(format!(
            "no accuracy run of {} in the stage journal",
            worktree.display()
        ));
    }
    std::fs::write(&file, new).map_err(|error| format!("{}: {error}", file.display()))?;
    for change in changes {
        println!("✓ {change}");
    }
    Ok(0)
}

/// The baseline file's new text and a line per entry it replaced: the last rate per sample size of the
/// journal lines whose `worktree` is `worktree`.
pub fn accept(
    baseline: &str,
    journal: &str,
    worktree: &str,
) -> Result<(String, Vec<String>), String> {
    let mut file: Value =
        serde_json::from_str(baseline).map_err(|error| format!("{FILE}: {error}"))?;
    let entries = file
        .pointer_mut("/wer")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{FILE}: no wer object"))?;
    let mut changes = Vec::new();
    for (sample, entry) in last_rates(journal, worktree) {
        let old = entries.get(&sample).map_or_else(|| "none".into(), shown);
        changes.push(format!("{sample} recordings: {old} → {}", shown(&entry)));
        entries.insert(sample, entry);
    }
    let text = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    Ok((format!("{text}\n"), changes))
}

/// A baseline entry as its change line shows it: the rate and its commit.
fn shown(entry: &Value) -> String {
    format!("{} ({})", entry["mean"], entry["commit"])
}

/// The last rate per sample size of the journal lines whose `worktree` is `worktree`, as baseline entries.
fn last_rates(journal: &str, worktree: &str) -> Map<String, Value> {
    let mut last = Map::new();
    for run in journal
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        if run.get("worktree").and_then(Value::as_str) != Some(worktree) {
            continue;
        }
        let wer = run.get("wer").cloned().unwrap_or(Value::Null);
        let (Some(sample), Some(mean)) =
            (wer.get("sample").and_then(Value::as_u64), wer.get("mean"))
        else {
            continue;
        };
        let commit = run.get("commit").cloned().unwrap_or(Value::Null);
        last.insert(sample.to_string(), json!({"mean": mean, "commit": commit}));
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_accept_the_last_rate_of_this_worktree_and_keep_the_others() {
        // Given: a baseline of both sizes; this worktree measured 10 recordings twice, another one once
        let baseline = r#"{"wer":{"10":{"mean":0.031,"commit":"5ee5fbd"},"100":{"mean":0.047,"commit":"5ee5fbd"}}}"#;
        let journal = [
            r#"{"worktree":"/w","commit":"old0000","wer":{"sample":10,"mean":0.05}}"#,
            r#"{"worktree":"/w","commit":"abc1234","wer":{"sample":10,"mean":0.04}}"#,
            r#"{"worktree":"/other","commit":"fff9999","wer":{"sample":100,"mean":0.2}}"#,
            r#"{"worktree":"/w","commit":"def5678","wer":null}"#,
        ]
        .join("\n");
        // When
        let (text, changes) = accept(baseline, &journal, "/w").unwrap();
        // Then
        let file: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            file["wer"]["10"],
            json!({"mean": 0.04, "commit": "abc1234"})
        );
        assert_eq!(
            file["wer"]["100"],
            json!({"mean": 0.047, "commit": "5ee5fbd"})
        );
        assert_eq!(
            changes,
            ["10 recordings: 0.031 (\"5ee5fbd\") → 0.04 (\"abc1234\")"]
        );
    }
}
