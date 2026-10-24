//! The steps of `change_workflow.feature`, by area: the rules the repository's files hold (kit configuration,
//! analyzers, hosted check, test layout) and the stage runner (`dev-tools`) driven in scratch repositories with
//! stand-ins for the task runner, the machine lock and the test runner, as its unit tests drive it.

pub mod analyzers;
pub mod frozen;
pub mod hosted;
pub mod journal;
pub mod landing;
pub mod locks;
pub mod named;
pub mod needs;
pub mod scratch;
pub mod test_rules;

use std::path::{Path, PathBuf};
use std::process::Command;

/// What a command printed and how it ended.
#[derive(Debug, Default)]
pub struct Ran {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Ran {
    /// Runs the command to its end.
    pub fn of(command: &mut Command) -> Self {
        let output = command.output().expect("the command starts");
        Self {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Both streams, stdout first.
    pub fn both(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

/// The workspace root.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A file of the repository, by its path from the workspace root.
pub fn repo_file(path: &str) -> String {
    std::fs::read_to_string(workspace().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The world's last command, which a `When` step ran.
pub fn ran(world: &crate::WorkflowWorld) -> &Ran {
    world.ran.as_ref().expect("a When step ran a command")
}

/// The world's scratch repository, which a `Given` step made.
pub fn scratch(world: &crate::WorkflowWorld) -> &scratch::Scratch {
    world
        .scratch
        .as_ref()
        .expect("a Given step made a scratch repository")
}

/// The body of a `justfile` recipe: the lines after its header line up to the next unindented line.
pub fn recipe_body(justfile: &str, header: &str) -> String {
    let mut lines = justfile
        .lines()
        .skip_while(|line| !line.starts_with(header));
    let Some(_) = lines.next() else {
        panic!("the justfile has no recipe {header}");
    };
    lines
        .take_while(|line| line.is_empty() || line.starts_with(' '))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The value of `key:` in the top-level `section:` of a YAML text, read line by line.
pub fn yaml_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        if !line.starts_with(' ') {
            in_section = line.trim_end() == format!("{section}:");
            continue;
        }
        let value = line
            .trim()
            .strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(':'));
        if let (true, Some(value)) = (in_section, value) {
            return Some(value.trim().to_string());
        }
    }
    None
}
