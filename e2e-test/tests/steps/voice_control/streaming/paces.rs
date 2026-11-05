//! The two paces of the streaming bench agree: a clip streamed as fast as it decodes, its
//! session replaying the service's clock, reports each command as late as in real time.

use cucumber::{then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_command_spotting::bench::{run_clip, Clip, Outcome, Pace};
use voicepuncher_e2e_test::voice_commands::{
    silence, spoken_by, spoken_with_pauses, without_trailing_silence,
};

use super::super::ENGLISH_VOICE;
use super::recorded::recorded;
use super::{every_phrase, run, shared, ENGINE_RUNS, SAMPLE_RATE};
use crate::steps::off_thread;
use crate::E2eWorld;

/// The dictation before the lone word, as the start cue scenario dictates it.
const DICTATED_SECS: usize = 20;
const DICTATED: [&str; 3] = [
    "Weather is sunny in Berlin today",
    "Please bring the slides to the review on Monday",
    "The train leaves at seven and arrives before noon",
];
/// The recorded clips the sample holds: a few sessions, not the 7 min of all of them.
const SAMPLE: usize = 3;

/// 20 s of dictated sentences, a pause and a lone "restart" cut at its last sound, then 2 s
/// of silence; its command and where its words end.
fn lone_restart() -> Result<Clip> {
    let mut audio = Vec::new();
    for sentence in DICTATED.iter().cycle() {
        if audio.len() >= DICTATED_SECS * SAMPLE_RATE {
            break;
        }
        audio.extend(spoken_by(ENGLISH_VOICE, sentence)?);
    }
    audio.extend(silence(1.0));
    audio.extend(without_trailing_silence(spoken_with_pauses("restart")?));
    let word_end = Some(audio.len() as f32 / SAMPLE_RATE as f32);
    audio.extend(silence(2.0));
    Ok(Clip {
        id: "lone-restart".to_owned(),
        audio,
        label: vec!["restart".to_owned()],
        listening: every_phrase(),
        word_end,
        words: String::new(),
    })
}

/// Each of `clips` streamed in real time and then as fast as it decodes, on the shared engine.
fn in_both_paces(clips: &[Clip]) -> Result<Vec<(Outcome, Outcome)>> {
    let _one_at_a_time = ENGINE_RUNS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let engine = shared()?;
    clips
        .iter()
        .map(|clip| {
            let real = run_clip(engine, clip, Pace::RealTime)?;
            let fast = run_clip(engine, clip, Pace::Fast)?;
            // The numbers behind a pass, for the design's measurements
            eprintln!("paces: {}", listed(&real, &fast));
            Ok((real, fast))
        })
        .collect()
}

/// Whether the fast pace heard every command the real-time pace heard, as late within
/// `margin_ms` when the real-time pace timed it.
fn agree(real: &Outcome, fast: &Outcome, margin_ms: f64) -> bool {
    let heard = real.heard.iter().all(|c| fast.heard.contains(c));
    let as_late = match (real.delay_ms, fast.delay_ms) {
        (Some(real), Some(fast)) => (real - fast).abs() <= margin_ms,
        (Some(_), None) => false,
        (None, _) => true,
    };
    heard && as_late
}

fn listed(real: &Outcome, fast: &Outcome) -> String {
    format!(
        "{}: real time heard {:?} {:?} ms, fast heard {:?} {:?} ms",
        real.id, real.heard, real.delay_ms, fast.heard, fast.delay_ms
    )
}

#[when(
    "a lone \"restart\" after 20 s of dictation is streamed in real time and as fast as it decodes"
)]
async fn lone_restart_in_both_paces(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        run(world).paced = in_both_paces(&[lone_restart()?])?;
        Ok(())
    })
    .await
}

#[when(
    "a sample of the user's recorded commands is streamed in real time and as fast as it decodes"
)]
async fn sample_in_both_paces(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let timed: Vec<Clip> = recorded()?
            .into_iter()
            .filter(|c| c.word_end.is_some())
            .collect();
        let step = (timed.len() / SAMPLE).max(1);
        let sample: Vec<Clip> = timed.into_iter().step_by(step).take(SAMPLE).collect();
        ensure!(
            !sample.is_empty(),
            "no recorded command with the end of its words"
        );
        run(world).paced = in_both_paces(&sample)?;
        Ok(())
    })
    .await
}

#[then(expr = "both paces report {string}")]
fn both_report(world: &mut E2eWorld, command: String) -> Result<()> {
    let paced = &run(world).paced;
    ensure!(!paced.is_empty(), "no clip was streamed");
    let wrong: Vec<String> = paced
        .iter()
        .filter(|(real, fast)| real.heard != [command.clone()] || fast.heard != [command.clone()])
        .map(|(real, fast)| listed(real, fast))
        .collect();
    ensure!(
        wrong.is_empty(),
        "expected {command:?} in both paces: {wrong:#?}"
    );
    Ok(())
}

#[then(expr = "the fast pace reports each command within {float} s of its real-time delay")]
fn same_delays(world: &mut E2eWorld, within_secs: f64) -> Result<()> {
    let paced = &run(world).paced;
    ensure!(!paced.is_empty(), "no clip was streamed");
    let apart: Vec<String> = paced
        .iter()
        .filter(|(real, fast)| !agree(real, fast, within_secs * 1000.0))
        .map(|(real, fast)| listed(real, fast))
        .collect();
    ensure!(apart.is_empty(), "the paces disagree: {apart:#?}");
    Ok(())
}
