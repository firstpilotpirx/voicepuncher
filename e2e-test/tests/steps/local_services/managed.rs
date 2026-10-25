//! The app's own supervisor run in the test process on ports of the scenario's own: the
//! services it starts as copies, the processes a step kills or pauses by port, and the log
//! lines the clients wrote meanwhile.

use std::sync::OnceLock;
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::secondary::localservices::catalog::Machine;
use voicepuncher::config::Config;
use voicepuncher::core::domain::{ServiceKind, ServiceStatus};
use voicepuncher::runtime::services::{LocalServices, ManagedServices, ServicesView};
use voicepuncher_copy_pool::{
    ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET, SPEECH_RECOGNITION_COPY_OFFSET,
};

use super::{answers, free_port, pgrep, signal, wait_until, workspace_root};

/// Loading the models of every copy takes up to a minute on a busy machine.
const READY_WITHIN: Duration = Duration::from_secs(240);
/// The programs the app starts, as their command lines begin.
const PROGRAMS: &str = "(whisper-server|voicepuncher-command-spotting)";

/// Which services a scenario needs, and as how many copies.
#[derive(Debug, Clone, Copy)]
pub struct Needed {
    pub dictation_copies: u32,
    pub voice_control: bool,
}

/// The services the app's supervisor started for one scenario; dropping it stops them.
pub struct ManagedRun {
    services: ManagedServices,
    view: std::sync::Arc<dyn ServicesView>,
    pub config: Config,
    home: tempfile::TempDir,
    paused: Vec<u32>,
}

impl std::fmt::Debug for ManagedRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedRun")
            .field("statuses", &self.view.statuses())
            .finish_non_exhaustive()
    }
}

impl Drop for ManagedRun {
    fn drop(&mut self) {
        for pid in &self.paused {
            let _ = signal(*pid, "-CONT");
        }
        self.services.stop();
    }
}

impl ManagedRun {
    /// Starts what `needed` names on free ports, from the models the `justfile` exports,
    /// and waits until every copy answers.
    pub fn start(needed: Needed) -> Result<Self> {
        let home = tempfile::tempdir()?;
        let config = Self::config_for(needed)?;
        let mut machine = Machine::current();
        machine.home = home.path().to_path_buf();
        machine.env.insert(
            "VOICEPUNCHER_KWS_BINARY".into(),
            workspace_root()
                .join("target/release/voicepuncher-command-spotting")
                .display()
                .to_string(),
        );
        let mut services = ManagedServices::start_on(machine)?;
        services.require(&config);
        let view = services.view();
        let run = Self { services, view, config, home, paused: Vec::new() };
        let ready = wait_until(READY_WITHIN, || {
            let progress = run.services.progress();
            progress.all_ready() || progress.failed.is_some()
        });
        let progress = run.services.progress();
        if !ready || !progress.all_ready() {
            bail!(
                "the app's services were not ready within {READY_WITHIN:?}: {progress:?}\n{:#?}",
                run.view.statuses()
            );
        }
        Ok(run)
    }

    /// The configuration of the services `needed` names, each on a free port.
    fn config_for(needed: Needed) -> Result<Config> {
        let mut config = Config::default();
        config.transcription.base_url = format!(
            "http://127.0.0.1:{}/v1",
            free_service_port(SPEECH_RECOGNITION_COPY_OFFSET)?
        );
        config.services.dictation_copies = needed.dictation_copies;
        config.services.commands_url = format!(
            "http://127.0.0.1:{}/v1",
            free_service_port(SPEECH_RECOGNITION_COPY_OFFSET)?
        );
        config.services.command_copies = 2;
        config.services.command_spotting_copies = 2;
        config.voice_control.enabled = needed.voice_control;
        config.voice_control.url = format!(
            "ws://127.0.0.1:{}",
            free_service_port(COMMAND_SPOTTING_COPY_OFFSET)?
        );
        Ok(config)
    }

    /// The ports of the copies of `kind`, in port order.
    pub fn ports(&self, kind: ServiceKind) -> Vec<u16> {
        let mut ports: Vec<u16> = self
            .statuses()
            .into_iter()
            .filter(|status| status.kind == kind)
            .map(|status| status.port)
            .collect();
        ports.sort_unstable();
        ports
    }

