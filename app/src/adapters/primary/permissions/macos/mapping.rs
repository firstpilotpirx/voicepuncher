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
    /// The line under the permission's name.
    pub line: String,
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
                    line: why(permission).to_owned(),
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

impl WindowModel {
    /// The rows' lines for macOS `macos_major`, with the hint in the rows whose System
    /// Settings pane was `opened` from the window.
    #[must_use]
    pub fn with_lines(mut self, opened: &[Permission], macos_major: u64) -> Self {
        for row in &mut self.rows {
            let was_opened = opened.contains(&row.permission);
            row.line = row_line(row.permission, row.shown, was_opened, macos_major);
        }
        self
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

/// How often the open window asks the gate again, in milliseconds.
pub const REFRESH_MS: i64 = 1_000;

/// What Open System Settings does, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsStep {
    /// Asks the system for the permission again.
    Request,
    /// Opens the permission's pane of System Settings.
    OpenPane,
    /// Shows the app's bundle in Finder, to add it with +.
    RevealBundle,
}

/// The pane of System Settings that holds `permission` on macOS `macos_major`.
pub fn pane_name(permission: Permission, macos_major: u64) -> &'static str {
    match permission {
        Permission::Microphone => "Microphone",
        Permission::Accessibility if macos_major >= 27 => "Device Control and Data Access",
        Permission::Accessibility => "Accessibility",
        Permission::InputMonitoring => "Input Monitoring",
    }
}

/// The line under a row's name: why the app needs it, where to switch it on, or the hint
/// once System Settings was opened from the row and the permission still is not granted.
pub fn row_line(permission: Permission, shown: Shown, opened: bool, macos_major: u64) -> String {
    match (shown, opened, permission) {
        (Shown::Status(PermissionStatus::Denied), true, Permission::InputMonitoring) => {
            "Not listed? Add voicepuncher with +.".to_owned()
        }
        (Shown::Status(PermissionStatus::Denied), true, _) => {
            "Already on? Remove voicepuncher with − and add it again with +.".to_owned()
        }
        (Shown::Status(PermissionStatus::Denied), false, _) => {
            format!(
                "Switch voicepuncher on in {}",
                pane_name(permission, macos_major)
            )
        }
        _ => why(permission).to_owned(),
    }
}

/// The steps of Open System Settings in `permission`'s row; `from_bundle` when the app runs
/// from its bundle.
pub fn settings_steps(permission: Permission, from_bundle: bool) -> Vec<SettingsStep> {
    if permission != Permission::InputMonitoring {
        return vec![SettingsStep::OpenPane];
    }
    let mut steps = vec![SettingsStep::Request, SettingsStep::OpenPane];
    if from_bundle {
        steps.push(SettingsStep::RevealBundle);
    }
    steps
}

#[cfg(test)]
mod tests;
