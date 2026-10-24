//! One staging of engines: each engine copied or built into a scratch folder beside the output, with its licenses.

use super::{copy_file, field, relocate, run, write, EngineLock, Sources};
use crate::context::target_dir;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One staging of engines into a scratch folder beside the output.
pub(super) struct Staging<'a> {
    pub(super) lock: &'a EngineLock,
    pub(super) sources: &'a Sources,
    pub(super) stage: &'a Path,
    pub(super) root: PathBuf,
}

impl Staging<'_> {
    pub(super) fn stage(&self, fresh: &[&str]) -> Result<(), String> {
        for engine in fresh.iter().filter(|engine| **engine != "command-spotting") {
            self.copy_engine(engine)?;
        }
        self.command_spotting()?;
        for engine in fresh {
            self.check(engine)?;
        }
        self.licenses()
    }

    fn copy_engine(&self, engine: &str) -> Result<(), String> {
        let dir = self.stage.join(engine);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let from = self.lock.optional(&format!("{engine}.from"));
        let source = self
            .sources
            .of
            .get(engine)
            .ok_or_else(|| format!("no source of {engine}"))?;
        for (file, place) in &from {
            copy_file(&source.path(place), &dir.join(file), file)?;
            // Extended attributes only stop a signature: a file without them needs nothing
            let _ = crate::process::status(Command::new("xattr").arg("-c").arg(dir.join(file)));
        }
        // The compiled-in folders of the formulae (ggml's GGML_BACKEND_DIR), so a backend is never loaded from them
        let folders: Vec<String> = match &self.sources.cellar {
            Some(cellar) => self
                .lock
                .optional(&format!("{engine}.formulae"))
                .iter()
                .map(|(formula, version)| {
                    format!("{}/{formula}/{version}/libexec", cellar.display())
                })
                .collect(),
            None => Vec::new(),
        };
        for file in from.keys() {
            relocate::relocate(&dir.join(file), file, &folders)?;
        }
        write(
            &dir.join("VERSION"),
            &format!(
                "{}\n",
                field(self.lock.section(engine, "the lock")?, "version")
            ),
        )
    }

    fn command_spotting(&self) -> Result<(), String> {
        let spotting = self.lock.section("command-spotting", "the lock")?;
        let program = field(spotting, "program");
        run(
            "cargo",
            &[
                "build",
                "--quiet",
                "--release",
                "-p",
                field(spotting, "package"),
            ],
        )?;
        let dir = self.stage.join("command-spotting");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        copy_file(
            &target_dir().join("release").join(program),
            &dir.join(program),
            program,
        )?;
        write(
            &dir.join("VERSION"),
            &format!(
                "{}\n",
                crate_version(&self.root.join("command-spotting/Cargo.toml"))
            ),
        )
    }

    /// Every reference stays in the engine's folder, and the engine runs with a clean environment.
    fn check(&self, engine: &str) -> Result<(), String> {
        let dir = self.stage.join(engine);
        relocate::check_folder(&dir, &dir.to_string_lossy())?;
        let program = field(self.lock.section(engine, "the lock")?, "program");
        let flag = if engine == "whisper" {
            "--help"
        } else {
            "--version"
        };
        let said = Command::new(dir.join(program))
            .arg(flag)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .output();
        match said {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(format!(
                "{program} {flag} failed with nothing but the system folders on $PATH:\n{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )),
            Err(error) => Err(format!(
                "{program} {flag} failed with nothing but the system folders on $PATH:\n{error}"
            )),
        }
    }

    /// The licenses of every bundled engine and library.
    fn licenses(&self) -> Result<(), String> {
        let dir = self.stage.join("licenses");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let crates =
            crates(&std::fs::read_to_string(self.root.join("Cargo.lock")).unwrap_or_default());
        let registry = registry();
        for (name, place) in &self.lock.optional("licenses") {
            let (kind, path) = place.split_once(':').unwrap_or((place, ""));
            let (first, rest) = path.split_once('/').unwrap_or((path, ""));
            let file = match kind {
                "homebrew" => {
                    let version = self
                        .lock
                        .optional("whisper.formulae")
                        .get(first)
                        .cloned()
                        .unwrap_or_default();
                    self.sources
                        .cellar
                        .clone()
                        .unwrap_or_default()
                        .join(first)
                        .join(version)
                        .join(rest)
                }
                "cargo" => registry
                    .join(format!(
                        "{first}-{}",
                        crates.get(first).map_or("", String::as_str)
                    ))
                    .join(rest),
                _ => self.root.join(path),
            };
            std::fs::copy(&file, dir.join(name))
                .map_err(|e| format!("license {name} ({}): {e}", file.display()))?;
        }
        Ok(())
    }
}

/// The `version = "…"` of a manifest's first such line.
pub(super) fn crate_version(manifest: &Path) -> String {
    let text = std::fs::read_to_string(manifest).unwrap_or_default();
    text.lines()
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .and_then(|rest| rest.split('"').next())
        })
        .unwrap_or_default()
        .to_string()
}

/// The version of each package `Cargo.lock` names.
pub(super) fn crates(lock: &str) -> std::collections::BTreeMap<String, String> {
    let mut found = std::collections::BTreeMap::new();
    let mut name: Option<&str> = None;
    for line in lock.lines() {
        if let Some(value) = line
            .strip_prefix("name = \"")
            .and_then(|rest| rest.strip_suffix('"'))
        {
            name = Some(value);
        } else if let (Some(package), Some(version)) = (
            name.take(),
            line.strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"')),
        ) {
            found.insert(package.to_string(), version.to_string());
        }
    }
    found
}

/// The first source folder of the Cargo registry.
fn registry() -> PathBuf {
    let folder = crate::context::home().join(".cargo/registry/src");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found.into_iter().next().unwrap_or_default()
}
