//! The program listening on a service's port: stopped when it is the service's own program, so
//! a hung copy the app did not start can be replaced.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use voicepuncher_copy_pool::holder_of;

use crate::core::application::HolderStop;
use crate::core::domain::ServiceSpec;

/// `ps` and `kill` ship with macOS here; the paths keep a changed `$PATH` from picking others.
const PS: &str = "/bin/ps";
const KILL: &str = "/bin/kill";
/// How long a killed program may take to be gone.
const GONE_LIMIT: Duration = Duration::from_secs(2);

pub(super) fn stop_holder(spec: &ServiceSpec) -> HolderStop {
    let holder = holder_of(spec.port);
    let Some(pid) = holder.pid.filter(|&pid| runs_program(pid, &spec.program)) else {
        return HolderStop::Foreign(holder.to_string());
    };
    let _ = Command::new(KILL).args(["-9", &pid.to_string()]).status();
    let deadline = Instant::now() + GONE_LIMIT;
    while alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    HolderStop::Stopped(pid)
}

/// Whether a word of the command line of `pid` names the file of `program`.
fn runs_program(pid: u32, program: &Path) -> bool {
    let Some(name) = program.file_name() else {
        return false;
    };
    let output = Command::new(PS)
        .args(["-o", "command=", "-p", &pid.to_string()])
        .output();
    let line = output.map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    line.unwrap_or_default()
        .split_whitespace()
        .any(|word| Path::new(word).file_name() == Some(name))
}

fn alive(pid: u32) -> bool {
    Command::new(KILL)
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}
