/// The Mac's input devices, by the names the configuration's `input_device` takes.
#[cfg_attr(test, mockall::automock)]
pub trait InputDevices: Send + Sync {
    /// The connected input devices' names; a name several devices share is numbered
    /// from its second device on: `USB Mic`, `USB Mic #2`.
    fn names(&self) -> Vec<String>;
}
