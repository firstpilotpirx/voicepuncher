//! `dev-tools stages <stage>…`: runs the named stages (the `justfile`'s private recipes without their `_`) in
//! phases. The stages before the first model stage (`e2e*` or `it-real`, which takes the screen
//! and the microphone) run one by one under the quiet lock in the shared mode, which gives each its time limit
//! and keeps it out of another run's model stages; fmt, lint, build, bundle and build-e2e run at the utility quality of service
//! clamp. The rest runs in `model-stages`, in one phase under the e2e lock (or the caller's machine lock), on the
//! test ports, so the model stages have the machine alone.
//!
//! A stage that passed before on the same inputs (its key has a pass record) is reused (↺) and does not run;
//! build and build-e2e run whenever a model stage runs, as they fill this worktree's `target/`; with every model
//! stage reused no lock is requested. A rerun of failed tests (`VOICEPUNCHER_RERUN_FILTER`, `…_SCENARIOS`)
//! neither reuses nor records. The run ends with one table (each stage's time, the lock wait and hold), the pass
//! records of the stages that passed while their key held, and one line in the stage journal; every stage's
//! output is also copied to a log of the run, where the journal finds the failed tests.

pub mod child;
mod environment;
pub mod it_real;
pub mod model;
mod table;
pub mod test;
#[cfg(test)]
mod tests;

use crate::clock::{clock_time, now, tenths};
use crate::context::{var, Context};
use crate::journal::{self, report::Report};
use crate::locks::{command_on_test_ports, Lock};
use crate::process::{code, Interrupt};
use crate::{keys, passes, Outcome};
use environment::{export_run, reuses_passes};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The stages that run at the utility quality of service clamp, so they yield the CPU to another run's model stages.
const CLAMPED: [&str; 5] = ["fmt", "lint", "build", "bundle", "build-e2e"];

/// Whether a stage is a model stage: it and every stage after it run under the e2e lock.
pub fn is_model(stage: &str) -> bool {
    stage.starts_with("e2e") || stage == "it-real"
}

/// Appends a line to the run's report.
pub fn note(report: Option<&Path>, line: &str) {
    let Some(report) = report else { return };
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(report);
    // A report that cannot be written loses its rows, not the run
    if let Ok(mut file) = file {
        let _ = writeln!(file, "{line}");
    }
}

pub fn command(args: &[String]) -> Outcome {
    run(args, Lock::from_env())
}

/// Runs the stages under the lock; refuses before the first stage when the lock's wrapper is missing.
fn run(args: &[String], lock: Lock) -> Outcome {
    if args.is_empty() {
        return Err("stages: name the stages to run".into());
    }
    lock.require()?;
    let mut runner = Runner::start(args.to_vec(), lock)?;
    let status = runner.stages();
    Ok(code(runner.finish(status)))
}

struct Runner {
    context: Context,
    lock: Lock,
    stages: Vec<String>,
    report: tempfile::NamedTempFile,
    log: tempfile::NamedTempFile,
    keys: Vec<(String, String)>,
    reuse: BTreeMap<String, PathBuf>,
    caller: bool,
    rerun: bool,
    interrupt: Interrupt,
    started: f64,
    phase_class: Option<&'static str>,
}

impl Runner {
    fn start(stages: Vec<String>, lock: Lock) -> Result<Self, String> {
        let started = now();
        let temp = || tempfile::NamedTempFile::new().map_err(|e| e.to_string());
        let (report, log) = (temp()?, temp()?);
        let context = Context::from_env();
        let caller = export_run(&context, &lock, report.path());
        let keys = keys::of_this_run(&context, &stages).unwrap_or_else(|error| {
            eprintln!("✗ the stage keys could not be computed ({error}): every stage runs and no pass is recorded");
            Vec::new()
        });
        let rerun = var("VOICEPUNCHER_RERUN_FILTER").is_some()
            || var("VOICEPUNCHER_RERUN_SCENARIOS").is_some();
        let reuse = if reuses_passes(rerun) {
            passed(&keys)
        } else {
            BTreeMap::new()
        };
        let interrupt = Interrupt::watch();
        Ok(Self {
            context,
            lock,
            stages,
            report,
            log,
            keys,
            reuse,
            caller,
            rerun,
            interrupt,
            started,
            phase_class: None,
        })
    }

    fn note(&self, line: &str) {
        note(Some(self.report.path()), line);
    }

    /// Runs every stage; the run's exit code.
    fn stages(&mut self) -> i32 {
        let first_model = self
            .stages
            .iter()
            .position(|stage| is_model(stage))
            .unwrap_or(self.stages.len());
        let (unlocked, model) = self.stages.split_at(first_model);
        let (unlocked, model) = (unlocked.to_vec(), model.to_vec());
        let to_run: Vec<String> = model
            .iter()
            .filter(|stage| !self.reuse.contains_key(*stage))
            .cloned()
            .collect();
        if !to_run.is_empty() {
            self.reuse.remove("build");
            self.reuse.remove("build-e2e");
        }
        for stage in &unlocked {
            if self.reuse_stage(stage) {
                continue;
            }
            let status = self.unlocked_stage(stage);
            if status != 0 {
                return status;
            }
            if self.interrupt.raised() {
                return 130;
            }
        }
        for stage in &model {
            self.reuse_stage(stage);
        }
        if to_run.is_empty() {
            return 0;
        }
        self.phase(&to_run)
    }

