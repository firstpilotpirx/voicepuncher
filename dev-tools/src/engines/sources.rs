//! Where the pinned engines come from, checked before anything is written: each Homebrew formula at the version
//! the lock pins, each release archive with its checksum (downloaded into the engine cache once), and every file
//! taken with the checksum the lock names.

use super::lock::{field, EngineLock};
use super::{run, sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where one engine's files are read from.
pub enum Source {
    /// `<formula>/<path>` in the formula's pinned version folder of the Homebrew cellar.
    Cellar { cellar: PathBuf, formulae: BTreeMap<String, String> },
    /// A path inside an extracted release archive.
    Archive(PathBuf),
}

impl Source {
    pub fn path(&self, from: &str) -> PathBuf {
        match self {
            Self::Cellar { cellar, formulae } => {
                let (formula, path) = from.split_once('/').unwrap_or((from, ""));
                cellar
                    .join(formula)
                    .join(formulae.get(formula).map_or("", String::as_str))
                    .join(path)
            }
            Self::Archive(base) => base.join(from),
        }
    }
}

/// The sources of whisper and llama, the Homebrew cellar when one was needed, and the problems found.
#[derive(Default)]
pub struct Sources {
    pub of: BTreeMap<String, Source>,
    pub cellar: Option<PathBuf>,
    pub problems: Vec<String>,
    /// The extracted archives, removed with the sources.
    pub extracted: Vec<tempfile::TempDir>,
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
        self.of
            .insert(engine.into(), Source::Cellar { cellar, formulae });
        Ok(())
    }

    /// A release archive, downloaded once into the cache; whether its checksum held.
    fn release(
        &mut self,
        engine: &str,
        pins: &BTreeMap<String, String>,
        cache: &Path,
    ) -> Result<bool, String> {
        let dir = cache.join(engine).join(field(pins, "version"));
        let archive = dir.join(field(pins, "asset"));
        if !archive.is_file() {
            download(&dir, &archive, pins)?;
        }
        let got = sha256(&archive)?;
        if got != field(pins, "asset_sha256") {
            self.problems.push(format!(
                "{}: sha256 is {got}, engines.lock expects {} ({}; delete it to download it again, or correct the lock)",
                field(pins, "asset"),
                field(pins, "asset_sha256"),
                archive.display()
            ));
            return Ok(false);
        }
        let base = tempfile::tempdir().map_err(|e| e.to_string())?;
        run(
            "tar",
            &[
                "-xzf",
                &archive.to_string_lossy(),
                "-C",
                &base.path().to_string_lossy(),
            ],
        )?;
        self.of
            .insert(engine.into(), Source::Archive(base.path().to_path_buf()));
        self.extracted.push(base);
        Ok(true)
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

/// The sources of whisper and llama as the lock pins them.
pub fn verify(lock: &EngineLock, name: &str, cache: &Path) -> Result<Sources, String> {
    let mut sources = Sources::default();
    for engine in ["whisper", "llama"] {
        let pins = lock.section(engine, name)?;
        lock.section(&format!("{engine}.from"), name)?;
        match field(pins, "source") {
            "homebrew" => sources.homebrew(lock, engine)?,
            "github-release" => {
                if !sources.release(engine, pins, cache)? {
                    continue;
                }
            }
            other => {
                return Err(format!(
                    "[{engine}] source '{other}' is neither homebrew nor github-release"
                ))
            }
        }
        if sources.problems.is_empty() {
            sources.files(lock, engine);
        }
    }
    Ok(sources)
}

/// Downloads a release archive beside its final name, then moves it there.
fn download(dir: &Path, archive: &Path, pins: &BTreeMap<String, String>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    println!("→ downloading {}", field(pins, "url"));
    let part = dir.join(format!("{}.part", field(pins, "asset")));
    run(
        "curl",
        &[
            "-sSL",
            "--fail",
            "-o",
            &part.to_string_lossy(),
            field(pins, "url"),
        ],
    )?;
    std::fs::rename(&part, archive).map_err(|e| format!("{}: {e}", archive.display()))
}
