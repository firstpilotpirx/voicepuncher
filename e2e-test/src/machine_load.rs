//! What ran beside a timed scenario: the busiest processes on the machine and every live
//! holder of a machine-wide test lock, for the failure of a start cue over its bound.

mod holders;
mod processes;
#[cfg(test)]
mod tests;

pub use holders::{live_holders, lock_dirs, LockHolder};
pub use processes::busiest_processes;
use processes::ps;

/// The machine's load right now, as lines for a failure report: the busiest processes and
/// the live lock holders, or a line saying none holds a lock.
pub fn report() -> String {
    let mut lines = vec![
        "machine load when the failure was reported:".to_string(),
        "  busiest processes (%cpu pid command):".to_string(),
    ];
    lines.extend(
        busiest_processes(&ps(&["-Ao", "pcpu,pid,comm", "-r"]))
            .into_iter()
            .map(|l| format!("    {l}")),
    );
    let holders = live_holders(&lock_dirs(), |pid| {
        !ps(&["-p", &pid.to_string(), "-o", "pid="])
            .trim()
            .is_empty()
    });
    if holders.is_empty() {
        lines.push("  no run holds a machine-wide test lock".to_string());
    } else {
        lines.push("  lock holders:".to_string());
        lines.extend(holders.into_iter().map(|h| {
            format!(
                "    {} ({}, pid {}): {} — {}",
                h.lock, h.mode, h.pid, h.command, h.dir
            )
        }));
    }
    lines.join("\n")
}
