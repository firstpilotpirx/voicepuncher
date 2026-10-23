//! One supervised service: a state machine the clock drives through [`Supervision::tick`],
//! with the numbers the service scripts use.

use std::time::{Duration, Instant};

use crate::core::domain::{ServiceSpec, ServiceState, ServiceStatus};

use super::{ProbeAnswer, ServiceLauncher, ServiceProbe, ServiceProcess};

/// How long a started service has to answer, and how long a port that answers as nothing
/// known is waited for before the decision to adopt gives up.
pub const READINESS_LIMIT: Duration = Duration::from_secs(60);
/// The pause before the first restart, doubled after every exit in a row.
pub const FIRST_PAUSE: Duration = Duration::from_secs(1);
pub const LONGEST_PAUSE: Duration = Duration::from_secs(30);
/// A run at least this long sets the pause back to the first one.
pub const LONG_RUN: Duration = Duration::from_secs(30);
/// How often a running service is checked.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// Checks failed in a row before a service counts as no longer answering.
pub const FAILED_CHECKS: u32 = 3;
/// Starts in a row that never answered before the service is Failed.
pub const FAILED_STARTS: u32 = 5;

/// What one tick may use: the launcher, the probe and the time.
pub struct Senses<'a> {
    pub launcher: &'a dyn ServiceLauncher,
    pub probe: &'a dyn ServiceProbe,
    pub now: Instant,
}

pub struct Supervision {
    spec: ServiceSpec,
    state: ServiceState,
    owned: bool,
    process: Option<Box<dyn ServiceProcess>>,
    /// When the current run, or the decision to adopt, began.
    since: Instant,
    /// When the next check or restart is due.
    next_at: Instant,
    failed_starts: u32,
    failed_checks: u32,
    pause: Duration,
    attempt: u32,
}

impl Supervision {
    pub fn new(spec: ServiceSpec, now: Instant) -> Self {
        Self {
            spec,
            state: ServiceState::Pending,
            owned: false,
            process: None,
            since: now,
            next_at: now,
            failed_starts: 0,
            failed_checks: 0,
            pause: FIRST_PAUSE,
            attempt: 0,
        }
    }

    pub fn status(&self) -> ServiceStatus {
        ServiceStatus {
            kind: self.spec.kind,
            label: self.spec.label.clone(),
            port: self.spec.port,
            state: self.state.clone(),
            owned: self.owned,
            log_file: self.spec.log_file.clone(),
        }
    }

    pub fn spec(&self) -> &ServiceSpec {
        &self.spec
    }

    /// One step of the supervision at `senses.now`.
    pub fn tick(&mut self, senses: &Senses) {
        match self.state {
            ServiceState::Pending => self.decide(senses),
            ServiceState::Starting { .. } => self.watch_start(senses),
            ServiceState::Ready => self.watch_owned(senses),
            ServiceState::Restarting { .. } if senses.now >= self.next_at => self.spawn(senses),
            ServiceState::Adopted => self.watch_adopted(senses),
            ServiceState::StoppedOutside => self.wait_for_return(senses),
            _ => {}
        }
    }

    /// Retries a Failed service from the start, or starts an owned one again at once.
    pub fn restart(&mut self, now: Instant) {
        if let ServiceState::Failed(_) = self.state {
            log::info!("services: {} — retry", self.spec.label);
            self.failed_starts = 0;
            self.pause = FIRST_PAUSE;
            self.since = now;
            self.state = ServiceState::Pending;
        } else if self.owned && self.state != ServiceState::Stopped {
            log::info!("services: {} — restart asked", self.spec.label);
            self.kill();
            self.pause = FIRST_PAUSE;
            self.next_at = now;
            self.state =
                ServiceState::Restarting { attempt: self.attempt + 1, pause: Duration::ZERO };
        }
    }

    /// Asks an owned process to end; false when there is none.
    pub fn ask_to_end(&mut self) -> bool {
        match self.process.as_mut() {
            Some(process) if self.owned => {
                process.terminate();
                true
            }
            _ => false,
        }
    }

    /// The owned process ended, or there is none.
    pub fn ended(&mut self) -> bool {
        self.exited().is_some() || self.process.is_none()
    }

    /// Kills what is left of an owned process and marks it stopped; adopted stays adopted.
    pub fn finish_stop(&mut self) {
        if self.process.is_some() {
            log::warn!(
                "services: {} did not end when asked — killed",
                self.spec.label
            );
            self.kill();
        }
        if self.owned {
            log::info!("services: {} stopped", self.spec.label);
            self.state = ServiceState::Stopped;
        }
    }

    fn decide(&mut self, senses: &Senses) {
        match senses.probe.answer(self.spec.kind, self.spec.port) {
            ProbeAnswer::Ready => {
                log::info!(
                    "services: adopted {} at port {}",
                    self.spec.label,
                    self.spec.port
                );
                self.next_at = senses.now + CHECK_INTERVAL;
                self.state = ServiceState::Adopted;
            }
            ProbeAnswer::Busy if senses.now.duration_since(self.since) >= READINESS_LIMIT => {
                self.fail(format!(
                    "port {} is held by a program that does not answer as the {}",
                    self.spec.port,
                    self.spec.kind.name()
                ));
            }
            ProbeAnswer::Busy => {}
            ProbeAnswer::Closed => match self.spec.problem.clone() {
                Some(problem) => self.fail(problem),
                None => self.spawn(senses),
            },
        }
    }

