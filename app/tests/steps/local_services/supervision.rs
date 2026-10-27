//! The service runtime on fake service programs: the real launcher starts shell scripts that
//! note their starts and pids in the scenario's folder, a probe answers on the ports a step
//! says, and the steps move the clock.

use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::secondary::localservices::process::ProcessLauncher;
use voicepuncher::core::application::{Clock, Services};
use voicepuncher::core::domain::{ServiceKind, ServiceSpec, ServiceState, ServiceStatus};

use super::{alive, fake_program, Answering, StepClock};

pub(super) const DICTATION: u16 = 18082;
pub(super) const COMMANDS: u16 = 18086;
pub(super) const COMMAND_SPOTTING: u16 = 49083;
/// How long the steps drive the runtime before they give up on what they wait for.
const DRIVE_LIMIT: Duration = Duration::from_secs(10);
/// How far the clock moves at every step of the runtime while a step drives it.
const STEP: Duration = Duration::from_secs(1);

/// The runtime of one scenario, its fake programs and the processes the service scripts run.
pub(super) struct Supervised {
    pub(super) dir: tempfile::TempDir,
    pub(super) services: Option<Services>,
    pub(super) clock: StepClock,
    pub(super) probe: Answering,
    /// The processes the service scripts run outside the app, by port.
    pub(super) scripts: Vec<(u16, Child)>,
    /// The specs the app supervises, by port.
    pub(super) specs: Vec<ServiceSpec>,
    /// How long the last stop took on the clock.
    pub(super) stop_took: Option<Duration>,
    /// The home whose catalog named a service's program and log file, if any.
    pub(super) home: Option<super::home::FakeHome>,
    /// The ports of the copies the app adopted, in the order they were planned.
    pub(super) adopted_copies: Vec<u16>,
    /// When, on the clock, an adopted copy's process ended or hung.
    pub(super) gone_at: Option<Instant>,
    /// The process that holds a copy's port and takes connections without answering.
    pub(super) hung: Option<voicepuncher_test_steps::loopback::SilentListener>,
}

impl std::fmt::Debug for Supervised {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Supervised")
            .field("statuses", &self.statuses())
            .finish_non_exhaustive()
    }
}

impl Supervised {
    pub(super) fn new() -> Result<Self> {
        let clock = StepClock::new();
        let probe = Answering::default();
        let services = Services::new(
            Box::new(ProcessLauncher),
            Box::new(probe.clone()),
            Arc::new(clock.clone()),
        );
        Ok(Self {
            dir: tempfile::tempdir()?,
            services: Some(services),
            clock,
            probe,
            scripts: Vec::new(),
            specs: Vec::new(),
            stop_took: None,
            home: None,
            adopted_copies: Vec::new(),
            gone_at: None,
            hung: None,
        })
    }

    /// The file a fake program of `port` notes `what` in: its starts, its pid.
    pub(super) fn note(&self, port: u16, what: &str) -> PathBuf {
        self.dir.path().join(format!("{port}.{what}"))
    }

    /// How many times the fake program of `port` was started.
    pub(super) fn starts(&self, port: u16) -> usize {
        std::fs::read_to_string(self.note(port, "starts")).map_or(0, |text| text.lines().count())
    }

    /// A fake program for `port` that notes its start and pid, then runs `then`.
    pub(super) fn program(&self, port: u16, then: &str) -> Result<PathBuf> {
        let bin = self.dir.path().join("bin");
        self.program_at(&bin, &format!("engine-{port}"), port, then)
    }

    /// The fake program `name` in `dir`, for `port`: notes its start and pid, then runs `then`.
    pub(super) fn program_at(
        &self,
        dir: &Path,
        name: &str,
        port: u16,
        then: &str,
    ) -> Result<PathBuf> {
        let (starts, pid) = (self.note(port, "starts"), self.note(port, "pid"));
        let body = format!(
            "echo start >> '{}'\necho $$ > '{}'\n{then}",
            starts.display(),
            pid.display()
        );
        fake_program(dir, name, &body)
    }

    /// The spec of a service of `kind` on `port` that runs `then` after noting its start.
    pub(super) fn spec(&self, kind: ServiceKind, port: u16, then: &str) -> Result<ServiceSpec> {
        Ok(ServiceSpec {
            kind,
            label: format!("{} {port}", kind.name()),
            program: self.program(port, then)?,
            args: Vec::new(),
            port,
            log_file: self.dir.path().join(format!("logs/engine-{port}.log")),
            health_checked: true,
            problem: None,
        })
    }

    pub(super) fn services(&mut self) -> Result<&mut Services> {
        self.services.as_mut().context("the app quit")
    }

