//! The copies of one service, and who holds which.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError, Weak};
use std::thread;
use std::time::{Duration, Instant};

use crate::address::{AddressError, ServiceAddress};
use crate::probe::{Probe, TcpProbe};

mod lease;
mod state;

pub use lease::Lease;
use state::State;

/// How often the copies are probed: a copy that comes back joins within this.
const PROBE_INTERVAL: Duration = Duration::from_secs(1);

/// What the pool knows of one copy that takes requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyState {
    pub port: u16,
    pub busy: bool,
    pub hung: bool,
}

/// Why no copy was leased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    /// Every copy stayed busy for the whole wait.
    NoFreeCopy { address: String, waited: Duration },
    /// No copy of the service accepts connections.
    NoCopies { address: String },
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFreeCopy { address, waited } => write!(
                f,
                "every copy of the service at {address} stayed busy for {} ms",
                waited.as_millis()
            ),
            Self::NoCopies { address } => {
                write!(f, "no copy of the service at {address} accepts connections")
            }
        }
    }
}

impl std::error::Error for PoolError {}

/// How a request on a copy ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// The copy answered, whatever it answered.
    Answered,
    /// The copy refused the connection: it is down until it accepts connections again.
    Refused,
}

/// What came of a request run on a lease within its wait.
#[derive(Debug)]
pub enum Ran<T> {
    Answered(T),
    /// The wait passed; the copy stays leased until the request ends.
    GaveUp,
}

/// The copies of one service: one request per copy, first come first served.
#[derive(Clone)]
pub struct CopyPool {
    inner: Arc<Inner>,
}

struct Inner {
    address: ServiceAddress,
    probe: Box<dyn Probe>,
    state: Mutex<State>,
    changed: Condvar,
}

static POOLS: OnceLock<Mutex<HashMap<String, Weak<Inner>>>> = OnceLock::new();

impl CopyPool {
    /// The one pool of this process for the service at `url`, so every client of the
    /// service shares its copies; each process starts its search at a copy of its own.
    pub fn shared(url: &str) -> Result<Self, AddressError> {
        Ok(Self::shared_at(ServiceAddress::parse(url)?))
    }

    /// As [`CopyPool::shared`], for an address whose copies may lie at another offset.
    pub fn shared_at(address: ServiceAddress) -> Self {
        let key = format!("{address} {:?}", address.copy_ports());
        let mut pools = unpoisoned(POOLS.get_or_init(Default::default).lock());
        if let Some(inner) = pools.get(&key).and_then(Weak::upgrade) {
            return Self { inner };
        }
        let probe = TcpProbe::new(address.host());
        let offset = std::process::id() as usize;
        let pool = Self::discover(address, probe, offset, PROBE_INTERVAL);
        pools.insert(key, Arc::downgrade(&pool.inner));
        pool
    }

    /// A pool that probes the copies of `address` at once and then every `interval`, and
    /// starts its search for a free copy at `offset`; it returns before the first probe
    /// round ends, and every question about the copies waits for that round.
    pub fn discover(
        address: ServiceAddress,
        probe: impl Probe,
        offset: usize,
        interval: Duration,
    ) -> Self {
        let state = Mutex::new(State::new(&address, offset));
        let inner =
            Arc::new(Inner { address, probe: Box::new(probe), state, changed: Condvar::new() });
        let watched = Arc::downgrade(&inner);
        let spawned = thread::Builder::new()
            .name("copy-probe".into())
            .spawn(move || watch(watched, interval));
        if spawned.is_err() {
            inner.probe_round();
        }
        Self { inner }
    }

    pub fn has_copies(&self) -> bool {
        self.inner.lock_probed().has_copies()
    }

    /// The copies that take requests, in port order.
    pub fn snapshot(&self) -> Vec<CopyState> {
        let copies = self.inner.lock_probed().snapshot();
        copies
            .into_iter()
            .map(|(port, busy, hung)| CopyState { port, busy, hung })
            .collect()
    }

    /// A free copy, after waiting at most `wait` for one, first come first served; a
    /// request that gives up waiting holds none. Fails at once when no copy takes
    /// requests.
    pub fn lease(&self, wait: Duration) -> Result<Lease, PoolError> {
        self.lease_avoiding(wait, &[])
    }

    /// As [`CopyPool::lease`], preferring a copy whose port is not in `tried`, so a
    /// new attempt of a request goes to another copy whenever one is free.
    pub fn lease_avoiding(&self, wait: Duration, tried: &[u16]) -> Result<Lease, PoolError> {
        self.place().lease_avoiding(wait, tried)
    }

