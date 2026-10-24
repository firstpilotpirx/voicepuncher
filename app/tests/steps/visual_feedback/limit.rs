//! The recording buffer the microphone recorder fills: whether it counts as cut off at the
//! recording limit decides the recorder's recording-limit warning.

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::recording::cpal::buffer::RecordingBuffer;

use crate::IntegrationWorld;

/// A recording limit of one second at 16 kHz.
const LIMIT: usize = 16_000;
/// A few seconds of a recording well under the limit, in samples.
const SHORT: usize = LIMIT / 4;

/// The buffer of a scenario's recording.
pub struct LimitRun(RecordingBuffer);

impl std::fmt::Debug for LimitRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LimitRun({} samples)", self.0.len())
    }
}

fn buffer(world: &mut IntegrationWorld) -> Result<&RecordingBuffer> {
    let run = world
        .extra
        .view
        .limit
        .as_ref()
        .context("no recording runs")?;
    Ok(&run.0)
}

#[given("a live recording shorter than the recording limit")]
fn short_recording(world: &mut IntegrationWorld) {
    let recording = RecordingBuffer::new(LIMIT);
    recording.append(&vec![0; SHORT]);
    world.extra.view.limit = Some(LimitRun(recording));
}

#[given("a recording whose audio fills the recording limit")]
fn full_recording(world: &mut IntegrationWorld) {
    let recording = RecordingBuffer::new(LIMIT);
    recording.append(&vec![0; LIMIT * 2]);
    world.extra.view.limit = Some(LimitRun(recording));
}

#[when("its last chunk cuts the recording at its end and the recording stops")]
fn cut_at_end(world: &mut IntegrationWorld) -> Result<()> {
    let recording = buffer(world)?;
    recording.cut_at(recording.len());
    Ok(())
}

#[when("the recording stops")]
fn stops(_world: &mut IntegrationWorld) {}

#[then("the recording does not count as cut off at its limit")]
fn not_at_limit(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        !buffer(world)?.is_full(),
        "a short recording counts as cut off at its limit"
    );
    Ok(())
}

#[then("the recording counts as cut off at its limit")]
fn at_limit(world: &mut IntegrationWorld) -> Result<()> {
    let recording = buffer(world)?;
    ensure!(
        recording.is_full(),
        "{} samples do not reach the limit",
        recording.len()
    );
    Ok(())
}
