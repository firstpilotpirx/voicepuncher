//! `dev-tools stop-model-servers [service…]`: stops the services `start-model-servers` launched (all, or the
//! named ones); services started by hand keep running.

use super::launch::remove;
use super::{alive, file_text, outcome, say, selected, signal, unknown, Settings, Step};
use crate::process::stdout;
use crate::Outcome;
use rustix::process::Signal;
use std::io::Write;
use std::process::Command;
use std::time::Duration;

/// `dev-tools stop-model-servers [service…]`.
pub fn command(args: &[String]) -> Outcome {
    let settings = Settings::from_env()?;
    outcome(stop(&settings, args, &mut std::io::stdout()))
}

/// Stops the named services, or every one, writing what it did to `out`.
pub fn stop(settings: &Settings, services: &[String], out: &mut dyn Write) -> Step {
    for service in selected(services) {
        match service.as_str() {
            "whisper-server" => stop_speech_recognition(
                settings,
                settings.dictation.port,
                settings.dictation.label,
                out,
            ),
            "whisper-server-commands" => {
                stop_speech_recognition(
                    settings,
                    settings.commands.port,
                    settings.commands.label,
                    out,
                );
            }
            "llama-server" => stop_service(
                settings,
                ["llama-server", "text refinement service"],
                settings.llm_port,
                out,
            ),
            "voicepuncher-command-spotting" => stop_service(
                settings,
                ["voicepuncher-command-spotting", "command spotting service"],
                settings.kws_port,
                out,
            ),
            _ => return Err(unknown(&service)),
        }
    }
    Ok(())
}

/// The copies of the speech recognition service on that port when it runs as several, or the one
/// `whisper-server` on its port.
fn stop_speech_recognition(
    settings: &Settings,
    service_port: u32,
    label: &str,
    out: &mut dyn Write,
) {
    let list = settings.copies_file(service_port);
    let Ok(ports) = std::fs::read_to_string(&list) else {
        stop_service(settings, ["whisper-server", label], service_port, out);
        return;
    };
    for port in ports.split_whitespace() {
        let copy = format!("{label}: copy on port {port}");
        match port.parse() {
            Ok(port) => stop_service(settings, ["whisper-server", &copy], port, out),
            Err(_) => say(out, &format!("· no {copy} started by start-model-servers")),
        }
    }
    remove(&list);
}

/// Stops the supervisor a PID file names (TERM, then KILL for it and its children after 10 s), and removes the
/// file. The PID is the service's supervisor, or the service itself when an older `justfile` started it; either has
/// the binary in its command line, so a PID reused by another program is left alone.
fn stop_service(settings: &Settings, [binary, label]: [&str; 2], port: u32, out: &mut dyn Write) {
    let pid_file = settings.pid_file(binary, port);
    if !pid_file.is_file() {
        say(out, &format!("· no {label} started by start-model-servers"));
        return;
    }
    let pid = file_text(&pid_file).unwrap_or_default();
    match pid.parse::<u32>() {
        Ok(number) if command_line(&pid).contains(binary) => {
            stop_process(number);
            say(out, &format!("✓ stopped {binary} (pid {pid})"));
        }
        _ => say(out, &format!("· {binary} (pid {pid}) is no longer running")),
    }
    remove(&pid_file);
}

/// The command line of a process, empty when it is gone.
pub fn command_line(pid: &str) -> String {
    stdout(Command::new("ps").args(["-p", pid, "-o", "command="]))
}

/// TERM, up to 10 s for the process to end, then KILL for its children and itself.
fn stop_process(pid: u32) {
    signal(pid, Signal::TERM);
    for _ in 0..40 {
        if !alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    // Children that are gone already need no KILL
    let _ = Command::new("pkill")
        .args(["-9", "-P", &pid.to_string()])
        .status();
    signal(pid, Signal::KILL);
}
