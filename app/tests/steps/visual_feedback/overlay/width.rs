//! The width of the overlay's pill.

use anyhow::{ensure, Context, Result};
use cucumber::then;

use super::overlay;
use crate::IntegrationWorld;

/// The width and height of the pill on screen.
pub(super) fn pill_size(world: &mut IntegrationWorld) -> Result<(f64, f64)> {
    let frame = overlay(world)?
        .first_pill_glass_frame()
        .context("no pill shows")?;
    Ok((frame.size.width, frame.size.height))
}

#[then(expr = "the pill is {float} points wide")]
fn pill_width(world: &mut IntegrationWorld, points: f64) -> Result<()> {
    let (width, _) = pill_size(world)?;
    ensure!(
        (width - points).abs() < 0.5,
        "the pill is {width} points wide"
    );
    Ok(())
}