    pub(super) fn supervise(&mut self, spec: ServiceSpec) -> Result<()> {
        self.specs.push(spec);
        self.supervise_planned()
    }

    /// Notes a service the app will need, without supervising it yet.
    pub(super) fn plan(&mut self, spec: ServiceSpec) {
        self.specs.push(spec);
    }

    /// Supervises every service planned so far and steps the runtime once.
    pub(super) fn supervise_planned(&mut self) -> Result<()> {
        let specs = self.specs.clone();
        let services = self.services()?;
        services.supervise(specs);
        services.tick();
        Ok(())
    }

    /// Steps the runtime through `span` on the clock, a second at a time.
    pub(super) fn drive_for(&mut self, span: Duration) -> Result<()> {
        for _ in 0..span.as_secs() {
            self.services()?.tick();
            self.clock.advance(STEP);
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    }

    pub(super) fn statuses(&self) -> Vec<ServiceStatus> {
        self.services
            .as_ref()
            .map(Services::snapshot)
            .unwrap_or_default()
    }

    pub(super) fn state(&self, port: u16) -> Result<ServiceState> {
        self.statuses()
            .into_iter()
            .find(|status| status.port == port)
            .map(|status| status.state)
            .with_context(|| format!("no service on port {port} is supervised"))
    }

    /// Steps the runtime, the clock a second further each time, until `done` holds for the
    /// service on `port`.
    pub(super) fn drive_until(
        &mut self,
        port: u16,
        done: impl Fn(&ServiceState) -> bool,
    ) -> Result<()> {
        let deadline = Instant::now() + DRIVE_LIMIT;
        while Instant::now() < deadline {
            self.services()?.tick();
            if done(&self.state(port)?) {
                return Ok(());
            }
            self.clock.advance(STEP);
            std::thread::sleep(Duration::from_millis(20));
        }
        bail!("the service on port {port} is {}", self.state(port)?)
    }

    /// Starts the service of `spec` as the app does and waits until it is Ready: its port
    /// answers once its program runs.
    pub(super) fn start_owned(&mut self, spec: ServiceSpec) -> Result<()> {
        let port = spec.port;
        self.supervise(spec)?;
        self.drive_until(port, |state| matches!(state, ServiceState::Starting { .. }))?;
        super::written_pid(&self.note(port, "pid"))?;
        self.probe.set(port, true);
        self.drive_until(port, |state| *state == ServiceState::Ready)
    }

    /// A process the service scripts run on `port`, outside the app; its port answers.
    pub(super) fn run_outside(&mut self, port: u16) -> Result<()> {
        let child = Command::new("/bin/sleep").arg("60").spawn()?;
        self.scripts.push((port, child));
        self.probe.set(port, true);
        Ok(())
    }

    /// Ends the process the scripts run on `port`; its port no longer answers.
    pub(super) fn end_outside(&mut self, port: u16) -> Result<()> {
        for (_, child) in self.scripts.iter_mut().filter(|(p, _)| *p == port) {
            child.kill()?;
            child.wait()?;
        }
        self.scripts.retain(|(p, _)| *p != port);
        self.probe.set(port, false);
        self.gone_at = Some(self.clock.now());
        Ok(())
    }

    /// Whether the process the scripts run on `port` still runs.
    pub(super) fn outside_runs(&mut self, port: u16) -> Result<bool> {
        let (_, child) = self
            .scripts
            .iter_mut()
            .find(|(p, _)| *p == port)
            .with_context(|| format!("the scripts run nothing on port {port}"))?;
        Ok(child.try_wait()?.is_none())
    }

    /// Stops every owned service as a quit does, noting how long that took on the clock.
    pub(super) fn quit(&mut self) -> Result<()> {
        let before = self.clock.now();
        self.services()?.stop_all();
        self.stop_took = Some(self.clock.now().duration_since(before));
        Ok(())
    }

    /// The pid a fake program of `port` noted.
    pub(super) fn pid(&self, port: u16) -> Result<u32> {
        super::written_pid(&self.note(port, "pid"))
    }
}

impl Drop for Supervised {
    fn drop(&mut self) {
        if let Some(services) = self.services.as_mut() {
            services.stop_all();
        }
        for (_, child) in &mut self.scripts {
            let _ = child.kill();
            let _ = child.wait();
        }
        let ports: Vec<u16> = self.specs.iter().map(|spec| spec.port).collect();
        for port in ports {
            if let Ok(pid) = std::fs::read_to_string(self.note(port, "pid")) {
                kill(pid.trim());
            }
        }
    }
}

/// Kills what is left of a fake program the scenario left running.
fn kill(pid: &str) {
    if pid.parse::<u32>().is_ok_and(alive) {
        let _ = Command::new("/bin/kill").args(["-9", pid]).status();
    }
}
