//! A copy that answers on the service's port but speaks another protocol version than the app:
//! never adopted, its program stopped and the app's own started in its place.

use std::time::Instant;

use crate::core::domain::ServiceState;

use super::{HolderStop, OtherProtocol, Senses, Supervision};

impl Supervision {
    /// Sends an adopted copy that refused a client back to the decision to adopt or replace
    /// it; any other state is left as it is.
    pub fn recheck(&mut self, now: Instant) {
        if self.state == ServiceState::Adopted {
            log::info!(
                "services: {} at port {} refused a client — checking it again",
                self.spec.label,
                self.spec.port
            );
            self.since = now;
            self.state = ServiceState::Pending;
        }
    }

    /// Adopts the copy that answers on the port, or replaces it when it speaks another protocol.
    pub(super) fn adopt_or_replace(&mut self, senses: &Senses) {
        match senses.probe.other_protocol(self.spec.kind, self.spec.port) {
            None => {
                log::info!(
                    "services: adopted {} at port {}",
                    self.spec.label,
                    self.spec.port
                );
                self.adopt(senses.now);
            }
            Some(other) => self.replace_stale(senses, other),
        }
    }

    fn replace_stale(&mut self, senses: &Senses, other: OtherProtocol) {
        let (label, port) = (&self.spec.label, self.spec.port);
        let OtherProtocol { service, app } = other;
        match senses.launcher.stop_holder(&self.spec) {
            HolderStop::Stopped(pid) => {
                log::warn!(
                    "services: {label} at port {port} speaks protocol version {service}, the app version {app} — replaced it (stopped pid {pid})"
                );
                self.spawn(senses);
            }
            HolderStop::Foreign(holder) => self.fail(format!(
                "port {port} is held by {holder}, which refuses the app's protocol version {app} and is not the {}; left running",
                self.spec.kind.name()
            )),
        }
    }
}

#[cfg(test)]
#[path = "stale_tests.rs"]
mod tests;
