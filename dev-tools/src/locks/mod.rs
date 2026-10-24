//! The machine-wide lock (agent-kit `with-machine-lock`, `MACHINE_LOCK` or `~/.claude/bin/with-machine-lock`;
//! none with `VOICEPUNCHER_NO_MACHINE_LOCK=1`, the hosted CI) and the two ways the stages take it:
//!
//! - the e2e lock, for the model stages of a run: the machine lock exclusively (the outer wrapper, so the kit
//!   keeps every other run's model stages and every exclusive run out), then the quiet lock exclusively (the
//!   inner wrapper, with the stage's deadline), so the other runs' build and test stages end before the first
//!   model stage starts and none starts beside them. Machine lock first: holding the quiet lock while queuing
//!   for the machine lock would stall every other check's builds for the queue's length. The outer deadline
//!   covers the wait for the quiet lock (`MACHINE_LOCK_WAIT`, 6 h) plus the inner deadline;
//! - the quiet lock in the shared mode, for a build or test stage: a lock directory beside the machine lock's
//!   that only the model stages take exclusively, so builds and tests start at once beside each other but never
//!   beside another run's model stages. The wrapper also gives the stage its time limit (`MACHINE_LOCK_TIMEOUT`,
//!   exit 124).

#[cfg(test)]
mod tests;

use crate::context::{home, var, Context};
use crate::process::{code, find_program, status};
use crate::Outcome;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

/// The lock's wait, unless the caller sets `MACHINE_LOCK_WAIT`: 6 h, so a run waits out a queue of several
/// sessions' checks instead of giving up.
pub const DEFAULT_WAIT: u64 = 21_600;
/// The stage deadline, unless the caller sets `MACHINE_LOCK_TIMEOUT`.
const DEFAULT_DEADLINE: u64 = 3600;
/// The test ports every locked run starts its services on.
pub const TEST_PORTS: [(&str, &str, &str); 4] = [
    (
        "VOICEPUNCHER_STT_PORT",
        "VOICEPUNCHER_TEST_STT_PORT",
        "28081",
    ),
    (
        "VOICEPUNCHER_COMMAND_STT_PORT",
        "VOICEPUNCHER_TEST_COMMAND_STT_PORT",
        "28085",
    ),
    (
        "VOICEPUNCHER_LLM_PORT",
        "VOICEPUNCHER_TEST_LLM_PORT",
        "28080",
    ),
    (
        "VOICEPUNCHER_KWS_PORT",
        "VOICEPUNCHER_TEST_KWS_PORT",
        "28082",
    ),
];

/// How to install the lock, for a clone that has none.
pub const INSTALL: &str = "git clone git@github.com:firstpilotpirx/agent-kit.git && go -C agent-kit/claude-code/kit run ./cmd/kit install";

/// The lock settings of an environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lock {
    /// The wrapper, or none when the recipes run without the lock.
    pub wrapper: Option<PathBuf>,
    pub dir: PathBuf,
    pub wait: u64,
    pub deadline: u64,
}

impl Lock {
    pub fn from_env() -> Self {
        Self::of(&var)
    }

    pub fn of(lookup: &dyn Fn(&str) -> Option<String>) -> Self {
        let wrapper = (lookup("VOICEPUNCHER_NO_MACHINE_LOCK").as_deref() != Some("1")).then(|| {
            lookup("MACHINE_LOCK").map_or_else(
                || home().join(".claude/bin/with-machine-lock"),
                PathBuf::from,
            )
        });
        let number = |name: &str, default: u64| {
            lookup(name)
                .and_then(|value| value.parse().ok())
                .unwrap_or(default)
        };
        Self {
            wrapper,
            dir: lookup("MACHINE_LOCK_DIR")
                .map_or_else(|| home().join(".cache/machine-lock"), PathBuf::from),
            wait: number("MACHINE_LOCK_WAIT", DEFAULT_WAIT),
            deadline: number("MACHINE_LOCK_TIMEOUT", DEFAULT_DEADLINE),
        }
    }

    fn quiet_dir(&self) -> OsString {
        let mut dir = self.dir.clone().into_os_string();
        dir.push("-voicepuncher-quiet");
        dir
    }

