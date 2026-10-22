//! The permissions gate: dictation by hotkey runs only while every permission is granted.

use crate::core::application::PermissionProbe;
use crate::core::domain::{Permission, PermissionStatus};

/// The status of every permission at one moment, and whom the user grants them to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionReport {
    pub statuses: Vec<(Permission, PermissionStatus)>,
    pub responsible_app: Option<String>,
}

impl PermissionReport {
    pub fn status(&self, permission: Permission) -> PermissionStatus {
        self.statuses
            .iter()
            .find(|(p, _)| *p == permission)
            .map_or(PermissionStatus::NotDetermined, |(_, status)| *status)
    }

    pub fn all_granted(&self) -> bool {
        self.missing().is_empty()
    }

    /// The permissions not granted, in the order of [`Permission::ALL`].
    pub fn missing(&self) -> Vec<Permission> {
        Permission::ALL
            .into_iter()
            .filter(|p| self.status(*p) != PermissionStatus::Granted)
            .collect()
    }

    /// The permissions granted in `earlier` and not granted now.
    pub fn revoked_since(&self, earlier: &Self) -> Vec<Permission> {
        self.missing()
            .into_iter()
            .filter(|p| earlier.status(*p) == PermissionStatus::Granted)
            .collect()
    }

    /// One log line naming every permission and its status.
    pub fn log_line(&self) -> String {
        let statuses: Vec<String> = Permission::ALL
            .into_iter()
            .map(|p| format!("{} {}", p.name().to_lowercase(), self.status(p).words()))
            .collect();
        format!("permissions: {}", statuses.join(", "))
    }
}

/// Checks and requests the permissions through the probe.
pub struct PermissionsGate {
    probe: Box<dyn PermissionProbe>,
}

impl PermissionsGate {
    pub fn new(probe: Box<dyn PermissionProbe>) -> Self {
        Self { probe }
    }

    pub fn report(&self) -> PermissionReport {
        PermissionReport {
            statuses: Permission::ALL
                .into_iter()
                .map(|p| (p, self.probe.status(p)))
                .collect(),
            responsible_app: self.probe.responsible_app(),
        }
    }

    pub fn request(&self, permission: Permission) {
        self.probe.request(permission);
    }

    pub fn settings_url(&self, permission: Permission) -> String {
        self.probe.settings_url(permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::permission_probe::MockPermissionProbe;

    fn probe(
        microphone: PermissionStatus,
        accessibility: PermissionStatus,
        input_monitoring: PermissionStatus,
        responsible_app: Option<&'static str>,
    ) -> PermissionsGate {
        let mut mock = MockPermissionProbe::new();
        mock.expect_status()
            .returning(move |permission| match permission {
                Permission::Microphone => microphone,
                Permission::Accessibility => accessibility,
                Permission::InputMonitoring => input_monitoring,
            });
        mock.expect_responsible_app()
            .returning(move || responsible_app.map(String::from));
        PermissionsGate::new(Box::new(mock))
    }

    use PermissionStatus::{Denied, Granted, NotDetermined};

    #[test]
    fn should_report_every_permission_with_its_status() {
        // Given
        let gate = probe(Granted, Denied, NotDetermined, None);
        // When
        let report = gate.report();
        // Then
        assert_eq!(
            report.statuses,
            vec![
                (Permission::Microphone, Granted),
                (Permission::Accessibility, Denied),
                (Permission::InputMonitoring, NotDetermined),
            ]
        );
        assert_eq!(report.status(Permission::Accessibility), Denied);
    }

    #[test]
    fn should_open_only_when_every_permission_is_granted() {
        assert!(probe(Granted, Granted, Granted, None)
            .report()
            .all_granted());
        assert!(!probe(Granted, Granted, NotDetermined, None)
            .report()
            .all_granted());
    }

    #[test]
    fn should_list_the_missing_permissions_in_order() {
        // Given
        let gate = probe(Denied, Granted, NotDetermined, None);
        // When
        let missing = gate.report().missing();
        // Then
        assert_eq!(
            missing,
            vec![Permission::Microphone, Permission::InputMonitoring]
        );
    }

    #[test]
    fn should_name_the_responsible_app() {
        let report = probe(Granted, Granted, Granted, Some("iTerm2")).report();
        assert_eq!(report.responsible_app.as_deref(), Some("iTerm2"));
    }

    #[test]
    fn should_tell_the_permissions_revoked_since_an_earlier_report() {
        // Given
        let earlier = probe(Granted, Granted, NotDetermined, None).report();
        let now = probe(Granted, Denied, Denied, None).report();
        // When
        let revoked = now.revoked_since(&earlier);
        // Then
        assert_eq!(revoked, vec![Permission::Accessibility]);
    }

    #[test]
    fn should_name_every_permission_and_its_status_in_one_log_line() {
        // Given
        let report = probe(Granted, Denied, NotDetermined, None).report();
        // When
        let line = report.log_line();
        // Then
        assert_eq!(
            line,
            "permissions: microphone granted, accessibility not granted, \
             input monitoring not asked yet"
        );
    }
}
