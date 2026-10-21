/// Whether the app starts when the user logs in, as the system keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginItemStatus {
    NotRegistered,
    Enabled,
    /// Registered, but the user has yet to approve it in System Settings.
    RequiresApproval,
    /// The app is not an installed bundle the system knows, so it cannot be a login item.
    Unavailable,
}

/// The app's login item: what the system says about it, and registering or removing it.
#[cfg_attr(test, mockall::automock)]
pub trait LoginItem: Send + Sync {
    fn status(&self) -> LoginItemStatus;
    /// Registers the app as a login item; the error is the system's reason.
    fn register(&self) -> Result<(), String>;
    /// Removes the app from the login items; the error is the system's reason.
    fn unregister(&self) -> Result<(), String>;
}
