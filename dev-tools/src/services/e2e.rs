//! `dev-tools model-servers-for-e2e start|stop`: every service an E2E recipe tests, on the test ports the machine
//! lock gives it, never reusing one it did not start: a service someone else runs may hold another model or other
//! code. A service a killed locked run left there (its PID file names it) is stopped and started again; any other
//! process on a test port fails the run before it starts anything. Inside check and full-check, which hold the
//! services for all their stages (`VOICEPUNCHER_E2E_SERVICES_HELD`), both do nothing.

use super::start::start;
use super::stop::{command_line, stop};
use super::{basename, copy_ports, file_text, outcome, Settings, Speech, Step, Stop};
use crate::context::var;
use crate::process::stdout;
use crate::Outcome;
use std::process::Command;

/// `dev-tools model-servers-for-e2e start|stop`.
pub fn command(args: &[String]) -> Outcome {
    let held = var("VOICEPUNCHER_E2E_SERVICES_HELD").is_some();
    match args {
        [what] if what == "start" => start_command(held),
        [what] if what == "stop" => stop_command(held),
        _ => Err("usage: dev-tools model-servers-for-e2e start|stop".into()),
    }
}

/// `start`: nothing while check or full-check holds the services.
fn start_command(held: bool) -> Outcome {
    if held {
        return Ok(0);
    }
    outcome(start_for_e2e(&Settings::from_env()?))
}

/// `stop`: nothing while check or full-check holds the services, nor off the test ports.
fn stop_command(held: bool) -> Outcome {
    if held || var("VOICEPUNCHER_TEST_PORTS").is_none() {
        return Ok(0);
    }
    outcome(stop(&Settings::from_env()?, &[], &mut std::io::stdout()))
}

/// Stops what a killed locked run left, checks that every test port is free, starts every service and names what
/// this run tests.
fn start_for_e2e(settings: &Settings) -> Step {
    let Some(prefix) = var("VOICEPUNCHER_TEST_PORTS") else {
        return Err(Stop::Message(
            "_start-model-servers-for-e2e runs only under the machine lock, on the test ports — run an E2E recipe \
             or check"
                .into(),
        ));
    };
    // Left by a killed locked run: the stop stops only what a PID file on these ports names; its failure, as its
    // lines for services that were not there, is no news here
    let mut stopped = Vec::new();
    let _ = stop(settings, &[], &mut stopped);
    print!("{}", unlisted(&String::from_utf8_lossy(&stopped)));
    let held: Vec<String> = watched_ports(settings, &prefix)
        .iter()
        .filter_map(holder)
        .collect();
    for line in &held {
        eprintln!("✗ {line}");
    }
    if !held.is_empty() {
        return Err(Stop::Code(1));
    }
    start(settings, &[])?;
    for line in testing_lines(settings) {
        println!("{line}");
    }
    Ok(())
}

/// The lines of the stop's output that do not start with `·`.
pub fn unlisted(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with('·'))
        .flat_map(|line| [line, "\n"])
        .collect()
}

/// A test port, what it is for and the variable that moves it.
#[derive(Debug, PartialEq, Eq)]
pub struct Watched {
    pub port: u32,
    pub service: String,
    pub variable: String,
}

/// Every port the services of a run on the test ports listen on, copies included.
pub fn watched_ports(settings: &Settings, prefix: &str) -> Vec<Watched> {
    let watched = |port: u32, service: &str, variable: &str| Watched {
        port,
        service: service.to_string(),
        variable: format!("{prefix}_{variable}"),
    };
    let mut ports = Vec::new();
    for (speech, variable) in [
        (&settings.dictation, "STT_PORT"),
        (&settings.commands, "COMMAND_STT_PORT"),
    ] {
        ports.push(watched(speech.port, speech.name, variable));
        let copy = format!("{} copy", speech.name);
        ports.extend(
            copies(speech)
                .into_iter()
                .map(|port| watched(port, &copy, variable)),
        );
    }
    ports.push(watched(
        settings.kws_port,
        "voicepuncher-command-spotting",
        "KWS_PORT",
    ));
    ports
}

/// The copy ports of a speech recognition service that runs as copies, none for one that runs as one.
fn copies(speech: &Speech) -> Vec<u32> {
    match speech.copies.parse::<u32>() {
        Ok(count) if count > 1 => copy_ports(speech.port, count),
        _ => Vec::new(),
    }
}

/// The message for a port some process listens on, nothing for a free one.
fn holder(watched: &Watched) -> Option<String> {
    let port = format!("-iTCP:{}", watched.port);
    let out = stdout(Command::new("lsof").args(["-nP", &port, "-sTCP:LISTEN", "-t"]));
    let pid = out.lines().next().filter(|pid| !pid.is_empty())?;
    let line = command_line(pid);
    Some(format!(
        "port {} ({}) is held by pid {pid}: {} — stop it or set {}",
        watched.port,
        watched.service,
        line.trim_end_matches('\n'),
        watched.variable
    ))
}

/// What this run tests: one line per service, its port, model and the PIDs of what it started.
pub fn testing_lines(settings: &Settings) -> Vec<String> {
    let mut lines: Vec<String> = [&settings.dictation, &settings.commands]
        .into_iter()
        .map(|speech| testing_copies(settings, speech))
        .collect();
    let (port, binary) = (settings.kws_port, "voicepuncher-command-spotting");
    let model = basename(&std::env::var("VOICEPUNCHER_KWS_MODEL_DIR").unwrap_or_default());
    let pid = pid_of(settings, binary, port);
    lines.push(format!(
        "● testing {binary} on port {port}: {model}, pid {pid}"
    ));
    lines
}

/// The testing line of a speech recognition service, one PID per copy.
fn testing_copies(settings: &Settings, speech: &Speech) -> String {
    let mut ports = copies(speech);
    if ports.is_empty() {
        ports.push(speech.port);
    }
    let pids: Vec<String> = ports
        .iter()
        .map(|port| pid_of(settings, "whisper-server", *port))
        .collect();
    let ports: Vec<String> = ports.iter().map(u32::to_string).collect();
    let model = basename(&std::env::var(speech.model_var).unwrap_or_default());
    format!(
        "● testing {} on port {}, copies on {}: {model}, pids {}",
        speech.name,
        speech.port,
        ports.join(" "),
        pids.join(" ")
    )
}

/// The PID a PID file holds, `?` without one.
fn pid_of(settings: &Settings, binary: &str, port: u32) -> String {
    file_text(&settings.pid_file(binary, port)).unwrap_or_else(|| "?".into())
}