    pub fn statuses(&self) -> Vec<ServiceStatus> {
        self.view.statuses()
    }

    /// Stops the copy on `port` without ending it, so it no longer answers; returns its process.
    pub fn pause(&mut self, port: u16) -> Result<u32> {
        let pid = process_on(port)
            .with_context(|| format!("no service process listens on port {port}"))?;
        signal(pid, "-STOP")?;
        self.paused.push(pid);
        Ok(pid)
    }

    /// The scenario's home: it holds the services' logs, for a failure message.
    pub fn home(&self) -> &std::path::Path {
        self.home.path()
    }
}

/// Kills the service copy on `port` at once; returns its process.
pub fn kill_copy(port: u16) -> Result<u32> {
    let pid =
        process_on(port).with_context(|| format!("no service process listens on port {port}"))?;
    signal(pid, "-KILL")?;
    Ok(pid)
}

/// The service copy on one of `ports` this process holds a connection to.
pub fn connected_copy(ports: &[u16]) -> Option<u16> {
    let me = std::process::id();
    ports.iter().copied().find(|port| {
        let Some(service) = process_on(*port) else {
            return false;
        };
        let holders = established(*port);
        holders.contains(&service) && holders.contains(&me)
    })
}

/// A free port whose copy ports at `offset` are free too, so nothing else is taken for a copy.
fn free_service_port(offset: u32) -> Result<u16> {
    for _ in 0..1_000 {
        let port = free_port()?;
        let copy_ports = ServiceAddress::parse(&format!("http://127.0.0.1:{port}/v1"))
            .map_err(|e| anyhow!("{e}"))?
            .with_copy_offset(offset)
            .copy_ports();
        if !copy_ports.is_empty() && !copy_ports.into_iter().any(answers) {
            return Ok(port);
        }
    }
    bail!("no free port with free copy ports {offset} away")
}

/// The service process on `port`, found by its command line.
pub fn process_on(port: u16) -> Option<u32> {
    pgrep(&format!("^[^ ]*{PROGRAMS} .*--port {port}( |$)"))
}

/// The processes with an established connection on `port`, either end.
fn established(port: u16) -> Vec<u32> {
    let Ok(output) = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:ESTABLISHED", "-t"])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

/// The process's logger: it keeps every note and warning for the scenarios, and passes
/// each line on to the logger `RUST_LOG` configures.
struct Lines {
    kept: Mutex<Vec<String>>,
    printed: env_logger::Logger,
}

impl log::Log for Lines {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info || self.printed.enabled(metadata)
    }
    fn log(&self, record: &log::Record) {
        if record.level() <= log::Level::Info {
            let line = format!("{} {}", record.level(), record.args());
            self.kept.lock().unwrap().push(line);
        }
        self.printed.log(record);
    }
    fn flush(&self) {
        self.printed.flush();
    }
}

static LINES: OnceLock<&'static Lines> = OnceLock::new();

/// Installs the process's logger, which prints what `printed` prints and keeps the rest.
pub fn capture_log(printed: env_logger::Logger) {
    let level = printed.filter().max(log::LevelFilter::Info);
    let lines: &'static Lines = Box::leak(Box::new(Lines { kept: Mutex::default(), printed }));
    if LINES.set(lines).is_ok() && log::set_logger(lines).is_ok() {
        log::set_max_level(level);
    }
}

/// Where the log stands now: [`logged_since`] reads only the lines after it.
pub fn log_mark() -> usize {
    LINES
        .get()
        .map_or(0, |lines| lines.kept.lock().unwrap().len())
}

/// The log lines after `mark` that contain `fragment`.
pub fn logged_since(mark: usize, fragment: &str) -> Vec<String> {
    let Some(lines) = LINES.get() else {
        return Vec::new();
    };
    lines
        .kept
        .lock()
        .unwrap()
        .iter()
        .skip(mark)
        .filter(|line| line.contains(fragment))
        .cloned()
        .collect()
}
