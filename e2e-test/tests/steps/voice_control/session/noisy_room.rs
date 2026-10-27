//! A voice start in a room whose silence is its own steady noise.

use voicepuncher::test_doubles::prelude::*;

use cucumber::when;
use voicepuncher_e2e_test::conversions::saturating_usize;

use super::start_cue::synthesized_command;
use super::{session_of, SAMPLE_RATE_HZ};
use crate::E2eWorld;

/// The room noise's level: a fan or an air conditioner a few metres away.
const NOISE_RMS: f32 = 0.01;
/// The noise heard before the command, so the service knows the room.
const NOISE_BEFORE_SECS: f32 = 1.0;

/// `samples` of white noise at `NOISE_RMS`, the same on every run.
fn steady_noise(samples: usize, seed: &mut u32) -> Vec<f32> {
    // A uniform sample in [-a, a] has an RMS of a / sqrt(3).
    let amplitude = NOISE_RMS * 3.0_f32.sqrt();
    (0..samples)
        .map(|_| {
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let unit = (*seed >> 8) as f32 / (1u32 << 24) as f32;
            amplitude * (unit * 2.0 - 1.0)
        })
        .collect()
}

#[when(expr = "someone says {string} in steady room noise and stays silent")]
fn says_command_in_noise(world: &mut E2eWorld, said: String) -> Result<()> {
    let session = session_of(world)?;
    let mut seed = 7;
    let lead = saturating_usize(NOISE_BEFORE_SECS * SAMPLE_RATE_HZ);
    let mut audio = steady_noise(lead, &mut seed);
    let command = synthesized_command(&said)?;
    let noise = steady_noise(command.len(), &mut seed);
    audio.extend(command.iter().zip(noise).map(|(voice, room)| voice + room));
    let words_end = audio.len();
    let room = steady_noise(saturating_usize(0.1 * SAMPLE_RATE_HZ), &mut seed);
    session.time_start(
        format!("\"{said}\" in room noise"),
        &audio,
        words_end,
        &room,
    )?;
    Ok(())
}
