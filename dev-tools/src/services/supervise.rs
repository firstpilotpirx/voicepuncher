//! `dev-tools supervise <name> <log> <health URL or -> <command…>`: runs a service and starts it again whenever it
//! exits, after 1 s, doubling up to 30 s while it keeps exiting within 30 s of its start. With a health URL, a
//! service that once answered it and then fails it 3 times in a row, `VOICEPUNCHER_HEALTH_CHECK_SECS` apart (default
//! 5 s; the `@resilience` steps set 1 s), is killed and so restarted: a service that hangs without exiting. TERM or
//! INT stops the service and the supervisor, without a restart; HUP is ignored, as under `nohup`.

use super::ready::curl_ok;
use super::{alive, signal};
use crate::context::var;
use crate::process::{exit_code, terminate};
use crate::Outcome;
use rustix::process::Signal;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The longest wait before a restart, and how long a service must have run for the wait to start again at 1 s.
const MOST_SECS: u64 = 30;
/// The health checks in a row a service may fail once it answered.
const MOST_FAILS: u32 = 3;

/// `dev-tools supervise <name> <log> <health URL or -> <command…>`; the name only marks the process.
pub fn command(args: &[String]) -> Outcome {
    let [_, log, health, line @ ..] = args else {
        return Err("usage: dev-tools supervise <name> <log> <health URL or -> <command…>".into());
    };
    if line.is_empty() {
        return Err("supervise: no command".into());
    }
    let stop = stop_flag();
    let supervisor = Supervisor {
        log: PathBuf::from(log),
        health: (health != "-").then(|| health.clone()),
        line,
    };
    supervisor.run(&stop);
    Ok(0)
}

/// Set once TERM or INT reached the supervisor; HUP sets a flag nobody reads, so it no longer stops the process.
fn stop_flag() -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let hangup = Arc::new(AtomicBool::new(false));
    for (signal, flag) in [
        (signal_hook::consts::SIGTERM, &stop),
        (signal_hook::consts::SIGINT, &stop),
        (signal_hook::consts::SIGHUP, &hangup),
    ] {
        // A handler that cannot be installed leaves the default stop, which still ends the supervisor
        let _ = signal_hook::flag::register(signal, Arc::clone(flag));
    }
    stop
}

struct Supervisor<'a> {
    log: PathBuf,
    health: Option<String>,
    line: &'a [String],
}

impl Supervisor<'_> {
    /// Runs the service until TERM or INT, restarting it whenever it exits.
    fn run(&self, stop: &AtomicBool) {
        let mut delay = 1;
        loop {
            let started = Instant::now();
            let Some(code) = self.run_once(stop) else {
                return;
            };
            if started.elapsed() >= Duration::from_secs(MOST_SECS) {
                delay = 1;
            }
            self.note(&exit_line(&self.line[0], code, delay));
            if !pause(stop, Duration::from_secs(delay)) {
                return;
            }
            delay = next_delay(delay);
        }
    }

    /// One run of the service and its exit code; nothing when TERM or INT stopped it.
    fn run_once(&self, stop: &AtomicBool) -> Option<i32> {
        let mut child = match self.spawn() {
            Ok(child) => child,
            Err(error) => {
                self.note(&format!("{}: {error}", self.line[0]));
                return Some(127);
            }
        };
        let done = Arc::new(AtomicBool::new(false));
        if let Some(health) = &self.health {
            let watch = Watch {
                pid: child.id(),
                health: health.clone(),
                log: self.log.clone(),
                name: self.line[0].clone(),
            };
            let done = Arc::clone(&done);
            std::thread::spawn(move || watch.run(&done));
        }
        let code = wait(&mut child, stop);
        done.store(true, Ordering::SeqCst);
        code
    }

    /// Starts the service with its output appended to the log.
    fn spawn(&self) -> std::io::Result<Child> {
        let out = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)?;
        let err = out.try_clone()?;
        Command::new(&self.line[0])
            .args(&self.line[1..])
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .spawn()
    }

    fn note(&self, line: &str) {
        append(&self.log, line);
    }
}

/// The service's exit code; nothing when TERM or INT reached the supervisor, which then stops the service first.
fn wait(child: &mut Child, stop: &AtomicBool) -> Option<i32> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(exit_code(status)),
            Ok(None) => {}
            Err(_) => return Some(1),
        }
        if stop.load(Ordering::SeqCst) {
            terminate(child.id());
            // The service is stopping; its status no longer matters
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Waits that long unless TERM or INT arrives; false when it did.
fn pause(stop: &AtomicBool, length: Duration) -> bool {
    let until = Instant::now() + length;
    while Instant::now() < until {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !stop.load(Ordering::SeqCst)
}

/// The log line of an exit.
pub fn exit_line(name: &str, code: i32, delay: u64) -> String {
    format!("… {name} exited with code {code} — restarting in {delay} s")
}

/// The wait before the next restart: twice this one, at most 30 s.
pub fn next_delay(delay: u64) -> u64 {
    (delay * 2).min(MOST_SECS)
}

/// Appends a line to the log.
fn append(log: &Path, line: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) {
        // A log that cannot be written must not stop the supervisor
        let _ = writeln!(file, "{line}");
    }
}

/// The health watch of one run of a service.
struct Watch {
    pid: u32,
    health: String,
    log: PathBuf,
    name: String,
}

impl Watch {
    /// Checks the health URL while the service runs; kills a service that answered once and then failed
    /// `MOST_FAILS` checks in a row.
    fn run(&self, done: &AtomicBool) {
        let every = health_interval();
        let mut fails = Fails::default();
        while !done.load(Ordering::SeqCst) && alive(self.pid) {
            std::thread::sleep(every);
            if done.load(Ordering::SeqCst) {
                return;
            }
            if fails.record(curl_ok(&self.health, "2")) {
                append(&self.log, &health_line(&self.name, fails.count));
                signal(self.pid, Signal::KILL);
                return;
            }
        }
    }
}

/// The health checks a service failed in a row since it last answered, counted only once it answered.
#[derive(Debug, Default)]
pub struct Fails {
    answered: bool,
    pub count: u32,
}

impl Fails {
    /// Records one check; true when the service is to be killed.
    pub fn record(&mut self, answered: bool) -> bool {
        if answered {
            self.answered = true;
            self.count = 0;
        } else if self.answered {
            self.count += 1;
        }
        self.count >= MOST_FAILS
    }
}

/// The log line of a service killed for its health checks.
pub fn health_line(name: &str, fails: u32) -> String {
    format!("… {name} stopped answering its health check ({fails} checks in a row) — restarting")
}

/// `VOICEPUNCHER_HEALTH_CHECK_SECS`, 5 s unless set.
fn health_interval() -> Duration {
    var("VOICEPUNCHER_HEALTH_CHECK_SECS")
        .and_then(|secs| secs.parse::<f64>().ok())
        .and_then(|secs| Duration::try_from_secs_f64(secs).ok())
        .unwrap_or(Duration::from_secs(5))
}
