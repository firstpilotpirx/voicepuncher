//! Test doubles of the service ports: a clock that moves when told, a launcher that records
//! its spawns, processes that exit when told and a probe answering what it is told.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::core::domain::{ServiceKind, ServiceSpec};

use super::{
    Clock, HolderStop, OtherProtocol, ProbeAnswer, ServiceLauncher, ServiceProbe, ServiceProcess,
};

#[derive(Clone)]
pub struct FakeClock(Arc<Mutex<Instant>>);

impl FakeClock {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }

    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }

    fn sleep(&self, pause: Duration) {
        self.advance(pause);
    }
}

/// What happened to one spawned process.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProcessLife {
    pub port: u16,
    pub exit: Option<String>,
    pub terminated: bool,
    pub killed: bool,
    /// Ends when asked to; a stubborn one only when killed.
    pub stubborn: bool,
}

/// Every process spawned, in order, shared by the launcher and its processes; and the ports
/// whose holder the launcher was asked to stop.
#[derive(Clone, Default)]
pub struct Spawns(
    Arc<Mutex<Vec<ProcessLife>>>,
    Arc<Mutex<Vec<u16>>>,
    Arc<Mutex<Vec<u16>>>,
);

impl Spawns {
    pub fn all(&self) -> Vec<ProcessLife> {
        self.0.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.0.lock().unwrap().len()
    }

    /// The ports of the processes asked to end, in the order they were asked.
    pub fn stop_order(&self) -> Vec<u16> {
        self.1.lock().unwrap().clone()
    }

    /// The ports whose holder the launcher was asked to stop, in order.
    pub fn holders_stopped(&self) -> Vec<u16> {
        self.2.lock().unwrap().clone()
    }

    /// Makes the last process exit with `code`.
    pub fn exit_last(&self, code: &str) {
        if let Some(life) = self.0.lock().unwrap().last_mut() {
            life.exit = Some(code.into());
        }
    }
}

pub struct FakeLauncher {
    pub spawns: Spawns,
    pub refuse: bool,
    pub stubborn: bool,
    /// What stopping the program on a port does: the service's own program by default.
    pub holder: HolderStop,
}

impl FakeLauncher {
    pub fn new(spawns: &Spawns) -> Self {
        Self {
            spawns: spawns.clone(),
            refuse: false,
            stubborn: false,
            holder: HolderStop::Stopped(4242),
        }
    }
}

impl ServiceLauncher for FakeLauncher {
    fn spawn(&self, spec: &ServiceSpec) -> Result<Box<dyn ServiceProcess>> {
        if self.refuse {
            bail!("no such program");
        }
        let mut all = self.spawns.0.lock().unwrap();
        all.push(ProcessLife {
            port: spec.port,
            stubborn: self.stubborn,
            ..ProcessLife::default()
        });
        Ok(Box::new(FakeProcess {
            spawns: self.spawns.clone(),
            index: all.len() - 1,
        }))
    }

    fn stop_holder(&self, spec: &ServiceSpec) -> HolderStop {
        self.spawns.2.lock().unwrap().push(spec.port);
        self.holder.clone()
    }
}

struct FakeProcess {
    spawns: Spawns,
    index: usize,
}

impl FakeProcess {
    fn with<T>(&self, f: impl FnOnce(&mut ProcessLife) -> T) -> T {
        f(&mut self.spawns.0.lock().unwrap()[self.index])
    }
}

impl ServiceProcess for FakeProcess {
    fn id(&self) -> u32 {
        u32::try_from(self.index).unwrap() + 1
    }

    fn exited(&mut self) -> Option<String> {
        self.with(|life| life.exit.clone())
    }

    fn terminate(&mut self) {
        let port = self.with(|life| life.port);
        self.spawns.1.lock().unwrap().push(port);
        self.with(|life| {
            life.terminated = true;
            if !life.stubborn {
                life.exit = Some("signal 15".into());
            }
        });
    }

    fn kill(&mut self) {
        self.with(|life| {
            life.killed = true;
            life.exit = Some("signal 9".into());
        });
    }
}

/// Answers per port what it was told, Closed by default; transcribes every warm-up except on
/// the ports told to refuse it, and counts the warm-ups each port got.
#[derive(Clone, Default)]
pub struct FakeProbe {
    answers: Arc<Mutex<HashMap<u16, ProbeAnswer>>>,
    refusing: Arc<Mutex<Vec<u16>>>,
    warm_ups: Arc<Mutex<HashMap<u16, usize>>>,
    protocols: Arc<Mutex<HashMap<u16, OtherProtocol>>>,
}

impl FakeProbe {
    pub fn set(&self, port: u16, answer: ProbeAnswer) {
        self.answers.lock().unwrap().insert(port, answer);
    }

    /// The protocol the copy on `port` speaks: `None` for the app's own.
    pub fn speak(&self, port: u16, other: Option<OtherProtocol>) {
        let mut protocols = self.protocols.lock().unwrap();
        match other {
            Some(other) => protocols.insert(port, other),
            None => protocols.remove(&port),
        };
    }

    /// Whether the copy on `port` fails to transcribe its warm-up.
    pub fn refuse_warm_up(&self, port: u16, refusing: bool) {
        let mut ports = self.refusing.lock().unwrap();
        ports.retain(|&p| p != port);
        if refusing {
            ports.push(port);
        }
    }

    /// How many warm-ups the copy on `port` was sent.
    pub fn warm_ups(&self, port: u16) -> usize {
        *self.warm_ups.lock().unwrap().get(&port).unwrap_or(&0)
    }
}

impl ServiceProbe for FakeProbe {
    fn answer(&self, _kind: ServiceKind, port: u16) -> ProbeAnswer {
        *self
            .answers
            .lock()
            .unwrap()
            .get(&port)
            .unwrap_or(&ProbeAnswer::Closed)
    }

    fn other_protocol(&self, _kind: ServiceKind, port: u16) -> Option<OtherProtocol> {
        self.protocols.lock().unwrap().get(&port).copied()
    }

    fn warm_up(&self, _kind: ServiceKind, port: u16) -> Option<Duration> {
        *self.warm_ups.lock().unwrap().entry(port).or_default() += 1;
        (!self.refusing.lock().unwrap().contains(&port)).then_some(Duration::from_millis(400))
    }
}

pub fn spec(kind: ServiceKind, port: u16) -> ServiceSpec {
    ServiceSpec {
        kind,
        label: format!("{} {port}", kind.name()),
        program: PathBuf::from("engine"),
        args: Vec::new(),
        port,
        log_file: PathBuf::from(format!("/logs/engine-{port}.log")),
        health_checked: true,
        problem: None,
    }
}
