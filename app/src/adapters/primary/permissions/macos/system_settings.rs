//! What Open System Settings does in a row: the steps of [`settings_steps`], and the macOS
//! version the rows name their panes by.

use std::sync::Arc;

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSArray, NSBundle, NSProcessInfo, NSString, NSURL};

use super::mapping::{settings_steps, SettingsStep};
use crate::core::application::PermissionsGate;
use crate::core::domain::Permission;

/// The major version of the running macOS.
pub fn macos_major() -> u64 {
    let version = NSProcessInfo::processInfo().operatingSystemVersion();
    u64::try_from(version.majorVersion).unwrap_or_default()
}

/// Runs the steps of Open System Settings in `permission`'s row.
pub fn open(gate: &Arc<PermissionsGate>, permission: Permission) {
    let bundle = NSBundle::mainBundle();
    let from_bundle = bundle.bundleIdentifier().is_some();
    for step in settings_steps(permission, from_bundle) {
        match step {
            SettingsStep::Request => request(Arc::clone(gate), permission),
            SettingsStep::OpenPane => open_pane(&gate.settings_url(permission)),
            SettingsStep::RevealBundle => {
                let urls = NSArray::from_retained_slice(&[bundle.bundleURL()]);
                NSWorkspace::sharedWorkspace().activateFileViewerSelectingURLs(&urls);
            }
        }
    }
}

/// Asks the system again off the main thread: the call may wait for the user's answer.
fn request(gate: Arc<PermissionsGate>, permission: Permission) {
    let spawned = std::thread::Builder::new()
        .name("permission-request".into())
        .spawn(move || gate.request(permission));
    if let Err(e) = spawned {
        log::warn!("permissions: {permission} not requested again: {e}");
    }
}

fn open_pane(url: &str) {
    let opened = NSURL::URLWithString(&NSString::from_str(url))
        .is_some_and(|url| NSWorkspace::sharedWorkspace().openURL(&url));
    if !opened {
        log::warn!("permissions: System Settings not opened at {url}");
    }
}
