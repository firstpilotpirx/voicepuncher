//! Running one phase of a stage run: a child process whose output also goes to the run's log, stopped with TERM
//! when this process is asked to stop.

use crate::process::{exit_code, terminate, Interrupt};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;

/// Copies a child's stream to this process's (stdout or stderr) and appends it to the log.
fn copy(mut from: impl Read + Send + 'static, log: PathBuf, to_stderr: bool) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut file = OpenOptions::new().create(true).append(true).open(&log).ok();
        let mut buffer = [0_u8; 8192];
        while let Ok(read) = from.read(&mut buffer) {
            if read == 0 {
                break;
            }
            let chunk = &buffer[..read];
            // A closed terminal or log must not stop the copy: the child would block on a full pipe
            if to_stderr {
                let _ = std::io::stderr().write_all(chunk);
            } else {
                let _ = std::io::stdout().write_all(chunk);
                let _ = std::io::stdout().flush();
            }
            if let Some(file) = file.as_mut() {
                let _ = file.write_all(chunk);
            }
        }
    })
}

/// Runs a command to its end and returns its exit code; with a log, its output is copied there too. When this
/// process is asked to stop, the child gets TERM once and the run waits for it.
pub fn run(command: &mut Command, interrupt: &Interrupt, log: Option<&Path>) -> i32 {
    if log.is_some() {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("✗ {}: {error}", command.get_program().to_string_lossy());
            return 127;
        }
    };
    let copies = log.map_or_else(Vec::new, |log| copies(&mut child, log));
    let code = wait(&mut child, interrupt);
    for copy in copies {
        let _ = copy.join();
    }
    code
}

/// Waits for `child` to end and returns its exit code; TERM once when this process is asked
/// to stop.
fn wait(child: &mut std::process::Child, interrupt: &Interrupt) -> i32 {
    let mut told = false;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return exit_code(status),
            Ok(None) => {}
            Err(_) => return 1,
        }
        if interrupt.raised() && !told {
            terminate(child.id());
            told = true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Copies the child's output and errors to the log, each on a thread of its own.
fn copies(child: &mut std::process::Child, log: &Path) -> Vec<std::thread::JoinHandle<()>> {
    let out = child
        .stdout
        .take()
        .map(|out| copy(out, log.to_path_buf(), false));
    let err = child
        .stderr
        .take()
        .map(|err| copy(err, log.to_path_buf(), true));
    out.into_iter().chain(err).collect()
}
