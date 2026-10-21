//! The local services the runtime waits for before it starts a pipeline.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::adapters::secondary::localservices::catalog::{self, Machine, Needs};
use crate::adapters::secondary::localservices::process::{HttpProbe, ProcessLauncher};
use crate::config::Config;
use crate::core::application::{Clock, ServiceRuntime, Services, TICK};
use crate::core::domain::{ServiceStatus, ServicesProgress};

/// How often the runtime looks at its services while it waits for them.
pub const SERVICES_POLL: Duration = Duration::from_millis(200);

/// The services one configuration needs: started on `require`, counted by `progress`,
/// stopped by `stop`.
pub trait LocalServices: Send {
    /// Starts supervising every service `config` needs that is not supervised yet; returns
    /// at once.
    fn require(&mut self, config: &Config);
    /// How far the services the last required configuration needs have come.
    fn progress(&self) -> ServicesProgress;
    /// Stops the services the app started; adopted ones keep running.
    fn stop(&mut self);
}

/// The menu's view of the services: their states, and a restart of one.
pub trait ServicesView: Send + Sync {
    fn statuses(&self) -> Vec<ServiceStatus>;
    fn restart(&self, port: u16);
}

/// The services started as processes on this machine and watched over loopback HTTP.
pub struct ManagedServices {
    runtime: Arc<ServiceRuntime>,
    machine: Machine,
    ports: Vec<u16>,
}

impl ManagedServices {
    /// Starts the supervision thread.
    pub fn start() -> Result<Self> {
        let services = Services::new(
            Box::new(ProcessLauncher),
            Box::new(HttpProbe),
            Arc::new(SystemClock),
        );
        Ok(Self {
            runtime: Arc::new(ServiceRuntime::start(services, TICK)?),
            machine: Machine::current(),
            ports: Vec::new(),
        })
    }

    /// Stops the services the app started, from any thread: a termination signal ends the
    /// process with it.
    pub fn stopper(&self) -> Box<dyn FnOnce() + Send> {
        let runtime = Arc::clone(&self.runtime);
        Box::new(move || runtime.stop())
    }

    /// What the menu shows and restarts.
    pub fn view(&self) -> Arc<dyn ServicesView> {
        Arc::new(View(Arc::clone(&self.runtime)))
    }
}

impl LocalServices for ManagedServices {
    fn require(&mut self, config: &Config) {
        let needs = Needs::of(config);
        let specs = catalog::needed(config, needs, &self.machine, &HttpProbe);
        self.ports = specs.iter().map(|spec| spec.port).collect();
        for spec in &specs {
            log::info!(
                "services: {} on port {} — {} {}",
                spec.label,
                spec.port,
                spec.program.display(),
                spec.args.join(" ")
            );
        }
        self.runtime.supervise(specs);
    }

    fn progress(&self) -> ServicesProgress {
        ServicesProgress::of(&self.ports, &self.runtime.snapshot())
    }

    fn stop(&mut self) {
        self.runtime.stop();
    }
}

impl Drop for ManagedServices {
    /// A runtime that never started still stops what it started.
    fn drop(&mut self) {
        self.runtime.stop();
    }
}

struct View(Arc<ServiceRuntime>);

impl ServicesView for View {
    fn statuses(&self) -> Vec<ServiceStatus> {
        self.0.snapshot()
    }

    fn restart(&self, port: u16) {
        self.0.restart(port);
    }
}

struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, pause: Duration) {
        std::thread::sleep(pause);
    }
}