    /// Reuses a stage that has a pass record; whether it did.
    fn reuse_stage(&self, stage: &str) -> bool {
        let Some(record) = self.reuse.get(stage) else {
            return false;
        };
        println!(
            "↺ {} {stage} reused: {}",
            clock_time(),
            passes::line(record)
        );
        self.note(&format!("reused {stage} {}", record.display()));
        true
    }

    /// `just --justfile <justfile> <recipe>`.
    fn recipe(&self, recipe: &str) -> Vec<OsString> {
        vec![
            self.context.just.clone().into(),
            "--justfile".into(),
            self.context.justfile.clone().into(),
            recipe.into(),
        ]
    }

    fn unlocked_stage(&self, stage: &str) -> i32 {
        let begun = now();
        let mut line: Vec<OsString> = Vec::new();
        if CLAMPED.contains(&stage) {
            line.extend(["taskpolicy", "-c", "utility"].map(OsString::from));
        }
        line.extend(self.lock.quiet(&self.recipe(&format!("_{stage}"))));
        let status = child::run(
            &mut command_on_test_ports(&line),
            &self.interrupt,
            Some(self.log.path()),
        );
        let mark = if status == 0 && !self.interrupt.raised() {
            "✓"
        } else {
            "✗"
        };
        self.note(&format!("stage {mark} {stage} {}", tenths(now() - begun)));
        status
    }

    /// Runs the model stages in `model-stages` under the e2e lock, or the caller's machine lock as it is.
    fn phase(&mut self, stages: &[String]) -> i32 {
        let class = if self.caller { "machine" } else { "e2e" };
        let exe = std::env::current_exe()
            .map_or_else(|_| OsString::from("dev-tools"), PathBuf::into_os_string);
        let mut line: Vec<OsString> = vec![exe, "model-stages".into()];
        line.extend(stages.iter().map(OsString::from));
        if self.caller {
            println!(
                "● {} the caller holds the machine lock: {}",
                clock_time(),
                stages.join(" ")
            );
        } else {
            println!(
                "→ {} waiting for the {class} lock: {}",
                clock_time(),
                stages.join(" ")
            );
            line = self.lock.e2e(&line);
        }
        self.phase_class = Some(class);
        self.note(&format!("requested {class} {}", tenths(now())));
        let mut command = command_on_test_ports(&line);
        command.env("VOICEPUNCHER_LOCK_CLASS", class);
        let status = child::run(&mut command, &self.interrupt, Some(self.log.path()));
        self.note(&format!("returned {class} {}", tenths(now())));
        self.phase_class = None;
        match status {
            0 if self.interrupt.raised() => 130,
            status => status,
        }
    }

    /// Prints the table, records the passes and journals the run; the run's exit code.
    fn finish(&mut self, status: i32) -> i32 {
        let status = if self.interrupt.raised() && status == 0 {
            130
        } else {
            status
        };
        // A phase the run left without its return mark returns now
        if let Some(class) = self.phase_class.take() {
            self.note(&format!("returned {class} {}", tenths(now())));
        }
        let text = std::fs::read_to_string(self.report.path()).unwrap_or_default();
        println!("\nstages:");
        for line in table::lines(&text, self.caller, now()) {
            println!("{line}");
        }
        let ended = now();
        println!("{}", table::total(ended - self.started));
        let report = Report::parse(&text);
        if let Err(error) = self.record_passes(&report) {
            eprintln!("✗ the passes of this run were not recorded: {error}");
        }
        if let Err(error) = self.journal(&report, ended, status) {
            eprintln!("✗ the run was not added to the stage journal: {error}");
        }
        status
    }

    /// The pass records of the stages that passed and whose key held through the run.
    fn record_passes(&self, report: &Report) -> Result<(), String> {
        let passed = report.passed();
        if passed.is_empty() || self.keys.is_empty() || self.rerun {
            return Ok(());
        }
        let folder = passes::folder();
        for (stage, key) in keys::of_this_run(&self.context, &passed)? {
            if !self.keys.contains(&(stage.clone(), key.clone())) {
                println!(
                    "● {stage} passed, but what it reads changed during the run: no pass recorded"
                );
                continue;
            }
            let seconds = report
                .seconds(&stage)
                .and_then(|seconds| seconds.parse().ok())
                .unwrap_or(0.0);
            passes::record(&folder, &stage, &key, seconds, &self.context.root)?;
        }
        Ok(())
    }

    fn journal(&self, report: &Report, ended: f64, status: i32) -> Result<(), String> {
        let run = journal::Run {
            log: self.log.path().to_path_buf(),
            stages: self.stages.clone(),
            started: self.started,
            ended,
            exit: status,
            interrupted: self.interrupt.raised(),
            caller: self.caller,
            worktree: self.context.root.clone(),
            recipe: var("VOICEPUNCHER_STAGES_RECIPE"),
            accuracy: var("VOICEPUNCHER_STAGES_ACCURACY"),
        };
        journal::append(
            &journal::path(),
            &journal::line(&run, report, &crate::clock::local_zone()),
        )
    }
}

/// The pass record of each stage whose key has one.
fn passed(keys: &[(String, String)]) -> BTreeMap<String, PathBuf> {
    let folder = passes::folder();
    keys.iter()
        .filter_map(|(stage, key)| {
            passes::found(&folder, stage, key).map(|record| (stage.clone(), record))
        })
        .collect()
}
