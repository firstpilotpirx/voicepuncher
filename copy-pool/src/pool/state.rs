//! Which copies take requests and which are held, behind the pool's lock.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::address::ServiceAddress;
use crate::probe::{ForeignProgram, Identity};

/// A copy is hung once it held one request for this many times the request's wait.
const HUNG_AFTER_WAITS: u32 = 2;

struct Copy {
    port: u16,
    up: bool,
    /// What the program on the port said it is, while it keeps accepting connections.
    identity: Option<Identity>,
    busy: bool,
    leased_at: Option<Instant>,
    hung_after: Option<Instant>,
    hung: bool,
}

/// The copies, the service's own port first, and the requests waiting for one.
pub(super) struct State {
    copies: Vec<Copy>,
    waiting: VecDeque<u64>,
    next_ticket: u64,
    offset: usize,
    /// Whether the first probe round ended: until then nothing is known of the copies.
    pub(super) probed_once: bool,
}

const SERVICE_PORT: usize = 0;

impl State {
    pub(super) fn new(address: &ServiceAddress, offset: usize) -> Self {
        let ports = std::iter::once(address.port()).chain(address.copy_ports());
        let copies = ports
            .map(|port| Copy {
                port,
                up: false,
                identity: None,
                busy: false,
                leased_at: None,
                hung_after: None,
                hung: false,
            })
            .collect();
        Self { copies, waiting: VecDeque::new(), next_ticket: 0, offset, probed_once: false }
    }

    /// The copies that take requests: the service's own port alone when it answers,
    /// else every copy port that answers.
    fn taking_requests(&self) -> Vec<usize> {
        if self.copies[SERVICE_PORT].up {
            return vec![SERVICE_PORT];
        }
        (SERVICE_PORT + 1..self.copies.len())
            .filter(|&index| self.copies[index].up)
            .collect()
    }

    pub(super) fn has_copies(&self) -> bool {
        !self.taking_requests().is_empty()
    }

    pub(super) fn snapshot(&self) -> Vec<(u16, bool, bool)> {
        let copies = self
            .taking_requests()
            .into_iter()
            .map(|index| &self.copies[index]);
        copies
            .map(|copy| (copy.port, copy.busy, copy.hung))
            .collect()
    }

    pub(super) fn port(&self, index: usize) -> u16 {
        self.copies[index].port
    }

    /// The ports to probe now: the service's own, and the copy ports.
    pub(super) fn ports(&self) -> Vec<u16> {
        self.copies.iter().map(|copy| copy.port).collect()
    }

    /// What each port said it is, in the order of `ports`.
    pub(super) fn identities(&self) -> Vec<Option<Identity>> {
        self.copies
            .iter()
            .map(|copy| copy.identity.clone())
            .collect()
    }

    /// The other program on the service's own port, if one holds it.
    pub(super) fn foreign_on_service_port(&self) -> Option<ForeignProgram> {
        match &self.copies[SERVICE_PORT].identity {
            Some(Identity::Foreign(foreign)) => Some(foreign.clone()),
            _ => None,
        }
    }

    /// Records a probe round, in the order of `ports`: a port takes requests when it accepts
    /// connections and said it is the service. Returns the other programs newly found.
    pub(super) fn probed(&mut self, probed: Vec<Probed>) -> Vec<ForeignProgram> {
        let mut found = Vec::new();
        for (copy, probed) in self.copies.iter_mut().zip(probed) {
            copy.up = probed.takes_requests();
            let known_foreign = matches!(copy.identity, Some(Identity::Foreign(_)));
            match &probed.identity {
                Some(Identity::Foreign(foreign)) if !known_foreign => found.push(foreign.clone()),
                _ => {}
            }
            copy.identity = probed.identity;
        }
        self.probed_once = true;
        found
    }

    pub(super) fn enqueue(&mut self) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        self.waiting.push_back(ticket);
        ticket
    }

    pub(super) fn leave_queue(&mut self, ticket: u64) {
        self.waiting.retain(|waiting| *waiting != ticket);
    }

    /// The copy `ticket` takes now, when it is first in the queue and a copy is free.
    pub(super) fn take_turn(&mut self, ticket: u64, now: Instant, tried: &[u16]) -> Option<usize> {
        if self.waiting.front() != Some(&ticket) {
            return None;
        }
        let index = self.free_copy(tried)?;
        self.waiting.pop_front();
        let copy = &mut self.copies[index];
        copy.busy = true;
        copy.leased_at = Some(now);
        Some(index)
    }

    /// The first free copy, searched round from the pool's offset: one whose port is
    /// not in `tried` first, else any.
    fn free_copy(&self, tried: &[u16]) -> Option<usize> {
        let taking = self.taking_requests();
        let start = self.offset % taking.len().max(1);
        let round: Vec<usize> = taking
            .iter()
            .cycle()
            .skip(start)
            .take(taking.len())
            .copied()
            .filter(|&index| !self.copies[index].busy && !self.copies[index].hung)
            .collect();
        let untried = round
            .iter()
            .copied()
            .find(|&index| !tried.contains(&self.copies[index].port));
        untried.or_else(|| round.first().copied())
    }

    pub(super) fn expect_answer_within(&mut self, index: usize, wait: Duration) {
        let copy = &mut self.copies[index];
        copy.hung_after = copy
            .leased_at
            .map(|leased| leased + wait * HUNG_AFTER_WAITS);
    }

    /// Marks hung every held copy past its time; returns each newly hung port and how long it waited.
    pub(super) fn find_hung(&mut self, now: Instant) -> Vec<(u16, Duration)> {
        let late = self
            .copies
            .iter_mut()
            .filter(|copy| !copy.hung && copy.hung_after.is_some_and(|after| now >= after));
        late.map(|copy| {
            copy.hung = true;
            (
                copy.port,
                copy.leased_at.map_or(Duration::ZERO, |leased| now - leased),
            )
        })
        .collect()
    }

    /// Frees the copy; one that refused the connection is down until a probe sees it again.
    /// Returns whether the copy had been hung.
    pub(super) fn release(&mut self, index: usize, refused: bool) -> bool {
        let copy = &mut self.copies[index];
        let was_hung = copy.hung;
        copy.busy = false;
        copy.hung = false;
        copy.leased_at = None;
        copy.hung_after = None;
        copy.up &= !refused;
        if refused {
            copy.identity = None;
        }
        was_hung
    }
}

/// What one probe round found on one port.
pub(super) struct Probed {
    pub(super) accepts: bool,
    /// What the program said it is; `None` when it did not answer in time.
    pub(super) identity: Option<Identity>,
}

impl Probed {
    pub(super) fn closed() -> Self {
        Self { accepts: false, identity: None }
    }

    pub(super) fn takes_requests(&self) -> bool {
        self.accepts && self.identity == Some(Identity::Service)
    }
}
