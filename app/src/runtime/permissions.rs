//! How the runtime watches the permissions: the gate, how often it asks, and what it heard.

use std::sync::Arc;
use std::time::Duration;

use crate::core::application::{PermissionReport, PermissionsGate};
use crate::core::domain::Permission;

use super::RuntimeState;

/// How often the permissions are checked while the pipeline runs.
pub const CHECK_WHILE_READY: Duration = Duration::from_secs(5);
/// How often the permissions are checked while the runtime waits for them.
pub const CHECK_WHILE_WAITING: Duration = Duration::from_secs(2);

/// The gate the runtime asks, and how often.
#[derive(Clone)]
pub struct PermissionsWatch {
    pub gate: Arc<PermissionsGate>,
    pub while_ready: Duration,
    pub while_waiting: Duration,
}

impl PermissionsWatch {
    pub fn new(gate: Arc<PermissionsGate>) -> Self {
        Self {
            gate,
            while_ready: CHECK_WHILE_READY,
            while_waiting: CHECK_WHILE_WAITING,
        }
    }
}

/// What a check of the permissions asks of the runtime.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    Unchanged,
    /// Every permission is granted again: start the pipeline.
    Open,
    /// The pipeline runs and a permission is no longer granted: stop it.
    Close(RuntimeState),
    /// Still waiting, for other permissions than before.
    StillWaiting(RuntimeState),
}

/// The watch and the report its last check heard.
pub(super) struct Watched {
    watch: PermissionsWatch,
    last: PermissionReport,
    /// Revoked while the pipeline ran and not granted again since.
    revoked: Vec<Permission>,
}

impl Watched {
    /// Checks at once and logs what it heard.
    pub(super) fn start(watch: PermissionsWatch) -> Self {
        let last = watch.gate.report();
        log::info!("{}", last.log_line());
        Self {
            watch,
            last,
            revoked: Vec::new(),
        }
    }

    pub(super) fn all_granted(&self) -> bool {
        self.last.all_granted()
    }

    pub(super) fn interval(&self, pipeline_runs: bool) -> Duration {
        if pipeline_runs {
            self.watch.while_ready
        } else {
            self.watch.while_waiting
        }
    }

    /// The state while the runtime waits for the permissions heard last.
    pub(super) fn waiting(&self) -> RuntimeState {
        RuntimeState::WaitingForPermissions {
            missing: self.last.missing(),
            revoked: self.revoked.clone(),
        }
    }

    /// Checks again; logs a change and tells what the runtime should do about it.
    pub(super) fn check(&mut self, pipeline_runs: bool) -> Verdict {
        let report = self.watch.gate.report();
        if report == self.last {
            return Verdict::Unchanged;
        }
        log::info!("{}", report.log_line());
        let newly_revoked = report.revoked_since(&self.last);
        let missing = report.missing();
        self.revoked.retain(|p| missing.contains(p));
        for permission in newly_revoked {
            if pipeline_runs && !self.revoked.contains(&permission) {
                self.revoked.push(permission);
            }
        }
        let was_open = self.last.all_granted();
        self.last = report;
        match (pipeline_runs, self.last.all_granted()) {
            (true, false) => Verdict::Close(self.waiting()),
            (false, true) => Verdict::Open,
            (false, false) if !was_open => Verdict::StillWaiting(self.waiting()),
            _ => Verdict::Unchanged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::permission_probe::MockPermissionProbe;
    use crate::core::domain::PermissionStatus;
    use std::sync::Mutex;

    type Statuses = Arc<Mutex<[PermissionStatus; 3]>>;

    fn watched(statuses: &Statuses) -> Watched {
        let shared = Arc::clone(statuses);
        let mut probe = MockPermissionProbe::new();
        probe.expect_status().returning(move |permission| {
            let index = Permission::ALL
                .iter()
                .position(|p| *p == permission)
                .unwrap();
            shared.lock().unwrap()[index]
        });
        probe.expect_responsible_app().returning(|| None);
        Watched::start(PermissionsWatch::new(Arc::new(PermissionsGate::new(
            Box::new(probe),
        ))))
    }

    use PermissionStatus::{Denied, Granted, NotDetermined};

    #[test]
    fn should_check_every_5_s_while_ready_and_every_2_s_while_waiting() {
        // Given
        let gate = Arc::new(PermissionsGate::new(Box::new(MockPermissionProbe::new())));
        // When
        let watch = PermissionsWatch::new(gate);
        // Then
        assert_eq!(watch.while_ready, Duration::from_secs(5));
        assert_eq!(watch.while_waiting, Duration::from_secs(2));
    }

    #[test]
    fn should_open_when_the_last_missing_permission_is_granted() {
        // Given
        let statuses: Statuses = Arc::new(Mutex::new([Granted, Granted, NotDetermined]));
        let mut watched = watched(&statuses);
        statuses.lock().unwrap()[2] = Granted;
        // When
        let verdict = watched.check(false);
        // Then
        assert_eq!(verdict, Verdict::Open);
    }

    #[test]
    fn should_close_with_the_revoked_permission_while_the_pipeline_runs() {
        // Given
        let statuses: Statuses = Arc::new(Mutex::new([Granted, Granted, Granted]));
        let mut watched = watched(&statuses);
        statuses.lock().unwrap()[1] = Denied;
        // When
        let verdict = watched.check(true);
        // Then
        assert_eq!(
            verdict,
            Verdict::Close(RuntimeState::WaitingForPermissions {
                missing: vec![Permission::Accessibility],
                revoked: vec![Permission::Accessibility],
            })
        );
    }

    #[test]
    fn should_tell_a_new_missing_list_while_waiting() {
        // Given
        let statuses: Statuses = Arc::new(Mutex::new([Denied, Denied, Granted]));
        let mut watched = watched(&statuses);
        statuses.lock().unwrap()[0] = Granted;
        // When
        let verdict = watched.check(false);
        // Then
        assert_eq!(
            verdict,
            Verdict::StillWaiting(RuntimeState::WaitingForPermissions {
                missing: vec![Permission::Accessibility],
                revoked: vec![],
            })
        );
    }

    #[test]
    fn should_tell_nothing_when_nothing_changed() {
        let statuses: Statuses = Arc::new(Mutex::new([Granted, Granted, Granted]));
        let mut watched = watched(&statuses);
        assert_eq!(watched.check(true), Verdict::Unchanged);
    }
}
