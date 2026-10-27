//! Local services as child processes, and a probe that asks a port whether it answers.
//!
//! Each service writes its output into its own log file itself, so the log goes on after
//! the app that started it quits.

mod holder;
mod probe;

use std::io::Write;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};

use anyhow::{Context, Result};

use crate::adapters::secondary::logging::file::{RotatingFileWriter, KEPT_BACKUPS, MAX_LOG_BYTES};
use crate::core::application::{HolderStop, ServiceLauncher, ServiceProcess};
use crate::core::domain::ServiceSpec;

pub use probe::HttpProbe;

/// Starts a service's program in a process group of its own, so a Ctrl-C in the app's
/// terminal reaches the app, which then stops its services in order.
pub struct ProcessLauncher;

impl ServiceLauncher for ProcessLauncher {
    fn spawn(&self, spec: &ServiceSpec) -> Result<Box<dyn ServiceProcess>> {
        let cannot_open = || format!("cannot open the log file {}", spec.log_file.display());
        let mut log = RotatingFileWriter::open(&spec.log_file, MAX_LOG_BYTES, KEPT_BACKUPS)
            .and_then(RotatingFileWriter::into_file)
            .with_context(cannot_open)?;
        let _ = writeln!(
            log,
            "── voicepuncher starts: {} {}",
            spec.program.display(),
            spec.args.join(" ")
        );
        let child = Command::new(&spec.program)
            .args(&spec.args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone().with_context(cannot_open)?))
            .stderr(Stdio::from(log))
            .process_group(0)
            .spawn()
            .with_context(|| format!("cannot start {}", spec.program.display()))?;
        Ok(Box::new(ChildProcess { child, end: None }))
    }

    fn stop_holder(&self, spec: &ServiceSpec) -> HolderStop {
        holder::stop_holder(spec)
    }
}

struct ChildProcess {
    child: Child,
    end: Option<String>,
}

impl ChildProcess {
    fn ended(&mut self, status: ExitStatus) {
        self.end = Some(describe(status));
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
