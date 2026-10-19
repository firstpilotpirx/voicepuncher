//! Watches the Pasted sign on the stand-ins of the text-insertion scenarios (a dictation
//! pasted at once, after a wait, or left in the clipboard) and of the voice-control scenarios
//! (a command said right after the paste).

use anyhow::{bail, Result};
use cucumber::{given, then};

use crate::steps::{text_insertion, voice_control};
use crate::IntegrationWorld;

/// The overlay event of the sign "press enter" shows.
const ENTER_SIGN: &str = "enter sign";

#[given("voice control is turned on and the overlay shows the Pasted sign")]
fn pasted_sign_shows(world: &mut IntegrationWorld) -> Result<()> {
    voice_control::listening_on_given(world)?;
    voice_control::records_with_hotkey(world)?;
    voice_control::ends_on_sign(voice_control::run(world), voice_control::PASTED_SIGN)?;
    Ok(())
}

#[then("the overlay shows the Pasted sign with no text window, then hides")]
fn pasted_sign_then_hidden(world: &mut IntegrationWorld) -> Result<()> {
    text_insertion::ends_on_pasted_sign(world)
}

#[then("the overlay hides without showing the Pasted sign")]
fn hidden_without_pasted_sign(world: &mut IntegrationWorld) -> Result<()> {
    text_insertion::hidden_without_pasted_sign(world)
}

#[then("the enter sign replaces the Pasted sign at once")]
fn enter_sign_replaces_pasted_sign(world: &mut IntegrationWorld) -> Result<()> {
    let run = voice_control::run(world);
    let shown = voice_control::ends_on_sign(run, ENTER_SIGN)?;
    let before = voice_control::overlay_before_mark(run);
    if before.last() != Some(&voice_control::PASTED_SIGN) {
        bail!("the overlay did not show the Pasted sign before the command: {before:?}");
    }
    if shown.first() != Some(&ENTER_SIGN) {
        bail!("expected the enter sign right after the Pasted sign: {shown:?}");
    }
    Ok(())
}
