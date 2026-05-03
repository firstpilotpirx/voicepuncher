// Cross-adapter integration test that drives the **real** macOS event chain to
// reproduce (or prove fixed) the right-Option drop-out after a paste.
//
// Bug under test:
//   In toggle mode the user presses right-Option to start recording, presses
//   right-Option again to stop and trigger transcribe→refine→paste, and then
//   presses right-Option once more to start the next recording. The third
//   press is silently dropped — the rdev listener does not deliver
//   `PttSignal::Pressed` after the paste posted a synthetic Cmd+V at the HID
//   tap location.
//
// What this test does:
//   1. Spawns the production `RdevHotkeyListener` configured for
//      `HotKey::RightOption`. Signals land on a real `mpsc::Sender<PttSignal>`,
//      same as in `main.rs`.
//   2. Synthesizes three right-Option down/up pairs at HID using
//      `rdev::simulate(EventType::KeyPress(Key::AltGr))` — exactly the
//      keyDown/keyUp shape the system emits for that key.
//   3. Between press 2 and press 3 it invokes the production
//      `MacOsTextPaster::paste`, which posts the synthetic Cmd+V at HID — the
//      same act suspected of corrupting rdev's modifier-flag tracking.
//   4. Asserts that EVERY simulated press surfaces as `Pressed` followed by
//      `Released` on the channel — including press 3, which is the press the
//      bug drops.
//
// Why this exercises the bug:
//   - The rdev listener and the paster both touch the HID event tap. If the
//     paster's synthetic Cmd+V leaves rdev's flag tracker in a state that
//     filters out the next AltGr press, this test fails on press 3.
//   - The test runs the real production code paths (the same listener struct,
//     the same paster struct, the same channel type) — no mocks, no shims.
//
// Gate: VOICEPUNCHER_REAL_KEYBOARD_IT=1
//   The test is skipped unless this env var is set, because it requires:
//     * macOS Accessibility + Input Monitoring permissions for the test binary
//     * a real GUI session (will not work over SSH / in CI without a display)
//     * exclusive use of the keyboard for the duration of the test
//
// Run it manually with:
//   VOICEPUNCHER_REAL_KEYBOARD_IT=1 \
//     cargo test --test ptt_paste_interaction_it -- --nocapture

use std::sync::mpsc;
use std::time::Duration;

use rdev::{simulate, EventType, Key};

use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::adapters::secondary::textpasting::macos::MacOsTextPaster;
use voicepuncher::core::application::HotkeyListener;
use voicepuncher::core::application::TextPaster;
use voicepuncher::core::domain::{CleanedText, HotKey, PttSignal};

const TAP_INSTALL_GRACE: Duration = Duration::from_millis(800);
const SIM_DOWN_UP_GAP: Duration = Duration::from_millis(40);
const SIGNAL_TIMEOUT: Duration = Duration::from_secs(2);
const POST_PASTE_SETTLE: Duration = Duration::from_millis(200);

#[test]
fn should_keep_delivering_right_option_signals_after_paste() {
    if std::env::var("VOICEPUNCHER_REAL_KEYBOARD_IT").is_err() {
        eprintln!("GATED: VOICEPUNCHER_REAL_KEYBOARD_IT not set, skipping");
        return;
    }

    // Given — production rdev listener wired to a real channel
    let (tx, rx) = mpsc::channel::<PttSignal>();
    let listener: Box<dyn HotkeyListener> = Box::new(RdevHotkeyListener::new());
    let _handle = listener
        .spawn(HotKey::RightOption, tx)
        .expect("rdev listener must spawn (Accessibility / Input Monitoring granted?)");

    // wait for the rdev tap to install before synthesizing events
    std::thread::sleep(TAP_INSTALL_GRACE);
    drain(&rx);

    // When — press 1: enter Recording in production toggle mode
    eprintln!("[IT] press 1 (would START recording in toggle mode)");
    sim_press_release(Key::AltGr);
    expect_press_release(&rx, "press 1");

    // When — press 2: leave Recording, would trigger transcribe→refine→paste
    eprintln!("[IT] press 2 (would STOP recording in toggle mode)");
    sim_press_release(Key::AltGr);
    expect_press_release(&rx, "press 2");

    // When — invoke the real macOS paster (posts synthetic Cmd+V at HID).
    // This is the suspected source of rdev flag-tracking corruption.
    eprintln!("[IT] paste (posts synthetic Cmd+V at HID)");
    let mut paster = MacOsTextPaster::new(50);
    paster
        .paste(&CleanedText::new("voicepuncher-it-paste".to_owned()).unwrap())
        .expect("paste must not error");
    std::thread::sleep(POST_PASTE_SETTLE);

    // The paste itself should not push any AltGr signals onto our channel
    // (it only synthesizes Cmd+V), but drain defensively.
    drain(&rx);

    // When — press 3: this is the press the bug silently drops.
    eprintln!("[IT] press 3 (the press the bug drops)");
    sim_press_release(Key::AltGr);

    // Then — press 3 must arrive intact, otherwise the bug is reproduced.
    expect_press_release(&rx, "press 3 (post-paste)");
}

fn sim_press_release(key: Key) {
    simulate(&EventType::KeyPress(key)).expect("simulate KeyPress");
    std::thread::sleep(SIM_DOWN_UP_GAP);
    simulate(&EventType::KeyRelease(key)).expect("simulate KeyRelease");
}

fn expect_press_release(rx: &mpsc::Receiver<PttSignal>, label: &str) {
    let down = rx.recv_timeout(SIGNAL_TIMEOUT).ok();
    assert_eq!(
        down,
        Some(PttSignal::Pressed),
        "{label}: expected Pressed, got {down:?}",
    );
    let up = rx.recv_timeout(SIGNAL_TIMEOUT).ok();
    assert_eq!(
        up,
        Some(PttSignal::Released),
        "{label}: expected Released, got {up:?}",
    );
}

fn drain(rx: &mpsc::Receiver<PttSignal>) {
    while rx.try_recv().is_ok() {}
}
