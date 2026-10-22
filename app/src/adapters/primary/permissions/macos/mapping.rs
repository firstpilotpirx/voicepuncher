//! What the Permissions window shows, as pure functions of the gate's report, the
//! permissions requested and not answered yet, and the permissions revoked.

use crate::core::application::PermissionReport;
use crate::core::domain::{Permission, PermissionStatus};

/// What a row shows: the status the gate reported, or Checking… while a request is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    Status(PermissionStatus),
    Checking,
}

/// The colour of a badge's dot; a pending request shows a spinner instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeMark {
    Green,
    Red,
    Grey,
    Spinner,
}

/// The one button of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowButton {
    Request,
    OpenSystemSettings,
}

/// One row of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowModel {
    pub permission: Permission,
    pub shown: Shown,
}

/// Everything the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowModel {
    pub rows: Vec<RowModel>,
    pub developer_line: Option<String>,
    pub revoked_line: Option<String>,
    pub continue_enabled: bool,
}

pub const TITLE: &str = "Permissions";
pub const HEADING: &str = "voicepuncher needs three permissions";
pub const SUBHEADING: &str =
    "Grant each one below. voicepuncher checks again by itself; no restart is needed.";

impl WindowModel {
    /// The window for `report`; a permission in `pending` that is still not asked yet shows
    /// Checking…, and `revoked` names the permissions revoked while dictation ran.
    pub fn new(report: &PermissionReport, pending: &[Permission], revoked: &[Permission]) -> Self {
        let rows = Permission::ALL
            .into_iter()
            .map(|permission| {
                let status = report.status(permission);
                let checking =
                    status == PermissionStatus::NotDetermined && pending.contains(&permission);
                RowModel {
                    permission,
                    shown: if checking {
                        Shown::Checking
                    } else {
                        Shown::Status(status)
                    },
                }
            })
            .collect();
        Self {
            rows,
            developer_line: report.responsible_app.as_deref().map(|app| {
                format!("voicepuncher runs from {app} — grant the permissions to {app}.")
            }),
            revoked_line: revoked_line(revoked),
            continue_enabled: report.all_granted(),
        }
    }
}

fn revoked_line(revoked: &[Permission]) -> Option<String> {
    let names: Vec<&str> = revoked.iter().map(|p| p.name()).collect();
    match names.as_slice() {
        [] => None,
        [one] => Some(format!("{one} was revoked — dictation stopped")),
        [first @ .., last] => Some(format!(
            "{} and {last} were revoked — dictation stopped",
            first.join(", ")
        )),
    }
}

/// The badge of a row: its words and its mark.
pub fn badge(shown: Shown) -> (&'static str, BadgeMark) {
    match shown {
        Shown::Status(PermissionStatus::Granted) => ("Granted", BadgeMark::Green),
        Shown::Status(PermissionStatus::Denied) => ("Not granted", BadgeMark::Red),
        Shown::Status(PermissionStatus::NotDetermined) => ("Not asked yet", BadgeMark::Grey),
        Shown::Checking => ("Checking…", BadgeMark::Spinner),
    }
}

/// The button of a row; none when the permission is granted or a request is pending.
pub fn button(shown: Shown) -> Option<RowButton> {
    match shown {
        Shown::Status(PermissionStatus::NotDetermined) => Some(RowButton::Request),
        Shown::Status(PermissionStatus::Denied) => Some(RowButton::OpenSystemSettings),
        Shown::Status(PermissionStatus::Granted) | Shown::Checking => None,
    }
}

impl RowButton {
    pub fn title(self) -> &'static str {
        match self {
            Self::Request => "Request…",
            Self::OpenSystemSettings => "Open System Settings",
        }
    }
}

/// Why the app needs `permission`, in one line.
pub fn why(permission: Permission) -> &'static str {
    match permission {
        Permission::Microphone => "to hear your dictation",
        Permission::Accessibility => "to paste the text into the app you are typing in",
        Permission::InputMonitoring => "to notice the hotkey in any app",
    }
}

