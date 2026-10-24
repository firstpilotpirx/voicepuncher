//! The engine lock and `just engines`, which stages the pinned engines into a folder: run on a
//! lock and a staging folder of the scenario's own, so the checkout's `app/packaging/engines/` is
//! never touched.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use super::{checkout, just, path_str, text};
use crate::steps::off_thread;
use crate::E2eWorld;

/// The engine sections of the lock and the folders `just engines` writes for them.
pub const ENGINES: [&str; 3] = ["whisper", "llama", "command-spotting"];
/// The file of an engine folder that holds its version.
pub const VERSION_FILE: &str = "VERSION";
/// The formula the wrong-version lock moves to a version no Mac has.
const PINNED_FORMULA: &str = "ggml";
const MISSING_VERSION: &str = "0.0.1";
const WRONG_CHECKSUM: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// What a scenario staged and what it expects the staging to say.
#[derive(Debug, Default)]
pub struct EngineRun {
    lock: Option<PathBuf>,
    out: Option<PathBuf>,
    run: Option<Output>,
    expected: Vec<String>,
}

/// The tables of the lock: `[section]` → key → value, for the `key = "value"` lines it holds.
pub fn read_lock(path: &Path) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let mut tables: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut section = String::new();
    for line in fs::read_to_string(path)?.lines() {
        let line = line.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name.to_string();
        } else if let Some((key, value)) = line.split_once(" = ") {
            if line.starts_with('#') {
                continue;
            }
            tables.entry(section.clone()).or_default().insert(
                key.trim_matches('"').to_string(),
                value.trim_matches('"').to_string(),
            );
        }
    }
    Ok(tables)
}

pub fn checkout_lock() -> PathBuf {
    checkout().join("app/packaging/engines.lock")
}

fn lock_with(world: &mut E2eWorld, edit: impl Fn(&str) -> String) -> Result<()> {
    let original = fs::read_to_string(checkout_lock())?;
    let lock = world.workdir()?.join("engines.lock");
    let edited = edit(&original);
    ensure!(edited != original, "the lock was not changed");
    fs::write(&lock, edited)?;
    world.distribution.engines.lock = Some(lock);
    Ok(())
}

#[given("an engine lock that names a Homebrew formula version that is not installed")]
fn lock_with_missing_version(world: &mut E2eWorld) -> Result<()> {
    let pinned = read_lock(&checkout_lock())?
        .get("whisper.formulae")
        .and_then(|formulae| formulae.get(PINNED_FORMULA))
        .cloned()
        .context("the lock pins no ggml")?;
    lock_with(world, |lock| {
        lock.replace(
            &format!("{PINNED_FORMULA} = \"{pinned}\""),
            &format!("{PINNED_FORMULA} = \"{MISSING_VERSION}\""),
        )
    })?;
    world.distribution.engines.expected = vec![
        format!("{PINNED_FORMULA} {MISSING_VERSION}"),
        pinned.clone(),
        "found: ".to_string(),
    ];
    Ok(())
}

#[given("an engine lock whose release archive checksum differs from the archive's")]
fn lock_with_wrong_checksum(world: &mut E2eWorld) -> Result<()> {
    let llama = read_lock(&checkout_lock())?
        .remove("llama")
        .context("the lock has no llama table")?;
    let asset = llama.get("asset").context("no asset")?.clone();
    let checksum = llama
        .get("asset_sha256")
        .context("no asset_sha256")?
        .clone();
    lock_with(world, |lock| lock.replace(&checksum, WRONG_CHECKSUM))?;
    world.distribution.engines.expected = vec![asset, checksum, WRONG_CHECKSUM.to_string()];
    Ok(())
}

#[given("the engine lock of the checkout")]
fn checkout_engine_lock(world: &mut E2eWorld) {
    world.distribution.engines.lock = Some(checkout_lock());
}

