//! One probe round: which ports accept connections, and what the program on a newly
//! accepting port says it is.

use std::time::Instant;

use super::state::Probed;
use super::Inner;
use crate::probe::Identity;

impl Inner {
    /// Probes the service's own port, and the copy ports only when it takes no requests; a
    /// port missed once is probed once more at once, as a copy busy decoding under load
    /// answers a probe late, and leaves only when both probes miss. A port that accepts
    /// connections is asked what it is once, until it stops accepting them.
    pub(super) fn probe_round(&self) {
        let (ports, known) = {
            let state = self.lock();
            (state.ports(), state.identities())
        };
        let service = self.probe_port(ports[0], known[0].as_ref(), false);
        let service_takes_requests = service.takes_requests();
        let copies = ports[1..].iter().zip(&known[1..]).map(|(port, known)| {
            if service_takes_requests {
                Probed::closed()
            } else {
                self.probe_port(*port, known.as_ref(), true)
            }
        });
        let probed: Vec<Probed> = std::iter::once(service).chain(copies).collect();
        let mut state = self.lock();
        for foreign in state.probed(probed) {
            log::warn!(
                "port {} of the service at {} is held by {foreign}, which is not the service: no request goes there",
                foreign.port,
                self.address
            );
        }
        for (port, waited) in state.find_hung(Instant::now()) {
            log::warn!(
                "copy {port} hung: no answer in {} s (service at {})",
                waited.as_secs(),
                self.address
            );
        }
        self.changed.notify_all();
    }

    fn probe_port(&self, port: u16, known: Option<&Identity>, twice: bool) -> Probed {
        let accepts = self.probe.answers(port) || (twice && self.probe.answers(port));
        if !accepts {
            return Probed::closed();
        }
        let identity = known.cloned().or_else(|| match self.probe.identify(port) {
            Identity::Unknown => None,
            identity => Some(identity),
        });
        Probed { accepts, identity }
    }
}
