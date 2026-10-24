//! `dev-tools test-stage`: the `test` stage — one nextest run over the workspace's test targets except the E2E
//! harness, whose suites are the E2E recipes'. Only the packages whose part key (`test@<member>`) has no pass
//! record run; the others are reused (↺), and each part that ran gets its record when what it reads held
//! through the run (`VOICEPUNCHER_REUSE_STAGES=0` runs every part). The build runs at the utility quality of service clamp; the
//! tests keep the default quality of service (clamped timers fire late under load). A rerun (`VOICEPUNCHER_RERUN_FILTER`) runs
//! only the tests of that selection over the workspace, reusing and recording nothing. With the machine lock on,
//! `MACHINE_LOCK` names its wrapper, for the tests of the lock itself (`needs_machine_lock`).

use crate::clock::{clock_time, now};
use crate::context::{var, Context};
use crate::filters::{gates, Selection};
use crate::locks::Lock;
use crate::process::status;
use crate::{keys, passes, Outcome};
use std::path::Path;
use std::process::Command;

const NOT_E2E: &str = "not binary_id(voicepuncher-e2e-test::cucumber_e2e)";

/// `cargo nextest run`, without the `VOICEPUNCHER_*` variables that are set empty: a model path the
/// `justfile` did not find is exported empty, and the tests close a need only when its variable is unset.
fn nextest(args: &[String]) -> Command {
    let mut command = Command::new("cargo");
    command.args(["nextest", "run"]).args(args);
    for (name, value) in std::env::vars_os() {
        if value.is_empty() && name.to_string_lossy().starts_with("VOICEPUNCHER_") {
            command.env_remove(name);
        }
    }
    if let Some(wrapper) = Lock::from_env().wrapper {
        command.env("MACHINE_LOCK", wrapper);
    }
    command
}

/// Builds the selection at the utility clamp, then runs it; nextest's exit code.
fn build_and_run(selection: &[String], filter: &str) -> i32 {
    let mut compile: Vec<String> = selection.to_vec();
    compile.extend(["-E".into(), NOT_E2E.into(), "--no-run".into()]);
    let mut clamped = Command::new("taskpolicy");
    clamped
        .args(["-c", "utility", "cargo", "nextest", "run"])
        .args(&compile);
    let built = status(&mut clamped);
    if built != 0 {
        return built;
    }
    let mut run = selection.to_vec();
    run.extend(["-E".into(), filter.into()]);
    status(&mut nextest(&run))
}

pub fn command(_: &[String]) -> Outcome {
    let context = Context::from_env();
    let frozen = var("VOICEPUNCHER_FROZEN_FILTER");
    if let Some(rerun) = var("VOICEPUNCHER_RERUN_FILTER") {
        let filter = gates::gated(
            &context,
            "--workspace",
            &Selection::Test.narrowed(Some(&rerun), frozen.as_deref()),
        )?;
        let selection = ["--workspace".to_string(), "--no-tests=pass".into()];
        return Ok(crate::process::code(build_and_run(&selection, &filter)));
    }
    let keys = keys::of_this_run(&context, &["test@members".to_string()]).unwrap_or_else(|error| {
        eprintln!("✗ the part keys could not be computed ({error}): every package runs and no part pass is recorded");
        Vec::new()
    });
    let parts = parts_to_run(&keys);
    let packages = packages(&context.root, &keys, &parts);
    let started = now();
    if packages.is_empty() {
        println!("↺ every package reused: no test runs");
    } else {
        let filter = gates::gated(
            &context,
            &packages.join(" "),
            &Selection::Test.narrowed(None, frozen.as_deref()),
        )?;
        let outcome = build_and_run(&packages, &filter);
        if outcome != 0 {
            return Ok(crate::process::code(outcome));
        }
    }
    record(&context, &keys, &parts, now() - started)?;
    Ok(0)
}

/// The parts without a pass record (every one with `VOICEPUNCHER_REUSE_STAGES=0`); says which are reused.
fn parts_to_run(keys: &[(String, String)]) -> Vec<(String, String)> {
    let reuse = var("VOICEPUNCHER_REUSE_STAGES").as_deref() != Some("0");
    let folder = passes::folder();
    keys.iter()
        .filter(|(part, key)| {
            let Some(record) = passes::found(&folder, part, key).filter(|_| reuse) else {
                return true;
            };
            println!(
                "↺ {} {part} reused: {}",
                clock_time(),
                passes::line(&record)
            );
            false
        })
        .cloned()
        .collect()
}

/// `-p <package>` per part to run; `--workspace` without keys.
fn packages(root: &Path, keys: &[(String, String)], parts: &[(String, String)]) -> Vec<String> {
    if keys.is_empty() {
        return vec!["--workspace".into()];
    }
    parts
        .iter()
        .filter_map(|(part, _)| part.strip_prefix("test@"))
        .flat_map(|member| {
            [
                "-p".to_string(),
                package_name(&root.join(member).join("Cargo.toml")),
            ]
        })
        .collect()
}

/// The first `name = "…"` of a manifest.
pub fn package_name(manifest: &Path) -> String {
    let text = std::fs::read_to_string(manifest).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix("name"))
        .filter_map(|rest| rest.trim_start().strip_prefix('='))
        .find_map(|rest| {
            rest.trim()
                .strip_prefix('"')
                .and_then(|rest| rest.split('"').next())
        })
        .unwrap_or_default()
        .to_string()
}

/// The pass records of the parts that ran, when what they read held through the run.
fn record(
    context: &Context,
    keys: &[(String, String)],
    parts: &[(String, String)],
    seconds: f64,
) -> Result<(), String> {
    if parts.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = parts.iter().map(|(part, _)| part.clone()).collect();
    let folder = passes::folder();
    let seconds = crate::clock::round1(seconds);
    for (part, key) in keys::of_this_run(context, &names)? {
        if keys.contains(&(part.clone(), key.clone())) {
            passes::record(&folder, &part, &key, seconds, &context.root)?;
        } else {
            println!("● {part} passed, but what it reads changed during the run: no pass recorded");
        }
    }
    Ok(())
}
