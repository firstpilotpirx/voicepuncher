//! One copy held for one request, freed when the request ends.

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::{Finish, Inner, Ran};

/// One copy, held for one request; dropping it frees the copy as answered.
pub struct Lease {
    pub(super) inner: Arc<Inner>,
    pub(super) index: usize,
    pub(super) port: u16,
    pub(super) url: String,
    pub(super) released: bool,
}

impl Lease {
    /// The copy's port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The copy's address: the service's address with the copy's port.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The copy answered: it takes the next request.
    pub fn finished(mut self) {
        self.release(Finish::Answered);
    }

    /// The copy refused the connection: no request goes to it until it accepts connections again.
    pub fn refused(mut self) {
        self.release(Finish::Refused);
    }

    /// Runs `request` on the copy's address on a thread of its own and waits at most `wait`
    /// for it. The copy stays leased until the request ends, however long it takes, and is
    /// reported hung once that passes twice `wait`.
    pub fn run<T: Send + 'static>(
        self,
        wait: Duration,
        request: impl FnOnce(&str) -> (T, Finish) + Send + 'static,
    ) -> Ran<T> {
        self.inner.expect_answer_within(self.index, wait);
        let (answer_tx, answer_rx) = mpsc::sync_channel(1);
        let spawned = thread::Builder::new()
            .name(format!("copy-{}", self.port))
            .spawn(move || {
                let mut lease = self;
                let (answer, finish) = request(&lease.url);
                lease.release(finish);
                let _ = answer_tx.send(answer);
            });
        if spawned.is_err() {
            return Ran::GaveUp;
        }
        answer_rx
            .recv_timeout(wait)
            .map_or(Ran::GaveUp, Ran::Answered)
    }

    fn release(&mut self, finish: Finish) {
        if !self.released {
            self.released = true;
            self.inner.release(self.index, finish);
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.release(Finish::Answered);
    }
}
