//! `dev-tools engines <lock> <out>`: stages the engines of a lock into a folder. It checks every pinned version
//! and checksum first and writes nothing when one differs; then copies each engine's files, relocates them
//! (`relocate`), proves no reference leaves the folder and each engine runs with nothing but the system folders
//! on `$PATH`, and writes each folder's `VERSION` and the licenses. Whisper is kept when the folder was
//! staged from the same lock; the command spotting service is rebuilt every time.
//!
//! `dev-tools check-bundle <app> <dmg>`: checks a built bundle and its disk image (`bundle`).

pub mod bundle;
mod lock;
mod relocate;
mod sources;
mod staging;
#[cfg(test)]
mod tests;

use crate::context::Context;
use crate::Outcome;
use lock::{field, EngineLock};
use sha2::{Digest, Sha256};
use sources::Sources;
use staging::{crate_version, Staging};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const ENGINES: [&str; 2] = ["whisper", "command-spotting"];

/// Runs a program; fails naming it.
fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let ran = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    ran.success()
        .then_some(())
        .ok_or_else(|| format!("{program} {} failed", args.join(" ")))
}

/// A program's output, standard error included; fails with it.
fn capture(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out.status
        .success()
        .then(|| text.clone())
        .ok_or_else(|| format!("{program} {} failed:\n{text}", args.join(" ")))
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        }))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn copy_file(from: &Path, to: &Path, what: &str) -> Result<(), String> {
    std::fs::copy(from, to).map_err(|e| format!("copy {what}: {e}"))?;
    let _ = crate::process::status(Command::new("chmod").arg("755").arg(to));
    Ok(())
}

pub fn command(args: &[String]) -> Outcome {
    let [lock_path, out] = args else {
        return Err("engines: give the lock and the folder".into());
    };
    let (lock_path, out) = (Path::new(lock_path), PathBuf::from(out));
    let shown = lock_path.to_string_lossy().into_owned();
    let text =
        std::fs::read_to_string(lock_path).map_err(|e| format!("engine lock {shown}: {e}"))?;
    let lock = EngineLock::parse(&text, &shown)?;
    let lock_sha = sha256(lock_path)?;
    let stamp = out.join(".lock-sha256");
    let kept = std::fs::read_to_string(&stamp).is_ok_and(|held| held.trim_end() == lock_sha)
        && ENGINES.iter().all(|engine| out.join(engine).is_dir());
    let sources = verified(&lock, &shown)?;
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let stage = tempfile::Builder::new()
        .prefix("engines.")
        .tempdir_in(&out)
        .map_err(|e| e.to_string())?;
    let fresh: Vec<&str> = if kept {
        vec!["command-spotting"]
    } else {
        ENGINES.to_vec()
    };
    let staging = Staging {
        lock: &lock,
        sources: &sources,
        stage: stage.path(),
        root: Context::from_env().root,
    };
    staging.stage(&fresh)?;
    for engine in fresh.iter().copied().chain(["licenses"]) {
        // A folder of an earlier staging goes first; on the first there is none
        let _ = std::fs::remove_dir_all(out.join(engine));
        std::fs::rename(stage.path().join(engine), out.join(engine))
            .map_err(|e| format!("{}/{engine}: {e}", out.display()))?;
    }
    write(&stamp, &format!("{lock_sha}\n"))?;
    summary(&out, &fresh);
    Ok(0)
}

/// The sources of every engine, each version and checksum as the lock pins it; fails naming every difference.
fn verified(lock: &EngineLock, shown: &str) -> Result<Sources, String> {
    accepted(sources::verify(lock, shown)?, shown)
}

/// The sources when they hold no problem; else every problem printed and the staging refused.
fn accepted(sources: Sources, shown: &str) -> Result<Sources, String> {
    if sources.problems.is_empty() {
        return Ok(sources);
    }
    for problem in &sources.problems {
        eprintln!("✗ {problem}");
    }
    Err(format!(
        "no engine staged: the engines are taken only as {shown} pins them (README § Engines)"
    ))
}

fn summary(out: &Path, fresh: &[&str]) {
    println!("  {:<17} {:<10} size", "engine", "version");
    for engine in ENGINES {
        let dir = out.join(engine);
        let kb: f64 = crate::process::stdout(Command::new("du").arg("-sk").arg(&dir))
            .split_whitespace()
            .next()
            .and_then(|kb| kb.parse().ok())
            .unwrap_or(0.0);
        let version = std::fs::read_to_string(dir.join("VERSION")).unwrap_or_default();
        let kept = if fresh.contains(&engine) {
            ""
        } else {
            " (kept)"
        };
        println!(
            "  {engine:<17} {:<10} {:.1} MB{kept}",
            version.lines().next().unwrap_or(""),
            kb / 1024.0
        );
    }
    println!("✓ engines staged in {}", out.display());
}
