use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::JoinHandle;

use anyhow::Result;
use rdev::{listen, EventType, Key};

use crate::core::application::ports::hotkey_listener::HotkeyListener;
use crate::core::domain::{HotKey, PttSignal};

#[derive(Default)]
pub struct RdevHotkeyListener;

impl RdevHotkeyListener {
    pub fn new() -> Self {
        Self
    }
}

impl HotkeyListener for RdevHotkeyListener {
    fn spawn(
        self: Box<Self>,
        target: HotKey,
        tx: mpsc::Sender<PttSignal>,
        processing: Arc<AtomicBool>,
    ) -> Result<JoinHandle<()>> {
        let rdev_key = hotkey_to_rdev_key(target);
        let handle = std::thread::Builder::new()
            .name("rdev-hotkey".into())
            .spawn(move || {
                let sender = tx;
                if let Err(e) = listen(move |event| {
                    let signal = match event.event_type {
                        EventType::KeyPress(k) if k == rdev_key => Some(PttSignal::Pressed),
                        EventType::KeyRelease(k) if k == rdev_key => Some(PttSignal::Released),
                        _ => None,
                    };
                    if let Some(s) = signal {
                        if !processing.load(Ordering::Relaxed) {
                            let _ = sender.send(s);
                        }
                    }
                }) {
                    log::error!("hotkey listener stopped unexpectedly: {e:?}");
                }
            })?;
        Ok(handle)
    }
}

fn hotkey_to_rdev_key(hotkey: HotKey) -> Key {
    match hotkey {
        HotKey::RightOption => Key::AltGr,
        HotKey::RightControl => Key::ControlRight,
        HotKey::RightShift => Key::ShiftRight,
        HotKey::RightCommand => Key::MetaRight,
        HotKey::LeftOption => Key::Alt,
        HotKey::LeftControl => Key::ControlLeft,
        HotKey::Fn => Key::Function,
        HotKey::CapsLock => Key::CapsLock,
        HotKey::Space => Key::Space,
        HotKey::F13 => Key::Unknown(105),
        HotKey::F14 => Key::Unknown(107),
        HotKey::F15 => Key::Unknown(113),
        HotKey::F16 => Key::Unknown(106),
        HotKey::F17 => Key::Unknown(64),
        HotKey::F18 => Key::Unknown(79),
        HotKey::F19 => Key::Unknown(80),
    }
}
