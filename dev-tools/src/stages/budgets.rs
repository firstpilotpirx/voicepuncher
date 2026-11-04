//! The time budget of a stage (`stage_budgets` in `.config/stages.toml`: `<stage> <seconds>` per line). A model
//! stage past its budget fails the run, naming the slowest scenarios of its output (`scenario time: <secs>
//! <name>`, as the E2E harness prints them) and the busiest processes beside it; an unlocked stage past its
//! budget only warns, as it runs beside other sessions' builds.

use crate::clock::tenths;
use crate::context::Context;
use std::collections::BTreeMap;
use std::process::Command;

/// How many of the slowest scenarios and busiest processes a failure names.
const NAMED: usize = 5;
const PROCESSES: usize = 3;

pub struct Budgets(BTreeMap<String, f64>);

impl Budgets {
    /// The budgets of the repository's tables; none without the table.
    pub fn read(context: &Context) -> Self {
        Self::parse(&context.table("stage_budgets").unwrap_or_default())
    }

    pub fn parse(text: &str) -> Self {
        let budgets = text
            .lines()
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                let stage = words.next()?;
                let seconds: f64 = words.next()?.parse().ok()?;
                Some((stage.to_owned(), seconds))
            })
            .collect();
        Self(budgets)
    }

    pub fn of(&self, stage: &str) -> Option<f64> {
        self.0.get(stage).copied()
    }

    /// What the report's stage row carries: ` budget=<seconds>`, or nothing.
    pub fn note(&self, stage: &str) -> String {
        self.of(stage)
            .map(|budget| format!(" budget={}", whole(budget)))
            .unwrap_or_default()
    }

    /// The line saying a stage ran past its budget, when it did.
    pub fn past(&self, stage: &str, seconds: f64) -> Option<String> {
        let budget = self.of(stage)?;
        (seconds > budget).then(|| {
            format!(
                "{stage}: {} s past its budget of {} s",
                tenths(seconds),
                whole(budget)
            )
        })
    }
}

/// A budget as a whole number of seconds, or with tenths when it has them.
fn whole(seconds: f64) -> String {
    if seconds.fract() == 0.0 {
        format!("{}", seconds as i64)
    } else {
        tenths(seconds)
    }
}

/// The slowest scenarios of a stage's output, slowest first: `(seconds, name)`.
pub fn slowest(output: &str) -> Vec<(f64, String)> {
    let mut times: Vec<(f64, String)> = output
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("scenario time: ")?;
            let (secs, name) = rest.split_once(' ')?;
            Some((secs.parse().ok()?, name.to_owned()))
        })
        .collect();
    times.sort_by(|a, b| b.0.total_cmp(&a.0));
    times.truncate(NAMED);
    times
}

/// The busiest processes now, `<%cpu> <command>`; nothing when `ps` is not there.
pub fn busiest() -> Vec<String> {
    let output = Command::new("ps")
        .args(["-Ao", "%cpu=,comm=", "-r"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .take(PROCESSES)
        .map(|line| {
            line.split_once(' ').map_or(line.clone(), |(cpu, command)| {
                format!("{cpu}% {}", command.trim())
            })
        })
        .collect()
}

/// The lines of a model stage's failure past its budget.
pub fn failure(past: &str, output: &str) -> Vec<String> {
    let mut lines = vec![format!("✗ {past}"), "slowest scenarios:".to_owned()];
    for (seconds, name) in slowest(output) {
        lines.push(format!("  {} s  {name}", tenths(seconds)));
    }
    lines.push("busiest processes:".to_owned());
    for process in busiest() {
        lines.push(format!("  {process}"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_a_budget_per_stage_and_none_for_an_unlisted_one() {
        let budgets = Budgets::parse("  e2e-smoke-voice 120\n test 60\n");
        assert_eq!(budgets.of("e2e-smoke-voice"), Some(120.0));
        assert_eq!(budgets.of("lint"), None);
        assert_eq!(budgets.note("test"), " budget=60");
        assert_eq!(budgets.note("lint"), "");
    }

    #[test]
    fn should_say_past_only_over_the_budget() {
        let budgets = Budgets::parse("test 60\n");
        assert_eq!(budgets.past("test", 60.0), None);
        assert_eq!(
            budgets.past("test", 72.04).as_deref(),
            Some("test: 72.0 s past its budget of 60 s")
        );
    }

    #[test]
    fn should_name_the_five_slowest_scenarios_slowest_first() {
        let output =
            "scenario time: 1.0 B\nnoise\nscenario time: 5.5 A slow one\nscenario time: 0.2 Q\n\
                      scenario time: 4.0 Another\nscenario time: 2.0 C\nscenario time: 3.0 D\n";
        let names: Vec<String> = slowest(output).into_iter().map(|(_, n)| n).collect();
        assert_eq!(names, ["A slow one", "Another", "D", "C", "B"]);
    }
}