/// The SF Symbol of `permission`'s row.
pub fn symbol(permission: Permission) -> &'static str {
    match permission {
        Permission::Microphone => "mic",
        Permission::Accessibility => "accessibility",
        Permission::InputMonitoring => "keyboard",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(statuses: [PermissionStatus; 3], responsible_app: Option<&str>) -> PermissionReport {
        PermissionReport {
            statuses: Permission::ALL.into_iter().zip(statuses).collect(),
            responsible_app: responsible_app.map(str::to_owned),
        }
    }

    use PermissionStatus::{Denied, Granted, NotDetermined};

    #[test]
    fn should_show_each_status_as_its_badge() {
        assert_eq!(badge(Shown::Status(Granted)), ("Granted", BadgeMark::Green));
        assert_eq!(
            badge(Shown::Status(Denied)),
            ("Not granted", BadgeMark::Red)
        );
        assert_eq!(
            badge(Shown::Status(NotDetermined)),
            ("Not asked yet", BadgeMark::Grey)
        );
        assert_eq!(badge(Shown::Checking), ("Checking…", BadgeMark::Spinner));
    }

    #[test]
    fn should_offer_request_for_not_asked_yet_settings_for_denied_and_nothing_for_granted() {
        assert_eq!(
            button(Shown::Status(NotDetermined)),
            Some(RowButton::Request)
        );
        assert_eq!(
            button(Shown::Status(Denied)),
            Some(RowButton::OpenSystemSettings)
        );
        assert_eq!(button(Shown::Status(Granted)), None);
        assert_eq!(button(Shown::Checking), None);
        assert_eq!(RowButton::Request.title(), "Request…");
        assert_eq!(
            RowButton::OpenSystemSettings.title(),
            "Open System Settings"
        );
    }

    #[test]
    fn should_name_each_permission_with_why_and_symbol() {
        assert_eq!(why(Permission::Microphone), "to hear your dictation");
        assert_eq!(
            why(Permission::Accessibility),
            "to paste the text into the app you are typing in"
        );
        assert_eq!(
            why(Permission::InputMonitoring),
            "to notice the hotkey in any app"
        );
        assert_eq!(symbol(Permission::Microphone), "mic");
        assert_eq!(symbol(Permission::Accessibility), "accessibility");
        assert_eq!(symbol(Permission::InputMonitoring), "keyboard");
    }

    #[test]
    fn should_show_one_row_per_permission_in_order() {
        let model = WindowModel::new(&report([Granted, Denied, NotDetermined], None), &[], &[]);
        let rows: Vec<_> = model.rows.iter().map(|r| (r.permission, r.shown)).collect();
        assert_eq!(
            rows,
            vec![
                (Permission::Microphone, Shown::Status(Granted)),
                (Permission::Accessibility, Shown::Status(Denied)),
                (Permission::InputMonitoring, Shown::Status(NotDetermined)),
            ]
        );
    }

    #[test]
    fn should_show_checking_while_a_request_is_not_answered() {
        let pending = [Permission::InputMonitoring];
        let asking = WindowModel::new(
            &report([Granted, Granted, NotDetermined], None),
            &pending,
            &[],
        );
        assert_eq!(asking.rows[2].shown, Shown::Checking);
        let answered = WindowModel::new(&report([Granted, Granted, Denied], None), &pending, &[]);
        assert_eq!(answered.rows[2].shown, Shown::Status(Denied));
    }

    #[test]
    fn should_name_the_responsible_app_when_run_from_another_app() {
        let model = WindowModel::new(&report([Granted; 3], Some("iTerm2")), &[], &[]);
        assert_eq!(
            model.developer_line.as_deref(),
            Some("voicepuncher runs from iTerm2 — grant the permissions to iTerm2.")
        );
    }

    #[test]
    fn should_name_no_other_app_when_run_from_the_bundle() {
        let model = WindowModel::new(&report([Granted; 3], None), &[], &[]);
        assert_eq!(model.developer_line, None);
    }

    #[test]
    fn should_say_which_permission_was_revoked() {
        let model = WindowModel::new(
            &report([Granted, Denied, Granted], None),
            &[],
            &[Permission::Accessibility],
        );
        assert_eq!(
            model.revoked_line.as_deref(),
            Some("Accessibility was revoked — dictation stopped")
        );
        let none = WindowModel::new(&report([Granted, Denied, Granted], None), &[], &[]);
        assert_eq!(none.revoked_line, None);
    }

    #[test]
    fn should_enable_continue_only_when_every_permission_is_granted() {
        let missing = WindowModel::new(&report([Granted, Denied, Granted], None), &[], &[]);
        assert!(!missing.continue_enabled);
        let all = WindowModel::new(&report([Granted; 3], None), &[], &[]);
        assert!(all.continue_enabled);
    }
}
