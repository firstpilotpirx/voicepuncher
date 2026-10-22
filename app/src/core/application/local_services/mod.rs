//! The service runtime: starts, adopts, watches, restarts and stops the local services one
//! configuration needs.

mod supervision;

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::core::domain::{ServiceSpec, ServiceStatus};

pub use supervision::{
    Senses, Supervision, CHECK_INTERVAL, FAILED_CHECKS, FAILED_STARTS, FIRST_PAUSE, LONGEST_PAUSE,
    READINESS_LIMIT,
};

/// How long an owned service has to end once asked, before it is killed.
pub const STOP_GRACE: Duration = Duration::from_secs(5);
/// How often the runtime's thread steps every supervision.
pub const TICK: Duration = Duration::from_millis(100);
/// How often a stop looks whether the services it asked to end have ended.
const STOP_POLL: Duration = Duration::from_millis(50);

/// Starts the process of a service.
pub trait ServiceLauncher: Send + Sync {
    fn spawn(&self, spec: &ServiceSpec) -> Result<Box<dyn ServiceProcess>>;
}

/// A started process of a service.
pub trait ServiceProcess: Send {
    fn id(&self) -> u32;
    /// How it ended, once it has: "code 1", "signal 9".
    fn exited(&mut self) -> Option<String>;
    /// Asks it to end.
    fn terminate(&mut self);
    /// Ends it at once and waits until it is gone.
    fn kill(&mut self);
}

/// What a port answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeAnswer {
    /// The expected service answers and is ready.
    Ready,
    /// Something accepts connections but does not answer as the ready service.
    Busy,
    /// Nothing accepts connections.
    Closed,
}

/// Asks a service's port whether the service answers.
pub trait ServiceProbe: Send + Sync {
    fn answer(&self, spec: &ServiceSpec) -> ProbeAnswer;
}

/// The time, and a pause; a test's clock moves on when it pauses.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn sleep(&self, pause: Duration);
}

/// Every supervised service, stepped together.
pub struct Services {
    units: Vec<Supervision>,
    launcher: Box<dyn ServiceLauncher>,
    probe: Box<dyn ServiceProbe>,
    clock: Arc<dyn Clock>,
}

impl Services {
    pub fn new(
        launcher: Box<dyn ServiceLauncher>,
        probe: Box<dyn ServiceProbe>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { units: Vec::new(), launcher, probe, clock }
    }

    /// Supervises every service of `specs` whose port is not supervised yet.
    pub fn supervise(&mut self, specs: Vec<ServiceSpec>) {
        let now = self.clock.now();
        for spec in specs {
            if !self.units.iter().any(|unit| unit.spec().port == spec.port) {
                self.units.push(Supervision::new(spec, now));
            }
        }
    }

    /// One step of every supervision.
    pub fn tick(&mut self) {
        let senses = Senses {
            launcher: self.launcher.as_ref(),
            probe: self.probe.as_ref(),
            now: self.clock.now(),
        };
        for unit in &mut self.units {
            unit.tick(&senses);
        }
    }

    pub fn snapshot(&self) -> Vec<ServiceStatus> {
        self.units.iter().map(Supervision::status).collect()
    }

    /// Retries or restarts the service on `port`.
    pub fn restart(&mut self, port: u16) {
        let now = self.clock.now();
        if let Some(unit) = self.units.iter_mut().find(|unit| unit.spec().port == port) {
            unit.restart(now);
        }
    }

    /// Stops every owned service, kind after kind in stop order: each is asked to end and
    /// killed after the grace period; adopted services keep running.
    pub fn stop_all(&mut self) {
        let mut kinds: Vec<_> = self.units.iter().map(|unit| unit.spec().kind).collect();
        kinds.sort();
        kinds.dedup();
        for kind in kinds {
            let mut group: Vec<&mut Supervision> = self
                .units
                .iter_mut()
                .filter(|unit| unit.spec().kind == kind)
                .collect();
            stop_group(&mut group, self.clock.as_ref());
        }
    }
}

fn stop_group(group: &mut [&mut Supervision], clock: &dyn Clock) {
    let asked = group
        .iter_mut()
        .fold(false, |asked, unit| unit.ask_to_end() || asked);
    let deadline = clock.now() + STOP_GRACE;
    while asked && clock.now() < deadline && !group.iter_mut().all(|unit| unit.ended()) {
        clock.sleep(STOP_POLL);
    }
    for unit in group.iter_mut() {
        unit.finish_stop();
    }
}

enum Command {
    Supervise(Vec<ServiceSpec>),
    Restart(u16),
    Stop,
}

/// The services stepped on a thread of their own; every method returns at once except
/// [`ServiceRuntime::stop`], which waits until the owned services ended.
pub struct ServiceRuntime {
    commands: Sender<Command>,
    statuses: Arc<Mutex<Vec<ServiceStatus>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl ServiceRuntime {
    /// Starts the thread that steps `services` every `tick`.
    pub fn start(services: Services, tick: Duration) -> Result<Self> {
        let (commands, inbox) = mpsc::channel();
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let published = Arc::clone(&statuses);
        let thread = std::thread::Builder::new()
            .name("services".into())
            .spawn(move || serve(services, &inbox, &published, tick))?;
        Ok(Self { commands, statuses, thread: Mutex::new(Some(thread)) })
    }

    pub fn supervise(&self, specs: Vec<ServiceSpec>) {
        let _ = self.commands.send(Command::Supervise(specs));
    }

    pub fn restart(&self, port: u16) {
        let _ = self.commands.send(Command::Restart(port));
    }

    /// The states after the last step.
    pub fn snapshot(&self) -> Vec<ServiceStatus> {
        lock(&self.statuses).clone()
    }

    /// Stops the owned services and the thread; later calls do nothing.
    pub fn stop(&self) {
        let Some(thread) = lock(&self.thread).take() else {
            return;
        };
        let _ = self.commands.send(Command::Stop);
        if thread.join().is_err() {
            log::error!("services: the services thread panicked");
        }
    }
}

impl Drop for ServiceRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

fn serve(
    mut services: Services,
    inbox: &Receiver<Command>,
    statuses: &Mutex<Vec<ServiceStatus>>,
    tick: Duration,
) {
    loop {
        match inbox.recv_timeout(tick) {
            Ok(Command::Supervise(specs)) => services.supervise(specs),
            Ok(Command::Restart(port)) => services.restart(port),
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        services.tick();
        *lock(statuses) = services.snapshot();
    }
    services.stop_all();
    *lock(statuses) = services.snapshot();
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) mod fakes;
#[cfg(test)]
mod tests;
