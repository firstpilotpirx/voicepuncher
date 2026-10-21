//! Local services as child processes: each one's output goes into its own rotating log file,
//! and a probe asks its port over loopback HTTP whether it answers.

mod probe;

use std::io::{self, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};

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
        if let Some(stdout) = child.stdout.take() {
            copy_into(stdout, Arc::clone(&log));
        }
        if let Some(stderr) = child.stderr.take() {
            copy_into(stderr, log);
        }
        Ok(Box::new(ChildProcess { child, end: None }))
    }
}

/// Copies one output stream into the log on a thread of its own, so a full pipe never
/// blocks the service.
fn copy_into(mut stream: impl Read + Send + 'static, log: Arc<Mutex<RotatingFileWriter>>) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let mut log = log.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let _ = log.write_all(&buffer[..read]);
                    let _ = log.flush();
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    });
}

struct ChildProcess {
    child: Child,
    end: Option<String>,
}

impl ServiceProcess for ChildProcess {
    fn id(&self) -> u32 {
        self.child.id()
    }

    fn exited(&mut self) -> Option<String> {
        if self.end.is_none() {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.end = Some(describe(status));
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
                self.end = Some(describe(status));
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
