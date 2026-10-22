//! The copy-paste detector: `jscpd` over the Rust sources, its duplicated lines and clones.

use serde::Deserialize;
use std::process::Command;

/// What `jscpd` found.
#[derive(Debug, PartialEq, Eq)]
pub struct Duplicates {
    pub lines: u64,
    pub clones: u64,
}

#[derive(Deserialize)]
struct Report {
    statistics: Statistics,
}

#[derive(Deserialize)]
struct Statistics {
    total: Total,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Total {
    duplicated_lines: u64,
    clones: u64,
}

/// Runs `jscpd` over the members' Rust files, clones of at least `min_tokens` tokens.
pub fn run(members: &[String], min_tokens: u64) -> Result<Duplicates, String> {
    let out = std::env::temp_dir().join(format!("voicepuncher-jscpd-{}", std::process::id()));
    let status = Command::new("jscpd")
        .args([
            "--min-tokens",
            &min_tokens.to_string(),
            "--reporters",
            "json",
            "--format",
            "rust",
        ])
        .args(["--ignore", "**/target/**", "--silent", "--output"])
        .arg(&out)
        .args(members)
        .output()
        .map_err(|e| format!("jscpd: {e}"))?;
    let report = std::fs::read_to_string(out.join("jscpd-report.json")).map_err(|e| {
        format!(
            "jscpd wrote no report ({e}): {}",
            String::from_utf8_lossy(&status.stderr)
        )
    })?;
    let _ = std::fs::remove_dir_all(&out);
    parse(&report)
}

/// The totals of a `jscpd-report.json`.
pub fn parse(report: &str) -> Result<Duplicates, String> {
    let report: Report = serde_json::from_str(report).map_err(|e| format!("jscpd report: {e}"))?;
    let total = report.statistics.total;
    Ok(Duplicates { lines: total.duplicated_lines, clones: total.clones })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_the_totals_of_a_jscpd_report() {
        let report = r#"{"statistics":{"total":{"lines":70220,"clones":134,"duplicatedLines":1577,"percentage":2.25}},"duplicates":[]}"#;
        assert_eq!(parse(report), Ok(Duplicates { lines: 1577, clones: 134 }));
    }
}
