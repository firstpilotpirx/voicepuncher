//! Which copies take requests and which are held, behind the pool's lock.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::address::ServiceAddress;

/// A copy is hung once it held one request for this many times the request's wait.
const HUNG_AFTER_WAITS: u32 = 2;

struct Copy {
    port: u16,
    up: bool,
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
}

const SERVICE_PORT: usize = 0;

impl State {
    pub(super) fn new(address: &ServiceAddress, offset: usize) -> Self {
        let ports = std::iter::once(address.port()).chain(address.copy_ports());
        let copies = ports
            .map(|port| Copy {
                port,
                up: false,
                busy: false,
                leased_at: None,
                hung_after: None,
                hung: false,
            })
            .collect();
        Self {
            copies,
            waiting: VecDeque::new(),
            next_ticket: 0,
            offset,
        }
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

    /// Records which ports accept connections, in the order of `ports`.
    pub(super) fn probed(&mut self, answers: &[bool]) {
        for (copy, up) in self.copies.iter_mut().zip(answers) {
            copy.up = *up;
        }
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
        was_hung
    }
}
