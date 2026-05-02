use std::sync::mpsc;
use std::thread::JoinHandle;

use anyhow::Result;

use crate::core::domain::{HotKey, PttSignal};

#[cfg_attr(test, mockall::automock)]
pub trait HotkeyListener: Send + 'static {
    fn spawn(
        self: Box<Self>,
        target: HotKey,
        tx: mpsc::Sender<PttSignal>,
    ) -> Result<JoinHandle<()>>;
}
