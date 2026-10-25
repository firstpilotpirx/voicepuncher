//! Drives the real macOS event chain: the production hotkey tap and the production paster
//! share the HID event tap. The synthetic Cmd+V of a paste once corrupted the listener's
//! modifier tracking so that the next right-Option press was dropped.

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use cucumber::{given, then, when};
use objc2_app_kit::NSPasteboard;
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::primary::ptt::eventtap::{keys::RIGHT_OPTION, EventTapHotkeyListener};
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::core::domain::{HotKey, HotkeyBinding};

use crate::IntegrationWorld;

const TAP_INSTALL_GRACE: Duration = Duration::from_millis(800);
const SIM_DOWN_UP_GAP: Duration = Duration::from_millis(40);
const SIGNAL_TIMEOUT: Duration = Duration::from_secs(2);
const POST_PASTE_SETTLE: Duration = Duration::from_millis(200);
const PASTE_DELAY_MS: u64 = 50;

#[given("the user pressed and released the hotkey twice")]
fn hotkey_pressed_twice(world: &mut IntegrationWorld) -> Result<()> {
    let (tx, rx) = mpsc::channel::<PttSignal>();
    Box::new(EventTapHotkeyListener::new())
        .spawn(HotkeyBinding::new(HotKey::RightOption), tx)
        .context("hotkey listener must spawn (Accessibility / Input Monitoring granted?)")?;
    std::thread::sleep(TAP_INSTALL_GRACE);
    (0..2).try_for_each(|_| press_and_release_right_option())?;
    std::thread::sleep(SIGNAL_TIMEOUT);
    drain(&rx);
    world.pipeline.ptt_signals = Some(rx);
    Ok(())
}

#[given("the app pasted text into the focused app")]
fn app_pasted_text(world: &mut IntegrationWorld) -> Result<()> {
    // A pasteboard of its own, so the user's clipboard is never written
    let own = NSPasteboard::pasteboardWithUniqueName().name().to_string();
    let mut paster = MacOsTextPaster::new(PASTE_DELAY_MS).with_pasteboard(&own);
    paster.paste(&CleanedText::new("voicepuncher-it-paste")?)?;
    std::thread::sleep(POST_PASTE_SETTLE);
    drain(signals(world)?);
    Ok(())
}

#[when("the user presses and releases the hotkey again")]
fn hotkey_pressed_again(_world: &mut IntegrationWorld) -> Result<()> {
    press_and_release_right_option()
}

#[then("both the press and the release are registered")]
fn press_and_release_registered(world: &mut IntegrationWorld) -> Result<()> {
    let rx = signals(world)?;
    for expected in [PttSignal::Pressed, PttSignal::Released] {
        let got = rx.recv_timeout(SIGNAL_TIMEOUT).ok();
        ensure!(got == Some(expected), "expected {expected:?}, got {got:?}");
    }
    Ok(())
}

fn signals(world: &IntegrationWorld) -> Result<&Receiver<PttSignal>> {
    world
        .pipeline
        .ptt_signals
        .as_ref()
        .context("no hotkey listener")
}

/// Posts right Option down (Alternate with its device bit) and up (no flag).
fn press_and_release_right_option() -> Result<()> {
    for flags in [CGEventFlags::CGEventFlagAlternate.bits() | 0x40, 0] {
        let event = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .and_then(|source| CGEvent::new_keyboard_event(source, RIGHT_OPTION, flags != 0))
            .map_err(|()| anyhow::anyhow!("cannot create the right Option event"))?;
        event.set_type(CGEventType::FlagsChanged);
        event.set_flags(CGEventFlags::from_bits_retain(flags));
        event.post(CGEventTapLocation::HID);
        std::thread::sleep(SIM_DOWN_UP_GAP);
    }
    Ok(())
}

fn drain(rx: &Receiver<PttSignal>) {
    while rx.try_recv().is_ok() {}
}
