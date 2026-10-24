//! Starting one service under its supervisor and waiting until it is ready: the PID file holds the supervisor's
//! PID, the log the service's output and the supervisor's restarts.

use super::ready::Ready;
use super::{alive, Settings, Step, Stop};
use crate::process::terminate;
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// How many readiness checks a start makes, 0.25 s apart (60 s without the checks' own time).
const CHECKS: u32 = 240;

/// One service to start: its binary (named in the PID and log files), port, how its messages call it, its
/// readiness check, its health URL for the supervisor and its command line.
pub struct Service {
    pub binary: String,
    pub port: u32,
    pub label: String,
    pub ready: Ready,
    pub health: Option<String>,
    pub command: Vec<String>,
}

/// The supervisor's arguments after `dev-tools`: its name, the log, the health URL or `-`, the command line.
pub fn supervisor_args(service: &Service, log: &Path) -> Vec<String> {
    let mut args = vec![
        "supervise".to_string(),
        format!("supervise-{}", service.binary),
        log.display().to_string(),
        service.health.clone().unwrap_or_else(|| "-".into()),
    ];
    args.extend(service.command.iter().cloned());
    args
}

/// Starts the service under its supervisor, in a process group of its own so the terminal's signals to this
/// command do not reach it, and waits until it is ready.
pub fn start(settings: &Settings, service: &Service) -> Step {
    let pid_file = settings.pid_file(&service.binary, service.port);
    let log = settings.log_file(&service.binary, service.port);
    File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
    let program = std::env::current_exe().map_err(|e| format!("dev-tools: {e}"))?;
    let mut child = Command::new(program)
        .args(supervisor_args(service, &log))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("supervise-{}: {e}", service.binary))?;
    let pid = child.id();
    // Reaps the supervisor when it ends while this command still runs, so a stopped one is not a live zombie
    std::thread::spawn(move || child.wait());
    std::fs::write(&pid_file, format!("{pid}\n"))
        .map_err(|e| format!("{}: {e}", pid_file.display()))?;
    println!(
        "… starting {} (pid {pid}), log: {}",
        service.binary,
        log.display()
    );
    wait_ready(service, pid, &log, &pid_file)
}

/// Waits until the service is ready; a service that exits before it was ever ready cannot start: its supervisor
/// is stopped instead of letting it retry.
fn wait_ready(service: &Service, pid: u32, log: &Path, pid_file: &Path) -> Step {
    for _ in 0..CHECKS {
        if let Some(note) = service.ready.check() {
            println!("✓ {} ready on port {}{note}", service.label, service.port);
            return Ok(());
        }
        if !alive(pid) || exited(log) {
            terminate(pid);
            eprintln!(
                "✗ {} exited during startup (log: {}) — last log lines:",
                service.binary,
                log.display()
            );
            eprint!("{}", tail(log, 20));
            remove(pid_file);
            return Err(Stop::Code(1));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!(
        "✗ {} was not ready on port {} within 60 s — see {}",
        service.binary,
        service.port,
        log.display()
    );
    terminate(pid);
    remove(pid_file);
    Err(Stop::Code(1))
}

/// Whether the supervisor logged an exit of the service.
fn exited(log: &Path) -> bool {
    std::fs::read(log)
        .is_ok_and(|bytes| String::from_utf8_lossy(&bytes).contains(" exited with code "))
}

/// The last lines of a file, as `tail -n` prints them.
pub fn tail(path: &Path, lines: usize) -> String {
    let text = std::fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    last_lines(&text, lines)
}

/// The last lines of a text, each with its newline.
pub fn last_lines(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    let from = all.len().saturating_sub(lines);
    all[from..].iter().flat_map(|line| [*line, "\n"]).collect()
}

/// Removes a file that may be gone.
pub fn remove(path: &Path) {
    // A file that is gone already is what the removal wants
    let _ = std::fs::remove_file(path);
}