#[when("the engines are staged")]
async fn stage_engines(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let out = world.workdir()?.join("engines");
        let lock = world.distribution.engines.lock.clone().context("no lock")?;
        world.distribution.engines.run =
            Some(just(&["engines", path_str(&lock)?, path_str(&out)?], None)?);
        world.distribution.engines.out = Some(out);
        Ok(())
    })
    .await
}

fn staging(world: &E2eWorld) -> Result<(&Output, &Path)> {
    let engines = &world.distribution.engines;
    Ok((
        engines.run.as_ref().context("nothing was staged")?,
        engines.out.as_deref().context("no staging folder")?,
    ))
}

fn fails_naming_expected(world: &mut E2eWorld) -> Result<()> {
    let (run, _) = staging(world)?;
    let shown = text(run);
    ensure!(!run.status.success(), "the staging went on: {shown}");
    for expected in &world.distribution.engines.expected {
        ensure!(
            shown.contains(expected.as_str()),
            "it does not name '{expected}': {shown}"
        );
    }
    Ok(())
}

#[then("the staging fails naming the formula, the version the lock expects and the versions found")]
fn fails_naming_versions(world: &mut E2eWorld) -> Result<()> {
    fails_naming_expected(world)
}

#[then("it says how to pin the formula")]
fn says_how_to_pin(world: &mut E2eWorld) -> Result<()> {
    let (run, _) = staging(world)?;
    let shown = text(run);
    ensure!(
        shown.contains("brew pin"),
        "it does not say 'brew pin': {shown}"
    );
    Ok(())
}

#[then(
    "the staging fails naming the archive, the checksum found and the checksum the lock expects"
)]
fn fails_naming_checksums(world: &mut E2eWorld) -> Result<()> {
    fails_naming_expected(world)
}

#[then("no engine folder is written")]
fn no_engine_folder(world: &mut E2eWorld) -> Result<()> {
    let (_, out) = staging(world)?;
    for engine in ENGINES {
        ensure!(
            !out.join(engine).exists(),
            "it wrote {}",
            out.join(engine).display()
        );
    }
    Ok(())
}

fn staged(world: &E2eWorld) -> Result<&Path> {
    let (run, out) = staging(world)?;
    ensure!(run.status.success(), "the staging failed: {}", text(run));
    Ok(out)
}

#[then("each engine folder holds exactly the files the lock lists for it")]
fn holds_locked_files(world: &mut E2eWorld) -> Result<()> {
    let out = staged(world)?;
    let lock = read_lock(&checkout_lock())?;
    for engine in ENGINES {
        let table = lock.get(engine).context(format!("no [{engine}]"))?;
        let mut expected: Vec<String> = match lock.get(&format!("{engine}.from")) {
            Some(files) => files.keys().cloned().collect(),
            None => vec![table.get("program").context("no program")?.clone()],
        };
        expected.push(VERSION_FILE.to_string());
        expected.sort();
        let mut found: Vec<String> = fs::read_dir(out.join(engine))?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_>>()?;
        found.sort();
        ensure!(
            found == expected,
            "{engine} holds {found:?}, the lock lists {expected:?}"
        );
    }
    Ok(())
}

#[then("each engine's version file holds the lock's version")]
fn version_files(world: &mut E2eWorld) -> Result<()> {
    let out = staged(world)?;
    let lock = read_lock(&checkout_lock())?;
    for engine in ["whisper", "llama"] {
        let expected = lock[engine].get("version").context("no version")?;
        let found = fs::read_to_string(out.join(engine).join(VERSION_FILE))?;
        ensure!(
            found.trim() == expected,
            "{engine}'s version file holds '{found}', the lock says '{expected}'"
        );
    }
    let manifest = fs::read_to_string(checkout().join("command-spotting/Cargo.toml"))?;
    let crate_version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .context("command-spotting/Cargo.toml names no version")?;
    let found = fs::read_to_string(out.join("command-spotting").join(VERSION_FILE))?;
    ensure!(
        found.trim() == crate_version,
        "the command spotting version file holds '{found}', its crate is {crate_version}"
    );
    Ok(())
}