    /// The command line that runs `command` under the e2e lock.
    pub fn e2e(&self, command: &[OsString]) -> Vec<OsString> {
        let Some(wrapper) = &self.wrapper else {
            return command.to_vec();
        };
        let mut line: Vec<OsString> = vec!["env".into(), dir_setting(self.dir.as_os_str())];
        line.push(format!("MACHINE_LOCK_TIMEOUT={}", self.wait + self.deadline).into());
        line.extend([
            wrapper.clone().into_os_string(),
            "env".into(),
            dir_setting(&self.quiet_dir()),
        ]);
        line.push(format!("MACHINE_LOCK_TIMEOUT={}", self.deadline).into());
        line.push(wrapper.clone().into_os_string());
        line.extend(command.iter().cloned());
        line
    }

    /// The command line that runs `command` under the quiet lock in the shared mode.
    pub fn quiet(&self, command: &[OsString]) -> Vec<OsString> {
        let Some(wrapper) = &self.wrapper else {
            return command.to_vec();
        };
        let mut line = vec![
            "env".into(),
            dir_setting(&self.quiet_dir()),
            wrapper.clone().into_os_string(),
            "--shared".into(),
        ];
        line.extend(command.iter().cloned());
        line
    }

    /// The command line that runs `command` under the machine lock exclusively.
    pub fn machine(&self, command: &[OsString]) -> Vec<OsString> {
        let Some(wrapper) = &self.wrapper else {
            return command.to_vec();
        };
        let mut line = vec![
            "env".into(),
            dir_setting(self.dir.as_os_str()),
            wrapper.clone().into_os_string(),
        ];
        line.extend(command.iter().cloned());
        line
    }

    /// Whether the caller already holds the machine lock (it exports the lock file it holds).
    pub fn held_by_caller(&self) -> bool {
        var("MACHINE_LOCK_HELD")
            .is_some_and(|held| PathBuf::from(held) == self.dir.join("tests.lock"))
    }

    /// Fails naming how to install the lock when the wrapper is missing.
    pub fn require(&self) -> Result<(), String> {
        let Some(wrapper) = &self.wrapper else {
            return Ok(());
        };
        if wrapper.is_file()
            || (!wrapper.is_absolute() && find_program(&wrapper.to_string_lossy()).is_some())
        {
            return Ok(());
        }
        Err(format!(
            "no machine lock at {}: install agent-kit ({INSTALL}), set MACHINE_LOCK to the lock's wrapper, or run without the lock with VOICEPUNCHER_NO_MACHINE_LOCK=1",
            wrapper.display()
        ))
    }
}

fn dir_setting(dir: &std::ffi::OsStr) -> OsString {
    let mut setting = OsString::from("MACHINE_LOCK_DIR=");
    setting.push(dir);
    setting
}

/// The test ports as `NAME=port` settings, each overridable by its `VOICEPUNCHER_TEST_*_PORT`.
pub fn test_ports() -> Vec<(String, String)> {
    let mut ports: Vec<(String, String)> = TEST_PORTS
        .iter()
        .map(|(name, test, port)| {
            (
                (*name).to_string(),
                var(test).unwrap_or_else(|| (*port).to_string()),
            )
        })
        .collect();
    ports.push(("VOICEPUNCHER_TEST_PORTS".into(), "VOICEPUNCHER_TEST".into()));
    ports
}

/// A command from a command line, on the test ports.
pub fn command_on_test_ports(line: &[OsString]) -> Command {
    let mut command = Command::new(&line[0]);
    command.args(&line[1..]).envs(test_ports());
    command
}

/// `dev-tools locked <recipe> [args]`: runs a recipe of the same `justfile` on the test ports under the machine
/// lock exclusively.
pub fn locked_command(args: &[String]) -> Outcome {
    let lock = Lock::from_env();
    lock.require()?;
    let context = Context::from_env();
    let mut line: Vec<OsString> = vec![
        context.just.clone().into(),
        "--justfile".into(),
        context.justfile.into(),
    ];
    line.extend(args.iter().map(OsString::from));
    Ok(code(status(&mut command_on_test_ports(
        &lock.machine(&line),
    ))))
}

/// `dev-tools require-lock`: fails naming the install command when the lock is missing.
pub fn require_command(_: &[String]) -> Outcome {
    Lock::from_env().require()?;
    Ok(0)
}
