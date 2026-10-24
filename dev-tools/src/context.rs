//! Where a command runs: the `just` and `justfile` that called it (`DEV_TOOLS_JUST`, `DEV_TOOLS_JUSTFILE`, set
//! by the `justfile`'s `dev_tools`), the repository they belong to and the tables of `.config/stages.toml`.

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Context {
    pub just: String,
    pub justfile: PathBuf,
    pub root: PathBuf,
}

impl Context {
    /// The calling `just` and `justfile`; without them `just` on the `PATH` and `./justfile`.
    pub fn from_env() -> Self {
        let just = std::env::var("DEV_TOOLS_JUST").unwrap_or_else(|_| "just".into());
        let justfile = std::env::var_os("DEV_TOOLS_JUSTFILE")
            .map_or_else(|| PathBuf::from("justfile"), PathBuf::from);
        let justfile = std::path::absolute(&justfile).unwrap_or(justfile);
        let root = justfile
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        Self { just, justfile, root }
    }

    /// `just --justfile <justfile>`, to run a recipe of the same `justfile`.
    pub fn just(&self) -> Command {
        let mut command = Command::new(&self.just);
        command.arg("--justfile").arg(&self.justfile);
        command
    }

    /// The repository the keys and the frozen list are read from: `VOICEPUNCHER_STAGE_ROOT`, else the
    /// `justfile`'s.
    pub fn stage_root(&self) -> PathBuf {
        std::env::var_os("VOICEPUNCHER_STAGE_ROOT")
            .filter(|root| !root.is_empty())
            .map_or_else(|| self.root.clone(), PathBuf::from)
    }

    /// The text of the `name = '''…'''` table of `.config/stages.toml`.
    pub fn table(&self, name: &str) -> Result<String, String> {
        let path = self.root.join(TABLES);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        table(&text, name).ok_or_else(|| format!("{TABLES} has no table {name}"))
    }
}

/// The file of the stage runner's tables, beside the `justfile`.
pub const TABLES: &str = ".config/stages.toml";

/// The body of `name = '''…'''` in the tables' text.
pub fn table(text: &str, name: &str) -> Option<String> {
    let start = format!("{name} = '''\n");
    let from = text.find(&start)? + start.len();
    let length = text[from..].find("'''")?;
    Some(text[from..from + length].to_string())
}

/// A variable that is set and not empty.
pub fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The cache directory of the stage runner under the home directory, or the variable that overrides it.
pub fn cache_path(variable: &str, default: &str) -> PathBuf {
    var(variable).map_or_else(
        || home().join(".cache/voicepuncher").join(default),
        PathBuf::from,
    )
}

/// Cargo's target directory: `CARGO_TARGET_DIR`, else `target/` of the workspace this binary was built in.
pub fn target_dir() -> PathBuf {
    var("CARGO_TARGET_DIR").map_or_else(|| Context::from_env().root.join("target"), PathBuf::from)
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_a_table_of_the_tables_file() {
        let text = "a = \"x\"\nstage_inputs = '''\n    docs/*  code\n'''\nb = 1\n";
        assert_eq!(
            table(text, "stage_inputs").as_deref(),
            Some("    docs/*  code\n")
        );
        assert_eq!(table(text, "other"), None);
    }
}
