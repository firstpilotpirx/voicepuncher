//! The hotkey listener on a listen-only `CGEventTap` of its own: it reads key codes and
//! modifier flags only, never what a key types, so no input-source call runs off the main
//! thread.

pub mod keys;

use std::cell::RefCell;
use std::sync::mpsc;
use std::thread::JoinHandle;

use anyhow::{anyhow, Result};
use core_foundation::runloop::CFRunLoop;
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};

use crate::core::application::hotkey_listener::HotkeyListener;
use crate::core::domain::{HotkeyBinding, PttSignal};
use keys::{classify, key_event, ChordState, TapKind};

pub struct EventTapHotkeyListener;

impl EventTapHotkeyListener {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for EventTapHotkeyListener {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyListener for EventTapHotkeyListener {
    fn spawn(
        self: Box<Self>,
        binding: HotkeyBinding,
        tx: mpsc::Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        let handle = std::thread::Builder::new()
            .name("hotkey-tap".into())
            .spawn(move || {
                if let Err(e) = listen(&binding, &tx) {
                    log::error!("hotkey listener stopped unexpectedly: {e:#}");
                }
            })?;
        Ok(handle)
    }
}

/// Runs the tap on this thread's run loop until the process ends; opens it again whenever
/// macOS turns it off.
fn listen(binding: &HotkeyBinding, tx: &mpsc::Sender<PttSignal>) -> Result<()> {
    let chord = RefCell::new(ChordState::default());
    loop {
        CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::KeyDown,
                CGEventType::KeyUp,
                CGEventType::FlagsChanged,
            ],
            |_proxy, kind, event| {
                if let Some(signal) = tap_kind(kind)
                    .and_then(|kind| signal_of(kind, event, binding, &mut chord.borrow_mut()))
                {
                    log::debug!("hotkey tap forward: {signal:?}");
                    let _ = tx.send(signal);
                }
                CallbackResult::Keep
            },
            CFRunLoop::run_current,
        )
        .map_err(|()| anyhow!("cannot open the hotkey tap (Input Monitoring not granted?)"))?;
        log::warn!("hotkey tap turned off by macOS; opening it again");
    }
}

/// The key kind of a tap event; a notice that macOS turned the tap off stops the run loop,
/// so the tap is opened again.
fn tap_kind(kind: CGEventType) -> Option<TapKind> {
    match kind {
        CGEventType::KeyDown => Some(TapKind::KeyDown),
        CGEventType::KeyUp => Some(TapKind::KeyUp),
        CGEventType::FlagsChanged => Some(TapKind::FlagsChanged),
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            CFRunLoop::get_current().stop();
            None
        }
        _ => None,
    }
}

fn signal_of(
    kind: TapKind,
    event: &CGEvent,
    binding: &HotkeyBinding,
    chord: &mut ChordState,
) -> Option<PttSignal> {
    let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
    let key = key_event(kind, u16::try_from(code).ok()?, event.get_flags().bits())?;
    classify(key, binding.current(), chord)
}
