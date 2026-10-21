use std::sync::mpsc;
use std::thread::JoinHandle;

use anyhow::Result;

use crate::core::domain::{HotkeyBinding, PttSignal};

#[cfg_attr(test, mockall::automock)]
/// Listens to the keyboard for the life of the process and signals presses and releases
/// of the hotkey `binding` holds at the moment of each key event.
pub trait HotkeyListener: Send + 'static {
    fn spawn(
        self: Box<Self>,
        binding: HotkeyBinding,
        tx: mpsc::Sender<PttSignal>,
    ) -> Result<JoinHandle<()>>;
}
