//! Where the pinned engines come from, checked before anything is written: each Homebrew formula at the version
//! the lock pins, and every file taken with the checksum the lock names.

use super::lock::{field, EngineLock};
use super::sha256;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where one engine's files are read from: `<formula>/<path>` in the formula's pinned version folder of the
/// Homebrew cellar.
pub struct Source {
    pub cellar: PathBuf,
    pub formulae: BTreeMap<String, String>,
}

impl Source {
    pub fn path(&self, from: &str) -> PathBuf {
        let (formula, path) = from.split_once('/').unwrap_or((from, ""));
        self.cellar
            .join(formula)
            .join(self.formulae.get(formula).map_or("", String::as_str))
            .join(path)
    }
}

/// The source of whisper, the Homebrew cellar when one was needed, and the problems found.
#[derive(Default)]
pub struct Sources {
    pub of: BTreeMap<String, Source>,
    pub cellar: Option<PathBuf>,
    pub problems: Vec<String>,
}

impl Sources {
    fn cellar(&mut self) -> Result<PathBuf, String> {
        if let Some(cellar) = &self.cellar {
            return Ok(cellar.clone());
        }
        let found = crate::process::stdout(std::process::Command::new("brew").arg("--cellar"));
        let found = found.trim();
        if found.is_empty() {
            return Err(
                "Homebrew is needed to stage the engines (developers only): https://brew.sh".into(),
            );
        }
        self.cellar = Some(PathBuf::from(found));
        Ok(PathBuf::from(found))
    }

    fn homebrew(&mut self, lock: &EngineLock, engine: &str) -> Result<(), String> {
        let cellar = self.cellar()?;
        let formulae = lock.optional(&format!("{engine}.formulae"));
        let names: Vec<&str> = formulae.keys().map(String::as_str).collect();
        for (formula, version) in &formulae {
            if cellar.join(formula).join(version).is_dir() {
                continue;
            }
            let found = installed(&cellar.join(formula));
            let found = if found.is_empty() {
                "none".to_string()
            } else {
                found.join(", ")
            };
            self.problems.push(format!(
                "{formula} {version} is not installed (found: {found}) — engines.lock pins {formula} {version}: install that version or update the lock, and keep it with 'brew pin {}'",
                names.join(" ")
            ));
        }
        self.of.insert(engine.into(), Source { cellar, formulae });
        Ok(())
    }

    /// Every file of an engine with the checksum the lock names.
    fn files(&mut self, lock: &EngineLock, engine: &str) {
        let (from, sums) = (
            lock.optional(&format!("{engine}.from")),
            lock.optional(&format!("{engine}.sha256")),
        );
        let Some(source) = self.of.get(engine) else {
            return;
        };
        let mut problems = Vec::new();
        for (file, place) in &from {
            let path = source.path(place);
            if !path.is_file() {
                problems.push(format!(
                    "{engine}: {} ({file}) does not exist",
                    path.display()
                ));
                continue;
            }
            problems.extend(checksum_problem(
                engine,
                file,
                &path,
                sums.get(file).map_or("", String::as_str),
            ));
        }
        self.problems.extend(problems);
    }
}

/// The problem of a file whose checksum is not the one the lock names.
fn checksum_problem(engine: &str, file: &str, path: &Path, want: &str) -> Option<String> {
    let got = sha256(path).unwrap_or_default();
    let want = if want.is_empty() { "none" } else { want };
    (got != want).then(|| {
        format!(
            "{engine}: {file} ({}) has sha256 {got}, engines.lock expects {want}",
            path.display()
        )
    })
}

/// The version folders of a formula in the cellar, by name.
fn installed(formula: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(formula)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// The source of whisper as the lock pins it.
pub fn verify(lock: &EngineLock, name: &str) -> Result<Sources, String> {
    verify_in(Sources::default(), lock, name)
}

/// The source of whisper as the lock pins it, in the Homebrew cellar `sources` names when it names one.
pub fn verify_in(mut sources: Sources, lock: &EngineLock, name: &str) -> Result<Sources, String> {
    let engine = "whisper";
    let pins = lock.section(engine, name)?;
    lock.section(&format!("{engine}.from"), name)?;
    match field(pins, "source") {
        "homebrew" => sources.homebrew(lock, engine)?,
        other => return Err(format!("[{engine}] source '{other}' is not homebrew")),
    }
    if sources.problems.is_empty() {
        sources.files(lock, engine);
    }
    Ok(sources)
}
