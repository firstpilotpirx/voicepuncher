//! The local-services steps: the service runtime on fake service programs, the catalog's
//! lookup on a home of the scenario's own, the copies of a service on fake copies and the
//! command spotting client on a fake service.

mod adopted_steps;
mod catalog;
mod catalog_copies;
mod catalog_history;
mod catalog_recognizer;
mod copies;
mod fake_copies;
mod foreign_program;
mod home;
mod hung_steps;
mod scripts;
mod stale;
mod stall;
mod supervision;
mod supervision_steps;

use std::collections::HashSet;
use std::os::unix::fs::PermissionsExt;
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::core::application::{Clock, OtherProtocol, ProbeAnswer, ServiceProbe};
use voicepuncher::core::domain::ServiceKind;

use crate::IntegrationWorld;

/// What one local-services scenario keeps between its steps.
#[derive(Debug, Default)]
pub struct LocalRun {
    supervised: Option<supervision::Supervised>,
    catalog: Option<catalog::CatalogRun>,
    copies: Option<copies::CopiesRun>,
    foreign: Option<foreign_program::ForeignRun>,
    stall: Option<stall::StallRun>,
    stale: Option<stale::StaleRun>,
    scripts_config: Option<voicepuncher::config::Config>,
}

fn local(world: &mut IntegrationWorld) -> &mut LocalRun {
    world
        .extra
        .local_services
        .get_or_insert_with(LocalRun::default)
}

/// Writes an executable shell script `name` into `dir`: a fake service program.
fn fake_program(dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n"))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    Ok(path)
}

/// Whether the process `pid` still runs.
fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

/// The pid a fake service program wrote into `file`, once it did.
fn written_pid(file: &Path) -> Result<u32> {
    let text = wait_for_file(file, |text| !text.trim().is_empty())?;
    text.trim()
        .parse()
        .with_context(|| format!("no pid in {}: {text:?}", file.display()))
}

/// The text of `file` once `done` holds for it, or after 5 s whatever it holds.
fn wait_for_file(file: &Path, done: impl Fn(&str) -> bool) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if done(&text) || Instant::now() > deadline {
            return Ok(text);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A clock that moves only when a step or a pause moves it.
#[derive(Clone)]
struct StepClock(Arc<Mutex<Instant>>);

impl StepClock {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }

    fn advance(&self, by: Duration) {
        *lock(&self.0) += by;
    }
}

impl Clock for StepClock {
    fn now(&self) -> Instant {
        *lock(&self.0)
    }

    fn sleep(&self, pause: Duration) {
        self.advance(pause);
        // the processes the stop waits for end in real time
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Answers Ready on the ports a step says answer; on the ports a step says hang, Busy while
/// something takes connections there; Closed on the others. The ports a step says speak an
/// older protocol refuse the app's.
#[derive(Clone, Default)]
struct Answering(
    Arc<Mutex<HashSet<u16>>>,
    Arc<Mutex<HashSet<u16>>>,
    Arc<Mutex<HashSet<u16>>>,
);

impl Answering {
    fn set(&self, port: u16, answers: bool) {
        mark(&self.0, port, answers);
    }

    /// The port no longer answers as the service; whatever listens there only takes connections.
    fn hang(&self, port: u16) {
        self.set(port, false);
        lock(&self.1).insert(port);
    }

    /// Whether what answers on `port` speaks an older protocol version than the app.
    fn speak_older(&self, port: u16, older: bool) {
        mark(&self.2, port, older);
    }
}

/// Puts `port` into `ports` when `on`, else takes it out.
fn mark(ports: &Mutex<HashSet<u16>>, port: u16, on: bool) {
    let mut ports = lock(ports);
    if on {
        ports.insert(port);
    } else {
        ports.remove(&port);
    }
}

/// The protocol version a copy left by an older app speaks, and the app's.
const OLDER_PROTOCOL: OtherProtocol = OtherProtocol { service: 2, app: 3 };

impl ServiceProbe for Answering {
    fn answer(&self, _kind: ServiceKind, port: u16) -> ProbeAnswer {
        if lock(&self.0).contains(&port) {
            ProbeAnswer::Ready
        } else if lock(&self.1).contains(&port)
            && std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
        {
            ProbeAnswer::Busy
        } else {
            ProbeAnswer::Closed
        }
    }

    fn other_protocol(&self, _kind: ServiceKind, port: u16) -> Option<OtherProtocol> {
        lock(&self.2).contains(&port).then_some(OLDER_PROTOCOL)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
