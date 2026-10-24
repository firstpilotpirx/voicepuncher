//! A scratch repository for the stage runner: a folder with an empty `justfile`, a stand-in task runner that logs
//! every recipe it is asked to run, a stand-in lock wrapper that logs how it was taken, its own home, stage
//! journal and pass records; `dev-tools` runs there with nothing of the caller's environment but its `PATH`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::Ran;

/// The stand-in task runner: logs the recipe with the selection it was given (tab-separated), skips the desktop stage when told
/// to (exit 77) and reports the accuracy stage's rate when one is set.
const JUST: &str = r#"printf 'just %s\trecipe=%s\tfilter=%s\tscenarios=%s\n' "$*" "$VOICEPUNCHER_STAGES_RECIPE" "$VOICEPUNCHER_RERUN_FILTER" "$VOICEPUNCHER_RERUN_SCENARIOS" >> "$SCRATCH/calls"
case "$3" in
    _it-real) if [ -f "$SCRATCH/skip-it-real" ]; then exit 77; fi ;;
    _e2e-quick) if [ -f "$SCRATCH/wer" ]; then cat "$SCRATCH/wer" >> "$VOICEPUNCHER_STAGES_REPORT"; fi ;;
esac
exit 0"#;

/// The stand-in lock wrapper: logs its lock folder and command line, then runs the command.
const WRAPPER: &str = r#"echo "wrapper $MACHINE_LOCK_DIR $*" >> "$SCRATCH/calls"
if [ "$1" = --shared ]; then shift; fi
exec "$@""#;

#[derive(Debug)]
pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    pub fn create() -> Self {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let scratch = Self { dir };
        for folder in ["repo/.config", "home", "bin"] {
            std::fs::create_dir_all(scratch.path(folder)).expect("a scratch subfolder");
        }
        std::fs::write(scratch.root().join("justfile"), "").expect("a justfile");
        scratch.script("just", JUST);
        scratch.script("wrapper", WRAPPER);
        scratch
    }

    /// A path in the scratch folder, outside the repository.
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// The repository the stage runner works in.
    pub fn root(&self) -> PathBuf {
        self.path("repo")
    }

    /// Writes an executable shell script into the scratch folder.
    pub fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("a script");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        path
    }

    /// Writes a file of the scratch folder.
    pub fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).expect("its folder");
        }
        std::fs::write(&path, text).expect("a scratch file");
    }

    /// Reads a file of the scratch folder, empty when it is missing.
    pub fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).unwrap_or_default()
    }

    /// What the stand-in task runner and lock wrapper logged.
    pub fn calls(&self) -> String {
        self.read("calls")
    }

    /// The stage journal of the scratch runs.
    pub fn journal(&self) -> String {
        self.read("journal.jsonl")
    }

    /// The tables of the stage runner, as the project has them.
    pub fn with_project_tables(&self) {
        let tables = super::repo_file(".config/stages.toml");
        self.write("repo/.config/stages.toml", &tables);
    }

    /// `dev-tools` with these arguments in the repository, the stand-ins in place and nothing of the caller's
    /// environment but its `PATH`, behind the scratch `bin` folder.
    pub fn dev_tools(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dev-tools"));
        command.args(args).current_dir(self.root()).env_clear();
        self.environment(&mut command);
        command
            .env("DEV_TOOLS_JUST", self.path("just"))
            .env("DEV_TOOLS_JUSTFILE", self.root().join("justfile"))
            .env("MACHINE_LOCK", self.path("wrapper"))
            .env("MACHINE_LOCK_DIR", self.path("lock"))
            .env("VOICEPUNCHER_STAGE_JOURNAL", self.path("journal.jsonl"))
            .env("VOICEPUNCHER_STAGE_PASSES", self.path("passes"));
        command
    }

    /// The scratch home, the `PATH` and a git identity.
    fn environment(&self, command: &mut Command) {
        let path = std::env::var("PATH").unwrap_or_default();
        command
            .env("PATH", format!("{}:{path}", self.path("bin").display()))
            .env("HOME", self.path("home"))
            .env("SCRATCH", self.dir.path())
            .env("GIT_AUTHOR_NAME", "scratch")
            .env("GIT_AUTHOR_EMAIL", "scratch@localhost")
            .env("GIT_COMMITTER_NAME", "scratch")
            .env("GIT_COMMITTER_EMAIL", "scratch@localhost");
    }

    /// Runs git in a folder of the scratch; fails the step when git fails.
    pub fn git(&self, folder: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.arg("-C").arg(folder).args(args).env_clear();
        self.environment(&mut command);
        let ran = Ran::of(&mut command);
        assert_eq!(ran.code, 0, "git {args:?}: {}", ran.stderr);
    }

    /// Makes the repository a clone of a scratch origin whose main holds `main_list` as its frozen list, on a
    /// branch of its own.
    pub fn clone_on_branch(&self, branch: &str, main_list: &str) {
        let origin = self.path("origin.git");
        self.git(
            self.dir.path(),
            &["init", "-q", "--bare", "-b", "main", "origin.git"],
        );
        let root = self.root();
        self.write("repo/.config/frozen-tests.toml", main_list);
        self.git(&root, &["init", "-q", "-b", "main"]);
        self.git(&root, &["add", "-A"]);
        self.git(&root, &["commit", "-q", "-m", "main"]);
        self.git(
            &root,
            &["remote", "add", "origin", &origin.to_string_lossy()],
        );
        self.git(&root, &["push", "-q", "origin", "main"]);
        self.git(&root, &["fetch", "-q", "origin"]);
        self.git(&root, &["switch", "-q", "-c", branch]);
    }

    /// Pushes `list` as main's frozen list from another clone.
    pub fn push_to_main(&self, list: &str) {
        let other = self.path("other");
        self.git(self.dir.path(), &["clone", "-q", "origin.git", "other"]);
        self.write("other/.config/frozen-tests.toml", list);
        self.git(&other, &["commit", "-q", "-am", "freeze on main"]);
        self.git(&other, &["push", "-q", "origin", "HEAD:main"]);
    }
}

/// A frozen list's entry, as `just freeze-test` writes it.
pub fn frozen_entry(test: &str, day: &str, stage: &str) -> String {
    format!(
        "[[frozen]]\ntest = \"{test}\"\nfrozen = \"{day}\"\nrecipe = \"full-check\"\nstage = \"{stage}\"\nbeside = \"two other checks\"\nbranch = \"fix/flaky\"\n\n"
    )
}
