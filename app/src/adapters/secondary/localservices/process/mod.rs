//! Local services as child processes: each one's output goes into its own rotating log file,
//! and a probe asks its port over loopback HTTP whether it answers.

mod probe;

use std::io::{self, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::adapters::secondary::logging::file::{RotatingFileWriter, KEPT_BACKUPS, MAX_LOG_BYTES};
use crate::core::application::{ServiceLauncher, ServiceProcess};
use crate::core::domain::ServiceSpec;

pub use probe::HttpProbe;

/// Starts a service's program in a process group of its own, so a Ctrl-C in the app's
/// terminal reaches the app, which then stops its services in order.
pub struct ProcessLauncher;

impl ServiceLauncher for ProcessLauncher {
    fn spawn(&self, spec: &ServiceSpec) -> Result<Box<dyn ServiceProcess>> {
        let mut log = RotatingFileWriter::open(&spec.log_file, MAX_LOG_BYTES, KEPT_BACKUPS)
            .with_context(|| format!("cannot open the log file {}", spec.log_file.display()))?;
        let _ = writeln!(
            log,
            "── voicepuncher starts: {} {}",
            spec.program.display(),
            spec.args.join(" ")
        );
        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .with_context(|| format!("cannot start {}", spec.program.display()))?;
        let log = Arc::new(Mutex::new(log));
        let mut copies = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            copies.push(copy_into(stdout, Arc::clone(&log)));
        }
        if let Some(stderr) = child.stderr.take() {
            copies.push(copy_into(stderr, log));
        }
        Ok(Box::new(ChildProcess::new(child, copies)))
    }
}

/// Copies one output stream into the log on a thread of its own, so a full pipe never
/// blocks the service.
fn copy_into(
    mut stream: impl Read + Send + 'static,
    log: Arc<Mutex<RotatingFileWriter>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let mut log = log
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let _ = log.write_all(&buffer[..read]);
                    let _ = log.flush();
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    })
}

/// How long the end of a service waits for each of its output copies: a child of the service
/// could still hold the pipe open, and then the copy is left to end on its own.
const COPY_JOIN_LIMIT: Duration = Duration::from_secs(1);

struct ChildProcess {
    child: Child,
    end: Option<String>,
    /// The threads copying the service's output into its log; joined once the service ended.
    copies: Vec<JoinHandle<()>>,
}

impl ChildProcess {
    fn new(child: Child, copies: Vec<JoinHandle<()>>) -> Self {
        Self { child, end: None, copies }
    }

    fn ended(&mut self, status: ExitStatus) {
        self.end = Some(describe(status));
        for copy in self.copies.drain(..) {
            join_within(copy, COPY_JOIN_LIMIT);
        }
    }
}

/// Joins `thread` when it ends within `limit`; otherwise leaves it running on its own.
fn join_within(thread: JoinHandle<()>, limit: Duration) {
    let deadline = Instant::now() + limit;
    while !thread.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if thread.is_finished() && thread.join().is_err() {
        log::warn!("service output copy panicked");
    }
}

impl ServiceProcess for ChildProcess {
    fn id(&self) -> u32 {
        self.child.id()
    }

    fn exited(&mut self) -> Option<String> {
        if self.end.is_none() {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.ended(status);
            }
        }
        self.end.clone()
    }

    fn terminate(&mut self) {
        if self.exited().is_none() {
            if let Ok(pid) = i32::try_from(self.child.id()) {
                // SAFETY: kill(2) on the pid of a child this process has not reaped yet.
                unsafe { libc::kill(pid, libc::SIGTERM) };
            }
        }
    }

    fn kill(&mut self) {
        if self.exited().is_none() {
            let _ = self.child.kill();
            if let Ok(status) = self.child.wait() {
                self.ended(status);
            }
        }
    }
}

fn describe(status: ExitStatus) -> String {
    match (status.code(), status.signal()) {
        (Some(code), _) => format!("code {code}"),
        (None, Some(signal)) => format!("signal {signal}"),
        (None, None) => "unknown end".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn sleeping_child() -> Child {
        Command::new("/bin/sleep").arg("30").spawn().unwrap()
    }

    #[test]
    fn should_join_the_output_copies_when_the_process_is_killed() {
        // Given — a copy that still writes for a moment after the process ends
        let copied = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&copied);
        let copy = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            flag.store(true, Ordering::SeqCst);
        });
        let mut process = ChildProcess::new(sleeping_child(), vec![copy]);
        // When
        process.kill();
        // Then
        assert!(
            copied.load(Ordering::SeqCst),
            "kill returned before its output was copied"
        );
    }

    #[test]
    fn should_stop_waiting_for_a_copy_that_never_ends() {
        // Given — a copy held open, as by a child of the service that keeps its pipe
        let copy = std::thread::spawn(|| std::thread::sleep(Duration::from_secs(30)));
        let mut process = ChildProcess::new(sleeping_child(), vec![copy]);
        // When
        let started = Instant::now();
        process.kill();
        // Then
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "kill waited {:?}",
            started.elapsed()
        );
    }
}
