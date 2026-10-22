//! The app's login item on macOS, through `SMAppService` (macOS 13 and later): System
//! Settings → General → Login Items lists it and the user can switch it off there.

pub mod launch;

use std::path::{Path, PathBuf};

use objc2_service_management::{SMAppService, SMAppServiceStatus};

use crate::core::application::login_item::{LoginItem, LoginItemStatus};

const UNAVAILABLE: &str = "voicepuncher does not run as an installed app";

/// The login item of the bundle this process runs from.
pub struct MacOsLoginItem {
    installed: bool,
}

impl MacOsLoginItem {
    /// The login item of the app at `executable`; outside an installed bundle the system is
    /// never asked.
    pub fn new(executable: &Path) -> Self {
        Self { installed: is_installed_bundle(executable) }
    }

    /// The login item of the running executable.
    pub fn of_this_process() -> Self {
        Self::new(&std::env::current_exe().unwrap_or_else(|_| PathBuf::new()))
    }
}

impl LoginItem for MacOsLoginItem {
    fn status(&self) -> LoginItemStatus {
        if !self.installed {
            return LoginItemStatus::Unavailable;
        }
        // SAFETY: plain getters of the main app service; no argument to keep alive.
        status_of(unsafe { SMAppService::mainAppService().status() })
    }

    fn register(&self) -> Result<(), String> {
        if !self.installed {
            return Err(UNAVAILABLE.to_owned());
        }
        // SAFETY: the main app service registers the bundle this process runs from.
        unsafe { SMAppService::mainAppService().registerAndReturnError() }
            .map_err(|error| error.localizedDescription().to_string())
    }

    fn unregister(&self) -> Result<(), String> {
        if !self.installed {
            return Err(UNAVAILABLE.to_owned());
        }
        // SAFETY: as in `register`.
        unsafe { SMAppService::mainAppService().unregisterAndReturnError() }
            .map_err(|error| error.localizedDescription().to_string())
    }
}

/// Whether `executable` is the main executable of an application bundle:
/// `<anything>.app/Contents/MacOS/<name>`; macOS decides whether it accepts the bundle.
pub fn is_installed_bundle(executable: &Path) -> bool {
    let Some(macos) = executable.parent() else {
        return false;
    };
    let Some(contents) = macos.parent() else {
        return false;
    };
    let bundle = contents.parent();
    macos.file_name().is_some_and(|name| name == "MacOS")
        && contents.file_name().is_some_and(|name| name == "Contents")
        && bundle
            .and_then(Path::extension)
            .is_some_and(|extension| extension == "app")
}

/// The system's status of the login item in the app's words; a status the system does
/// not know the app by (`NotFound`, or a newer one) is unavailable.
pub fn status_of(status: SMAppServiceStatus) -> LoginItemStatus {
    match status {
        SMAppServiceStatus::NotRegistered => LoginItemStatus::NotRegistered,
        SMAppServiceStatus::Enabled => LoginItemStatus::Enabled,
        SMAppServiceStatus::RequiresApproval => LoginItemStatus::RequiresApproval,
        _ => LoginItemStatus::Unavailable,
    }
}

#[cfg(test)]
mod tests;
