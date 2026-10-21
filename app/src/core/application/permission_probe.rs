use crate::core::domain::{Permission, PermissionStatus};

/// Asks the system about the permissions voicepuncher needs, without opening a device.
#[cfg_attr(test, mockall::automock)]
pub trait PermissionProbe: Send + Sync {
    fn status(&self, permission: Permission) -> PermissionStatus;
    /// Shows the system's request for `permission`; returns at once, the answer shows in
    /// a later `status`.
    fn request(&self, permission: Permission);
    /// The System Settings page where the user grants `permission`.
    fn settings_url(&self, permission: Permission) -> String;
    /// The application macOS attributes the permissions to when it is not voicepuncher
    /// itself: the parent of a bare binary run from a terminal; none in the bundle.
    fn responsible_app(&self) -> Option<String>;
}
