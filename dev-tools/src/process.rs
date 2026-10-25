//! Running other programs: their output, `git` in a directory, and the stop signals this process receives.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The standard output of a command, whatever its status (as the former Perl's backticks); empty when it
/// cannot start.
pub fn stdout(command: &mut Command) -> String {
    command
        .stderr(Stdio::null())
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

/// The standard output of a command that succeeded.
pub fn success_stdout(command: &mut Command) -> Option<String> {
    let out = command.stderr(Stdio::null()).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs a command with this process's output; its exit code (1 when it cannot start, 128 + the signal
/// when one stopped it).
pub fn status(command: &mut Command) -> i32 {
    command.status().map_or(1, exit_code)
}

pub fn exit_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
}

/// `git -C <dir> <args>`.
pub fn git(dir: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir);
    command
}

/// Runs `git` in `dir` with `args` and fails the test when it fails.
#[cfg(test)]
pub fn git_ok(dir: &Path, args: &[&str]) {
    let ok = git(dir).args(args).output().unwrap().status.success();
    assert!(ok, "git {args:?}");
}

/// One line of `git` output without its newline, or nothing when it printed nothing.
pub fn git_line(dir: &Path, args: &[&str]) -> Option<String> {
    let out = stdout(git(dir).args(args));
    let line = out.strip_suffix('\n').unwrap_or(&out);
    (!line.is_empty()).then(|| line.to_string())
}

/// The first executable of that name on the `PATH`, as `command -v` finds it.
pub fn find_program(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|file| {
            std::fs::metadata(file)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// The exit code a byte-sized `ExitCode` can carry.
pub fn code(value: i32) -> u8 {
    u8::try_from(value.clamp(0, 255)).unwrap_or(1)
}

/// Set once TERM, INT or HUP reached this process; the handlers replace the default stop, so the process
/// can stop its children first.
#[derive(Clone)]
pub struct Interrupt(Arc<AtomicBool>);

impl Interrupt {
    pub fn watch() -> Self {
        let flag = Arc::new(AtomicBool::new(false));
        for signal in [
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGHUP,
        ] {
            // A handler that cannot be installed leaves the default stop, which still ends the run
            let _ = signal_hook::flag::register(signal, Arc::clone(&flag));
        }
        Self(flag)
    }

    pub fn raised(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Sends TERM to a process.
pub fn terminate(pid: u32) {
    if let Some(pid) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    {
        // A process that is gone already needs no signal
        let _ = rustix::process::kill_process(pid, rustix::process::Signal::TERM);
    }
}
