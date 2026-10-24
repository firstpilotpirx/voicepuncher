//! What a key covers beside the repository's files: the toolchain, the input variables and, for a model
//! stage, what it runs on outside the repository (the size and modification time of every file a variable
//! names, and of the speech recognition and text refinement servers).

use crate::process::{find_program, stdout};
use regex::Regex;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

/// The variables that feed every key: the project's, Cucumber's, and the Rust and Cargo build settings.
static INPUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "^(VOICEPUNCHER_|CUCUMBER_|RUSTFLAGS$|RUSTDOCFLAGS$|RUSTC_WRAPPER$|CARGO_BUILD_|CARGO_PROFILE_|CARGO_TARGET_\
         |CARGO_ENCODED_RUSTFLAGS$|NEXTEST_PROFILE$|NEXTEST_RETRIES$|NEXTEST_TEST_THREADS$)",
    )
    .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});

/// The input variables that change from run to run and feed no key: the journal and reports, ports, the
/// lock's and the frozen list's exports, colours.
static VOLATILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "^(VOICEPUNCHER_STAGE.*|VOICEPUNCHER_REUSE_STAGES|VOICEPUNCHER_TEST_PORTS|VOICEPUNCHER_.*_PORT\
         |VOICEPUNCHER_E2E_SERVICES_HELD|VOICEPUNCHER_FAIL_FAST|VOICEPUNCHER_FROZEN_.*|CARGO_TERM_.*)$",
    )
    .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});

/// The programs whose binaries a model stage's key covers.
pub const SERVERS: [&str; 2] = ["whisper-server", "llama-server"];

/// What every key covers beside the files.
pub struct Inputs {
    pub toolchain: String,
    pub variables: String,
    pub runtime: String,
    pub table: String,
}

impl Inputs {
    /// From the variables given, the servers found (or not) and the toolchain text.
    pub fn new(
        toolchain: String,
        env: &BTreeMap<String, String>,
        servers: &[(&str, Option<PathBuf>)],
        table: String,
    ) -> Self {
        let mut variables = String::new();
        let mut runtime = String::new();
        for (name, value) in env.iter().filter(|(name, _)| is_input(name)) {
            let _ = writeln!(variables, "{name}={value}");
            if value.starts_with('/') && Path::new(value).exists() {
                runtime.push_str(&described(Path::new(value)));
            }
        }
        for (name, path) in servers {
            runtime.push_str(name);
            runtime.push(' ');
            runtime.push_str(
                &path
                    .as_deref()
                    .map_or_else(|| "none\n".to_string(), described),
            );
        }
        Self { toolchain, variables, runtime, table }
    }

    /// This process's: its variables, the servers on its `PATH`, the toolchain `just` runs with.
    pub fn gather(just: &str, table: String) -> Self {
        let env: BTreeMap<String, String> = std::env::vars().collect();
        let servers: Vec<(&str, Option<PathBuf>)> = SERVERS
            .iter()
            .map(|name| (*name, find_program(name)))
            .collect();
        Self::new(toolchain(just), &env, &servers, table)
    }
}

fn is_input(name: &str) -> bool {
    INPUT.is_match(name) && !VOLATILE.is_match(name)
}

/// rustc, cargo, nextest, just and the macOS build (which fixes the `say` voices); `none` for each that
/// prints nothing.
pub fn toolchain(just: &str) -> String {
    let commands: [(&str, &[&str]); 5] = [
        ("rustc", &["-vV"]),
        ("cargo", &["-V"]),
        ("cargo", &["nextest", "--version"]),
        (just, &["--version"]),
        ("sw_vers", &["-buildVersion"]),
    ];
    commands
        .iter()
        .map(|(program, args)| {
            let out = stdout(Command::new(program).args(*args));
            if out.is_empty() {
                "none\n".to_string()
            } else {
                out
            }
        })
        .collect()
}

/// `<path below> <size> <mtime>` of a file, or of every file under a folder, in sorted order.
pub fn described(path: &Path) -> String {
    let mut files = Vec::new();
    walk(path, &mut files);
    files
        .iter()
        .filter_map(|file| {
            let meta = std::fs::metadata(file).ok()?;
            let below = file.to_string_lossy();
            let below = below
                .strip_prefix(&*path.to_string_lossy())
                .unwrap_or(&below)
                .to_string();
            Some(format!("{below} {} {}\n", meta.len(), mtime(&meta)))
        })
        .collect()
}

fn walk(path: &Path, files: &mut Vec<PathBuf>) {
    let is_folder = std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir());
    if !is_folder {
        if path.is_file() {
            files.push(path.to_path_buf());
        }
        return;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for entry in entries {
        walk(&entry, files);
    }
}

fn mtime(meta: &std::fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_leave_the_volatile_variables_out() {
        assert!(is_input("VOICEPUNCHER_HANG_PROBE"));
        assert!(is_input("RUSTFLAGS"));
        assert!(!is_input("RUSTFLAGS_EXTRA"));
        assert!(!is_input("VOICEPUNCHER_TEST_STT_PORT"));
        assert!(!is_input("VOICEPUNCHER_STAGE_JOURNAL"));
        assert!(!is_input("VOICEPUNCHER_FROZEN_FILTER"));
        assert!(!is_input("CARGO_TERM_COLOR"));
        assert!(!is_input("HOME"));
    }

    #[test]
    fn should_describe_every_file_of_a_folder_in_sorted_order() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        std::fs::write(dir.path().join("b/x"), "12").unwrap();
        std::fs::write(dir.path().join("a"), "1").unwrap();
        let text = described(dir.path());
        let lines: Vec<&str> = text
            .lines()
            .map(|line| line.split(' ').next().unwrap_or(""))
            .collect();
        assert_eq!(lines, ["/a", "/b/x"]);
        assert!(text.starts_with("/a 1 "));
    }
}
