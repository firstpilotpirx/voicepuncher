//! The user's recorded sessions streamed through the streaming recognizer as the streaming
//! bench streams them: which commands are heard, and how late "start" and "stop" come.

use std::path::Path;

use cucumber::{then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_command_spotting::bench::{recorded_clips, run_clip, Clip, Outcome, Pace};

use super::{run, shared, ENGINE_RUNS};
use crate::steps::file_transcription::env;
use crate::steps::off_thread;
use crate::E2eWorld;

/// The user's "clear" that the Russian recognizer writes «клио»: session-2, 21.2–21.52 s.
const CLEAR_HEARD_AS_KLIO: &str = "session-2/5";

fn recorded() -> Result<Vec<Clip>> {
    recorded_clips(Path::new(&env("VOICEPUNCHER_VOICE_SAMPLES")?))
}

/// Streams each of `clips` into a new session of the shared engine, at `pace`.
fn streamed(clips: &[Clip], pace: Pace) -> Result<Vec<Outcome>> {
    let _one_at_a_time = ENGINE_RUNS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let engine = shared()?;
    clips
        .iter()
        .map(|clip| run_clip(engine, clip, pace))
        .collect()
}

fn listed(outcomes: &[Outcome]) -> Vec<String> {
    outcomes
        .iter()
        .map(|o| {
            format!(
                "{} said {:?} heard {:?} {:?} ms",
                o.id, o.label, o.heard, o.delay_ms
            )
        })
        .collect()
}

#[when("every recording of the user's commands is streamed to the service")]
async fn every_recording(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let said: Vec<Clip> = recorded()?
            .into_iter()
            .filter(|c| !c.label.is_empty())
            .collect();
        run(world).recorded = streamed(&said, Pace::Fast)?;
        Ok(())
    })
    .await
}

#[when("each recorded \"start\" and \"stop\" is streamed to the service in real time")]
async fn recorded_start_and_stop(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let timed: Vec<Clip> = recorded()?
            .into_iter()
            .filter(|c| c.word_end.is_some() && (c.label == ["start"] || c.label == ["stop"]))
            .collect();
        run(world).recorded = streamed(&timed, Pace::RealTime)?;
        Ok(())
    })
    .await
}

#[when("the user's recorded \"clear\" that the Russian recognizer mishears is streamed again")]
async fn misheard_clear(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let clear: Vec<Clip> = recorded()?
            .into_iter()
            .filter(|c| c.id == CLEAR_HEARD_AS_KLIO)
            .collect();
        ensure!(clear.len() == 1, "no recording {CLEAR_HEARD_AS_KLIO}");
        run(world).recorded = streamed(&clear, Pace::Fast)?;
        Ok(())
    })
    .await
}

#[then(expr = "at least {int} of 100 commands are reported as the command said")]
fn heard_of_100(world: &mut E2eWorld, bar: usize) -> Result<()> {
    let outcomes = &run(world).recorded;
    let right = outcomes.iter().filter(|o| o.right()).count();
    let wrong: Vec<Outcome> = outcomes.iter().filter(|o| !o.right()).cloned().collect();
    ensure!(!outcomes.is_empty(), "no recorded command");
    ensure!(
        right * 100 >= bar * outcomes.len(),
        "{right} of {} heard, below {bar} of 100: {:#?}",
        outcomes.len(),
        listed(&wrong)
    );
    Ok(())
}

#[then("at least half are reported within 0.3 s and at least 9 of 10 within 0.6 s of the end of their words")]
fn within_their_bound(world: &mut E2eWorld) -> Result<()> {
    let outcomes = &run(world).recorded;
    let within = |ms: f64| {
        outcomes
            .iter()
            .filter(|o| o.right() && o.delay_ms.is_some_and(|d| d <= ms))
            .count()
    };
    let n = outcomes.len();
    ensure!(n > 0, "no recorded start or stop");
    ensure!(
        2 * within(300.0) >= n && 10 * within(600.0) >= 9 * n,
        "{} of {n} within 0.3 s, {} within 0.6 s: {:#?}",
        within(300.0),
        within(600.0),
        listed(outcomes)
    );
    Ok(())
}

#[then(expr = "the service reports the recorded {string}")]
fn reports_the_recorded(world: &mut E2eWorld, command: String) -> Result<()> {
    let outcomes = &run(world).recorded;
    ensure!(
        outcomes
            .iter()
            .all(|o| o.right() && o.heard == [command.clone()]),
        "expected {command:?}: {:#?}",
        listed(outcomes)
    );
    Ok(())
}
