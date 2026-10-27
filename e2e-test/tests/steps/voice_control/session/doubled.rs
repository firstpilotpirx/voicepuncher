//! A doubled command said in running speech takes effect at once, before the speech ends.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{then, when};

use super::session_of;
use super::start_cue::{sentence_at_once, synthesized_command, within_while_speaking};
use crate::E2eWorld;

#[when(expr = "someone says a sentence, {string} and goes on speaking")]
fn says_command_inside_speech(world: &mut E2eWorld, said: String) -> Result<()> {
    let session = session_of(world)?;
    let mut audio = sentence_at_once()?;
    audio.extend(synthesized_command(&said)?);
    let words_end = audio.len();
    audio.extend(sentence_at_once()?);
    let label = format!("\"{said}\" inside speech");
    session.time_cue(label, (&audio, words_end), &silence_room(), &session.stops)?;
    Ok(())
}

#[then(
    expr = "the stop sound plays within {float} s of the end of {string}, while the speech goes on"
)]
fn stop_while_speaking(world: &mut E2eWorld, bound_secs: f32, said: String) -> Result<()> {
    within_while_speaking(&session_of(world)?.last_start_of(&said)?, bound_secs)
}

fn silence_room() -> Vec<f32> {
    voicepuncher_e2e_test::voice_commands::silence(0.1)
}
