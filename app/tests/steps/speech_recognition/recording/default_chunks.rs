//! A dictation on the stand-in microphone cut into live chunks as the default settings cut
//! them: which chunks were made while the user spoke and which only after the stop.

use std::sync::mpsc::Receiver;

use cucumber::{then, when};
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::runtime::wiring::chunking_cfg;
use voicepuncher::test_doubles::prelude::*;

use super::{recording, RecordingRun, MIC_RATE};
use crate::IntegrationWorld;

/// How long the chunk watcher gets to cut what was delivered before the user stops.
const SETTLE: Duration = Duration::from_millis(500);
/// The longest chunk the default settings allow, in recorded samples.
const LONGEST_CHUNK: usize = 10 * AudioBuffer16k::SAMPLE_RATE_HZ as usize;
/// Microphone samples of one period of the dictated tone: 200 Hz, so no frame is a pause.
const TONE_PERIOD: usize = MIC_RATE / 200;

/// `seconds` of a steady tone at the microphone rate: speech without a pause.
fn tone(seconds: usize) -> Vec<f32> {
    (0..seconds * MIC_RATE)
        .map(|i| {
            let phase = (i % TONE_PERIOD) as f32 / TONE_PERIOD as f32;
            0.3 * (phase * std::f32::consts::TAU).sin()
        })
        .collect()
}

impl RecordingRun {
    /// Delivers `seconds` of speech with live chunks on the default settings, then stops,
    /// keeping the lengths of the chunks made before and after the stop.
    fn dictate_with_defaults(&mut self, seconds: usize) -> Result<()> {
        let chunks = self.start_default_chunks()?;
        self.deliver(&tone(seconds), Duration::from_millis(1))?;
        std::thread::sleep(SETTLE);
        self.early_chunks = lengths(&chunks);
        self.recorder()?.flush_live_chunks();
        self.stop()?;
        self.late_chunks = lengths(&chunks);
        Ok(())
    }

    fn start_default_chunks(&mut self) -> Result<Receiver<LiveChunk>> {
        let cfg = chunking_cfg(&TranscriptionCfg::default());
        self.recorder()?.start()?;
        self.recorder()?
            .start_live_chunks(cfg)?
            .context("the default settings make no live chunks")
    }
}

/// The lengths of the chunks `chunks` holds so far, in recorded samples.
fn lengths(chunks: &Receiver<LiveChunk>) -> Vec<usize> {
    chunks.try_iter().map(|c| c.audio.samples().len()).collect()
}

#[when(regex = r"^the user dictates for (\d+) seconds with the default settings and stops$")]
fn dictates_with_defaults(world: &mut IntegrationWorld, seconds: usize) -> Result<()> {
    recording(world)?.dictate_with_defaults(seconds)
}

#[then("every chunk but the last was made before the stop")]
fn early_but_the_last(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    if run.early_chunks.len() < 2 || run.late_chunks.len() != 1 {
        bail!(
            "{} chunks before the stop, {} after it",
            run.early_chunks.len(),
            run.late_chunks.len()
        );
    }
    Ok(())
}

#[then("the last chunk is at most 10 seconds long")]
fn short_last_chunk(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    let last = run
        .late_chunks
        .last()
        .copied()
        .context("no chunk after the stop")?;
    if last > LONGEST_CHUNK {
        bail!("the last chunk holds {last} samples, over {LONGEST_CHUNK}");
    }
    Ok(())
}

#[then("the dictation is one chunk, made after the stop")]
fn one_late_chunk(world: &mut IntegrationWorld) -> Result<()> {
    let run = recording(world)?;
    if !run.early_chunks.is_empty() || run.late_chunks.len() != 1 {
        bail!(
            "{} chunks before the stop, {} after it",
            run.early_chunks.len(),
            run.late_chunks.len()
        );
    }
    Ok(())
}