    /// Whether a copy takes requests, after waiting at most `wait` for one to come back
    /// when none does: a service being restarted answers again within seconds.
    pub fn await_copy(&self, wait: Duration) -> bool {
        let deadline = Instant::now() + wait;
        let mut state = self.inner.lock_probed();
        loop {
            if state.has_copies() {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            state = unpoisoned(self.inner.changed.wait_timeout(state, deadline - now)).0;
        }
    }

    /// A place in the queue for a copy, for one request that may wait more than once.
    pub fn place(&self) -> Place {
        Place { inner: self.inner.clone(), ticket: None }
    }
}

/// A request's place in the queue for a copy: taken at its first wait, kept when a wait
/// runs out, given up once a copy is leased or the place is dropped.
pub struct Place {
    inner: Arc<Inner>,
    ticket: Option<u64>,
}

impl Place {
    /// A free copy, after waiting at most `wait` for one, first come first served,
    /// preferring a copy whose port is not in `tried`; when the wait runs out the place
    /// stays in the queue for the next wait. Fails at once when no copy takes requests.
    pub fn lease_avoiding(&mut self, wait: Duration, tried: &[u16]) -> Result<Lease, PoolError> {
        let deadline = Instant::now() + wait;
        let mut state = self.inner.lock_probed();
        let ticket = *self.ticket.get_or_insert_with(|| state.enqueue());
        loop {
            let now = Instant::now();
            if let Some(index) = state.take_turn(ticket, now, tried) {
                self.ticket = None;
                self.inner.changed.notify_all();
                return Ok(self.inner.lease_of(index, state.port(index)));
            }
            if !state.has_copies() {
                state.leave_queue(ticket);
                self.ticket = None;
                self.inner.changed.notify_all();
                return Err(self.inner.no_copy(false, wait));
            }
            if now >= deadline {
                return Err(self.inner.no_copy(true, wait));
            }
            state = unpoisoned(self.inner.changed.wait_timeout(state, deadline - now)).0;
        }
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        if let Some(ticket) = self.ticket.take() {
            self.inner.lock().leave_queue(ticket);
            self.inner.changed.notify_all();
        }
    }
}

impl Inner {
    fn lease_of(self: &Arc<Self>, index: usize, port: u16) -> Lease {
        let url = self.address.with_port(port);
        Lease { inner: self.clone(), index, port, url, released: false }
    }

    fn no_copy(&self, has_copies: bool, waited: Duration) -> PoolError {
        let address = self.address.to_string();
        if has_copies {
            PoolError::NoFreeCopy { address, waited }
        } else {
            PoolError::NoCopies { address }
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        unpoisoned(self.state.lock())
    }

    /// The state once the first probe round ended: it takes at most one round.
    fn lock_probed(&self) -> MutexGuard<'_, State> {
        let state = self.lock();
        unpoisoned(self.changed.wait_while(state, |state| !state.probed_once))
    }

    /// Probes the service's own port, and the copy ports only when it does not answer; a
    /// port missed once is probed once more at once, as a copy busy decoding under load
    /// answers a probe late, and leaves only when both probes miss.
    fn probe_round(&self) {
        let ports = self.lock().ports();
        let service_answers = self.probe.answers(ports[0]);
        let copies = ports[1..]
            .iter()
            .map(|port| !service_answers && self.answers_twice_asked(*port));
        let answers: Vec<bool> = std::iter::once(service_answers).chain(copies).collect();
        let mut state = self.lock();
        state.probed(&answers);
        for (port, waited) in state.find_hung(Instant::now()) {
            log::warn!(
                "copy {port} hung: no answer in {} s (service at {})",
                waited.as_secs(),
                self.address
            );
        }
        self.changed.notify_all();
    }

    fn answers_twice_asked(&self, port: u16) -> bool {
        self.probe.answers(port) || self.probe.answers(port)
    }

    fn expect_answer_within(&self, index: usize, wait: Duration) {
        self.lock().expect_answer_within(index, wait);
    }

    fn release(&self, index: usize, finish: Finish) {
        let mut state = self.lock();
        let port = state.port(index);
        if state.release(index, finish == Finish::Refused) {
            log::info!(
                "copy {port} of the service at {} answered again",
                self.address
            );
        }
        if finish == Finish::Refused {
            log::warn!(
                "copy {port} refused the connection: skipped until it answers (service at {})",
                self.address
            );
        }
        self.changed.notify_all();
    }
}

/// Probes the copies at once and then every `interval` while the pool is in use.
fn watch(inner: Weak<Inner>, interval: Duration) {
    loop {
        let Some(pool) = inner.upgrade() else {
            return;
        };
        pool.probe_round();
        drop(pool);
        thread::sleep(interval);
    }
}

fn unpoisoned<T>(result: Result<T, PoisonError<T>>) -> T {
    result.unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
