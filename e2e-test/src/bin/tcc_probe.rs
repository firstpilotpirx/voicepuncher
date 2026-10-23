//! The permissions the desktop tests need (`just it-real`): prints each one macOS has not granted
//! to the process running them, with the app macOS attributes it to, and exits 1 when one is
//! missing, so the stage skips its tests instead of failing on a denied microphone,
//! Accessibility API or event tap.

use std::process::ExitCode;

use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::core::application::PermissionProbe;
use voicepuncher::core::domain::{Permission, PermissionStatus};

fn main() -> ExitCode {
    let probe = MacOsPermissionProbe::new();
    let missing: Vec<Permission> = Permission::ALL
        .into_iter()
        .filter(|permission| probe.status(*permission) != PermissionStatus::Granted)
        .collect();
    if missing.is_empty() {
        return ExitCode::SUCCESS;
    }
    let app = probe
        .responsible_app()
        .unwrap_or_else(|| "voicepuncher".to_owned());
    for permission in missing {
        println!("{permission:?} not granted to {app}");
    }
    ExitCode::FAILURE
}
