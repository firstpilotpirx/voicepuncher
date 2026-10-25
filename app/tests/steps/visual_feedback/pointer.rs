//! Shows the real dictation sign on the main thread and reads where it stands against the real
//! pointer and whether it takes input — the calls the main-thread tests of `visualfeedback_macos`
//! make.

use core_graphics::display::CGDisplay;
use core_graphics::geometry::CGPoint;
use cucumber::{then, when};
use objc2::MainThreadMarker;
use objc2_app_kit::NSEvent;
use objc2_foundation::NSPoint;
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::secondary::visualfeedback::macos::MacosPointerSign;
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    DictationSignState, PointerSignView,
};

use super::overlay::pump;
use crate::IntegrationWorld;

/// How many times the sign is read again while the real pointer moves (someone uses the mouse).
const STILL_POINTER_TRIES: usize = 50;
/// Long enough for a follow tick to move the sign.
const FOLLOWED_SECS: f64 = 0.05;

/// The dictation sign of a scenario, hidden when the scenario ends.
pub struct PointerRun(MacosPointerSign);

impl std::fmt::Debug for PointerRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PointerRun")
    }
}

impl Drop for PointerRun {
    fn drop(&mut self) {
        self.0.hide();
        pump(FOLLOWED_SECS);
    }
}

/// The pointer and the sign's top-left corner, read while the pointer stood still.
fn beside_a_still_pointer(sign: &MacosPointerSign) -> (NSPoint, NSPoint) {
    let read = || {
        let before = NSEvent::mouseLocation();
        pump(FOLLOWED_SECS);
        (before, NSEvent::mouseLocation(), sign.top_left())
    };
    let mut seen = read();
    for _ in 1..STILL_POINTER_TRIES {
        if (seen.0.x, seen.0.y) == (seen.1.x, seen.1.y) {
            break;
        }
        seen = read();
    }
    (seen.1, seen.2)
}

fn sign(world: &mut IntegrationWorld) -> Result<&MacosPointerSign> {
    let run = world
        .extra
        .view
        .pointer
        .as_ref()
        .context("the dictation sign never showed")?;
    Ok(&run.0)
}

#[when("the dictation sign shows for a recording")]
fn shows_for_recording(world: &mut IntegrationWorld) -> Result<()> {
    let mtm = MainThreadMarker::new().context("the harness runs its steps on the main thread")?;
    let sign = MacosPointerSign::initialize(mtm);
    sign.show(DictationSignState::Recording);
    pump(FOLLOWED_SECS * 2.0);
    world.extra.view.pointer = Some(PointerRun(sign));
    Ok(())
}

#[then("it stands right of and below the pointer")]
fn beside_pointer(world: &mut IntegrationWorld) -> Result<()> {
    let sign = sign(world)?;
    ensure!(sign.is_visible(), "the dictation sign is not shown");
    let (pointer, top_left) = beside_a_still_pointer(sign);
    ensure!(
        top_left.x > pointer.x && top_left.y < pointer.y,
        "the sign {top_left:?} is not right of and below the pointer {pointer:?}"
    );
    Ok(())
}

/// How far the pointer is moved to see the sign follow it, in points.
const STEP: f64 = 40.0;

/// Moves the real pointer by `dx` right and `dy` down (global top-left coordinates).
fn move_pointer(dx: f64, dy: f64) -> Result<()> {
    let at = NSEvent::mouseLocation();
    let top = CGDisplay::main().bounds().size.height - at.y;
    CGDisplay::warp_mouse_cursor_position(CGPoint::new(at.x + dx, top + dy))
        .map_err(|e| anyhow::anyhow!("the pointer did not move: {e}"))
}

#[then("it moves with the pointer and keeps its place beside it")]
fn moves_with_pointer(world: &mut IntegrationWorld) -> Result<()> {
    let sign = sign(world)?;
    let (pointer, top_left) = beside_a_still_pointer(sign);
    move_pointer(STEP, STEP)?;
    let (moved, followed) = beside_a_still_pointer(sign);
    move_pointer(-STEP, -STEP)?;
    ensure!(
        (moved.x - pointer.x).abs() > 1.0,
        "the pointer did not move"
    );
    let offset = (top_left.x - pointer.x, top_left.y - pointer.y);
    let kept = (followed.x - moved.x, followed.y - moved.y);
    ensure!(
        (offset.0 - kept.0).abs() < 1.0 && (offset.1 - kept.1).abs() < 1.0,
        "the sign kept {kept:?} from the pointer, not {offset:?}"
    );
    Ok(())
}

#[then("it lets clicks through and takes no keyboard focus")]
fn takes_no_input(world: &mut IntegrationWorld) -> Result<()> {
    let sign = sign(world)?;
    ensure!(
        sign.ignores_mouse_events(),
        "the dictation sign takes clicks"
    );
    ensure!(
        !sign.can_become_key(),
        "the dictation sign can take the keyboard focus"
    );
    Ok(())
}
