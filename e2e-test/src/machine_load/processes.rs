//! The busiest processes of the machine, from `ps`.

use std::process::Command;

/// Processes the report names, the busiest first.
const BUSIEST: usize = 5;

/// The `BUSIEST` rows of a `ps` listing sorted by CPU share, the header dropped.
pub fn busiest_processes(ps_output: &str) -> Vec<String> {
    ps_output
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(BUSIEST)
        .map(str::to_string)
        .collect()
}

pub(super) fn ps(args: &[&str]) -> String {
    Command::new("/bin/ps")
        .args(args)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}
