//! How far a timing bound stretches for the load other programs put on the machine: the
//! specs bound a cue on an idle machine, and a build or a VM beside the run takes the cores
//! the recognizer needs, so a late cue then says nothing about the recognizer.

use std::process::Command;
use std::time::Duration;

/// The machine's load beside a timed scenario: the runnable threads the kernel averages over
/// the last minute, the cores, and the threads the scenario keeps busy itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadAllowance {
    pub load: f32,
    pub cores: usize,
    pub own_threads: usize,
}

impl LoadAllowance {
    /// The machine's load right now; `own_threads` are the threads the scenario spins itself,
    /// left out of the load of other programs.
    pub fn now(own_threads: usize) -> Self {
        let load = Command::new("/usr/sbin/sysctl")
            .args(["-n", "vm.loadavg"])
            .output()
            .ok()
            .and_then(|out| parse_loadavg(&String::from_utf8_lossy(&out.stdout)))
            .unwrap_or(0.0);
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        Self { load, cores, own_threads }
    }

    /// How many times longer a step takes than on an idle machine.
    pub fn factor(&self) -> f32 {
        let others = (self.load - self.own_threads as f32).max(0.0);
        (others / self.cores.max(1) as f32).max(1.0)
    }

    /// `bound` stretched by the factor.
    pub fn stretch(&self, bound: Duration) -> Duration {
        let factor = self.factor();
        if factor <= 1.0 {
            return bound;
        }
        bound.mul_f64(f64::from(factor))
    }

    /// The bound and its stretch, for a failure or a pass that needed it.
    pub fn describe(&self, bound: Duration) -> String {
        format!(
            "bound {:.2} s × {:.1} = {:.2} s (load {:.1} on {} cores, {} of its threads the scenario's own)",
            bound.as_secs_f32(),
            self.factor(),
            self.stretch(bound).as_secs_f32(),
            self.load,
            self.cores,
            self.own_threads
        )
    }
}

/// The one-minute load of `sysctl -n vm.loadavg`'s output, `{ 4.12 3.98 3.50 }`.
pub fn parse_loadavg(output: &str) -> Option<f32> {
    output
        .split_whitespace()
        .find(|word| *word != "{")
        .and_then(|word| word.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowance(load: f32, own_threads: usize) -> LoadAllowance {
        LoadAllowance { load, cores: 10, own_threads }
    }

    #[test]
    fn should_keep_the_bound_on_an_idle_machine() {
        assert_eq!(
            allowance(3.0, 0).stretch(Duration::from_millis(500)),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn should_stretch_the_bound_by_the_load_per_core() {
        let stretched = allowance(80.0, 0).stretch(Duration::from_millis(500));
        assert_eq!(stretched, Duration::from_millis(4000));
    }

    #[test]
    fn should_leave_the_scenarios_own_busy_threads_out_of_the_load() {
        let stretched = allowance(12.0, 10).stretch(Duration::from_millis(1200));
        assert_eq!(stretched, Duration::from_millis(1200));
    }

    #[test]
    fn should_read_the_one_minute_load_of_sysctl() {
        assert_eq!(parse_loadavg("{ 80.72 124.32 97.73 }\n"), Some(80.72));
        assert_eq!(parse_loadavg("garbage"), None);
    }
}
