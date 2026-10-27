//! The watching of an adopted service: one the app found answering and did not start. One that
//! stops goes back to the decision, which starts the app's own on its port.

use std::time::Instant;

use crate::core::domain::ServiceState;

use super::{
    HolderStop, ProbeAnswer, Senses, Supervision, CHECK_INTERVAL, FAILED_CHECKS, FIRST_PAUSE,
};

impl Supervision {
    pub(super) fn watch_adopted(&mut self, senses: &Senses) {
        let Some(answer) = self.due_answer(senses) else {
            return;
        };
        if answer == ProbeAnswer::Ready {
            self.failed_checks = 0;
            return;
        }
        self.failed_checks += 1;
        if self.failed_checks < FAILED_CHECKS {
            return;
        }
        if answer == ProbeAnswer::Busy {
            self.stop_hung(senses);
        } else {
            log::warn!(
                "services: {} at port {} stopped — it was started outside the app; starting the app's own",
                self.spec.label,
                self.spec.port
            );
        }
        self.decide_again(senses);
    }

    /// The adopted service takes connections but no longer answers: its program is stopped,
    /// another program on its port is named and left running.
    fn stop_hung(&self, senses: &Senses) {
        let (label, port) = (&self.spec.label, self.spec.port);
        match senses.launcher.stop_holder(&self.spec) {
            HolderStop::Stopped(pid) => log::warn!(
                "services: {label} at port {port} stopped answering — stopped it (pid {pid}); starting the app's own"
            ),
            HolderStop::Foreign(holder) => log::warn!(
                "services: {label} at port {port} stopped answering; the port is held by {holder}, not its program — left running"
            ),
        }
    }

    /// Asks the port again whether to adopt or to start: after an adopted service stopped, and
    /// before every restart.
    pub(super) fn decide_again(&mut self, senses: &Senses) {
        self.since = senses.now;
        self.failed_checks = 0;
        self.state = ServiceState::Pending;
        self.decide(senses);
    }

    /// The service answers on its port without the app's process: adopted, its failed starts
    /// and its pause forgotten.
    pub(super) fn adopt(&mut self, now: Instant) {
        self.kill();
        self.owned = false;
        self.failed_starts = 0;
        self.failed_checks = 0;
        self.pause = FIRST_PAUSE;
        self.next_at = now + CHECK_INTERVAL;
        self.state = ServiceState::Adopted;
    }
}

#[cfg(test)]
#[path = "adopted_tests.rs"]
mod tests;