    fn spawn(&mut self, senses: &Senses) {
        self.owned = true;
        self.attempt += 1;
        self.since = senses.now;
        match senses.launcher.spawn(&self.spec) {
            Ok(process) => {
                log::info!(
                    "services: starting {} (pid {}, try {}), log {}",
                    self.spec.label,
                    process.id(),
                    self.attempt,
                    self.spec.log_file.display()
                );
                self.process = Some(process);
                self.state = ServiceState::Starting { attempt: self.attempt };
            }
            Err(e) => self.start_failed(senses.now, format!("{e:#}")),
        }
    }

    fn watch_start(&mut self, senses: &Senses) {
        if let Some(exit) = self.exited() {
            return self.start_failed(senses.now, format!("exited ({exit})"));
        }
        let warmed = (senses.probe.answer(self.spec.kind, self.spec.port) == ProbeAnswer::Ready)
            .then(|| senses.probe.warm_up(self.spec.kind, self.spec.port))
            .flatten();
        if let Some(first_transcription) = warmed {
            let took = senses.now.duration_since(self.since);
            log::info!(
                "services: {} ready in {:.1} s{}",
                self.spec.label,
                took.as_secs_f64(),
                transcription_note(first_transcription)
            );
            self.failed_starts = 0;
            self.failed_checks = 0;
            self.next_at = senses.now + CHECK_INTERVAL;
            self.state = ServiceState::Ready;
        } else if senses.now.duration_since(self.since) >= READINESS_LIMIT {
            self.kill();
            self.start_failed(senses.now, "not ready within 60 s".into());
        }
    }

    fn start_failed(&mut self, now: Instant, reason: String) {
        self.failed_starts += 1;
        if self.failed_starts < FAILED_STARTS {
            return self.schedule_restart(now, &reason);
        }
        self.fail(format!(
            "the {} could not start {} times in a row ({reason}); see {}",
            self.spec.kind.name(),
            self.failed_starts,
            self.spec.log_file.display()
        ));
    }

    fn fail(&mut self, reason: String) {
        log::error!("services: {} failed: {reason}", self.spec.label);
        self.state = ServiceState::Failed(reason);
    }

    fn watch_owned(&mut self, senses: &Senses) {
        if let Some(exit) = self.exited() {
            return self.schedule_restart(senses.now, &format!("exited ({exit})"));
        }
        if !self.spec.health_checked || senses.now < self.next_at {
            return;
        }
        self.next_at = senses.now + CHECK_INTERVAL;
        if senses.probe.answer(self.spec.kind, self.spec.port) == ProbeAnswer::Ready {
            self.failed_checks = 0;
            return;
        }
        self.failed_checks += 1;
        if self.failed_checks >= FAILED_CHECKS {
            self.failed_checks = 0;
            self.kill();
            self.schedule_restart(senses.now, "stopped answering its checks");
        }
    }

    fn schedule_restart(&mut self, now: Instant, reason: &str) {
        if now.duration_since(self.since) >= LONG_RUN {
            self.pause = FIRST_PAUSE;
        }
        log::warn!(
            "services: {} {reason}; restart in {} s",
            self.spec.label,
            self.pause.as_secs_f64()
        );
        self.next_at = now + self.pause;
        self.state = ServiceState::Restarting { attempt: self.attempt + 1, pause: self.pause };
        self.pause = (self.pause * 2).min(LONGEST_PAUSE);
    }

    fn watch_adopted(&mut self, senses: &Senses) {
        if senses.now < self.next_at {
            return;
        }
        self.next_at = senses.now + CHECK_INTERVAL;
        if senses.probe.answer(self.spec.kind, self.spec.port) != ProbeAnswer::Closed {
            self.failed_checks = 0;
            return;
        }
        self.failed_checks += 1;
        if self.failed_checks >= FAILED_CHECKS {
            log::warn!(
                "services: {} stopped — it was started outside the app",
                self.spec.label
            );
            self.state = ServiceState::StoppedOutside;
        }
    }

    fn wait_for_return(&mut self, senses: &Senses) {
        if senses.now < self.next_at {
            return;
        }
        self.next_at = senses.now + CHECK_INTERVAL;
        if senses.probe.answer(self.spec.kind, self.spec.port) == ProbeAnswer::Ready {
            log::info!("services: {} answers again — adopted", self.spec.label);
            self.failed_checks = 0;
            self.state = ServiceState::Adopted;
        }
    }

    /// How the owned process ended, once; it is then forgotten.
    fn exited(&mut self) -> Option<String> {
        let exit = self.process.as_mut()?.exited()?;
        self.process = None;
        Some(exit)
    }

    fn kill(&mut self) {
        if let Some(mut process) = self.process.take() {
            process.kill();
        }
    }
}

/// ", first transcription N ms" for a copy that was warmed up, nothing for a service that
/// transcribes nothing.
fn transcription_note(took: Duration) -> String {
    if took.is_zero() {
        String::new()
    } else {
        format!(", first transcription {} ms", took.as_millis())
    }
}

#[cfg(test)]
#[path = "supervision_tests.rs"]
mod tests;
