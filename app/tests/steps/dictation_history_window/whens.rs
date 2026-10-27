//! When steps: the user opens the window, copies a row, a dictation is archived.

use anyhow::{bail, Result};
use cucumber::when;

use super::archive::Said;
use super::givens::TEXTS;
use super::run;
use crate::IntegrationWorld;

/// The start of the dictation archived while the window is open.
pub(super) const NEW_SECOND: u64 = 10;

#[when("the user opens the history window")]
fn opens(world: &mut IntegrationWorld) -> Result<()> {
    run(world)?.open();
    Ok(())
}

#[when("the user copies the second row")]
fn copies_second(world: &mut IntegrationWorld) -> Result<()> {
    run(world)?.view()?.copy(1)
}

#[when("a new dictation is archived and the window checks the archive")]
fn new_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    run.keep(NEW_SECOND, &Said::pasted(TEXTS[2]));
    if !run.view()?.check() {
        bail!("the window saw no new entry");
    }
    Ok(())
}
