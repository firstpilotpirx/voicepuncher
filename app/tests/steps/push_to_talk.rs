//! Drives the real macOS event chain: the production rdev listener and the production
//! paster share the HID event tap. The synthetic Cmd+V of a paste used to corrupt rdev's
//! modifier tracking so that the next right-Option press was dropped.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};
use rdev::{simulate, EventType, Key};

use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::core::application::{HotkeyListener, TextPaster};
use voicepuncher::core::domain::{CleanedText, HotKey, HotkeyBinding, PttSignal};

use crate::support::pasteboard_lock::PasteboardLock;
use crate::IntegrationWorld;

const TAP_INSTALL_GRACE: Duration = Duration::from_millis(800);
const SIM_DOWN_UP_GAP: Duration = Duration::from_millis(40);
const SIGNAL_TIMEOUT: Duration = Duration::from_secs(2);
const POST_PASTE_SETTLE: Duration = Duration::from_millis(200);
const PASTE_DELAY_MS: u64 = 50;

#[given("the user pressed and released the hotkey twice")]
fn hotkey_pressed_twice(world: &mut IntegrationWorld) -> Result<()> {
    let (tx, rx) = mpsc::channel::<PttSignal>();
    let listener: Box<dyn HotkeyListener> = Box::new(RdevHotkeyListener::new());
    listener
        .spawn(HotkeyBinding::new(HotKey::RightOption), tx)
        .context("rdev listener must spawn (Accessibility / Input Monitoring granted?)")?;
    std::thread::sleep(TAP_INSTALL_GRACE);
    for _ in 0..2 {
        press_and_release_right_option()?;
    }
    std::thread::sleep(SIGNAL_TIMEOUT);
    drain(&rx);
    world.ptt_signals = Some(rx);
    Ok(())
}

#[given("the app pasted text into the focused app")]
fn app_pasted_text(world: &mut IntegrationWorld) -> Result<()> {
    // Held until the paste's restore (PASTE_DELAY_MS) is done within POST_PASTE_SETTLE
    let _pasteboard_lock = PasteboardLock::take();
    let mut paster = MacOsTextPaster::new(PASTE_DELAY_MS);
    paster.paste(&CleanedText::new("voicepuncher-it-paste".to_owned())?)?;
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
        if got != Some(expected) {
            bail!("expected {expected:?}, got {got:?}");
        }
    }
    Ok(())
}

fn signals(world: &IntegrationWorld) -> Result<&Receiver<PttSignal>> {
    world
        .ptt_signals
        .as_ref()
        .context("hotkey listener is not running")
}

fn press_and_release_right_option() -> Result<()> {
    simulate(&EventType::KeyPress(Key::AltGr)).context("simulate KeyPress")?;
    std::thread::sleep(SIM_DOWN_UP_GAP);
    simulate(&EventType::KeyRelease(Key::AltGr)).context("simulate KeyRelease")?;
    Ok(())
}

fn drain(rx: &Receiver<PttSignal>) {
    while rx.try_recv().is_ok() {}
}
