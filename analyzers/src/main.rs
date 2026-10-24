//! `voicepuncher-analyzers <tool> [--write-baseline]`: runs one analyzer from the workspace root and compares its
//! findings with `.config/analyzers/<tool>.toml` (AGENTS.md § How to verify a change).

mod baseline;
mod clippy;
mod metrics;
mod recipes;
mod size;

use baseline::Counts;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str =
    "usage: voicepuncher-analyzers <clippy|recipes|metrics|size> [--write-baseline] | metrics --changed <base>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let write = args.iter().any(|a| a == "--write-baseline");
    let Some(tool) = args.first().filter(|a| !a.starts_with("--")) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let outcome = dispatch(tool, &args, write);
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("✗ {tool}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the tool the arguments name: `size`, `metrics --changed <base>`, or a ratcheted analyzer.
fn dispatch(tool: &str, args: &[String], write: bool) -> Result<bool, String> {
    let changed = args
        .iter()
        .position(|a| a == "--changed")
        .map(|at| args.get(at + 1));
    match (tool, changed) {
        ("size", None) => size::run(Path::new("."), write),
        ("metrics", Some(Some(base))) if !write => changed_metrics(base),
        (_, Some(_)) => Err(format!(
            "--changed takes a base and goes only with metrics, without --write-baseline\n{USAGE}"
        )),
        _ => run(tool).and_then(|counts| ratchet(tool, &counts, write)),
    }
}

/// The metrics ratchet over the files changed against `base`; prints every breach, true when none.
fn changed_metrics(base: &str) -> Result<bool, String> {
    let config = Config::load()?;
    let breaches = metrics::run_changed(
        &config.metrics,
        base,
        Path::new(".config/analyzers/metrics.toml"),
    )?;
    for breach in &breaches {
        println!("{breach}");
    }
    if breaches.is_empty() {
        println!("✓ metrics: no changed function beyond .config/analyzers/metrics.toml");
    }
    Ok(breaches.is_empty())
}

/// Runs the named analyzer and returns its findings.
fn run(tool: &str) -> Result<Counts, String> {
    let config = Config::load()?;
    match tool {
        "clippy" => clippy::run(&config.clippy),
        "recipes" => recipes::run(),
        "metrics" => metrics::run(&config.metrics),
        _ => Err(format!("unknown tool '{tool}'\n{USAGE}")),
    }
}

/// `.config/analyzers.toml`: what each analyzer checks.
#[derive(serde::Deserialize)]
struct Config {
    clippy: clippy::Config,
    metrics: metrics::Config,
}

impl Config {
    fn load() -> Result<Self, String> {
        let path = ".config/analyzers.toml";
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        toml::from_str(&text).map_err(|e| format!("{path}: {e}"))
    }
}

/// Writes the baseline, or compares with it and prints every breach; true when the findings hold the line.
fn ratchet(tool: &str, counts: &Counts, write: bool) -> Result<bool, String> {
    let path = Path::new(".config/analyzers").join(format!("{tool}.toml"));
    if write {
        std::fs::write(&path, baseline::render(tool, counts))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("● {tool}: baseline written to {}", path.display());
        return Ok(true);
    }
    let breaches = baseline::compare(&baseline::load(&path)?, counts);
    for breach in &breaches {
        println!("{breach}");
    }
    let total: u64 = counts.values().sum();
    if breaches.is_empty() {
        println!("✓ {tool}: {total} findings, none beyond {}", path.display());
    }
    Ok(breaches.is_empty())
}
