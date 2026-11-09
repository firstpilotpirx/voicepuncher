//! The streaming recognizer fed in real time, 20 ms at a time as the microphone gives it, and
//! timed by the wall clock from the end of the command's words to its report.

use std::time::{Duration, Instant};

use cucumber::{then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_command_spotting::protocol::{PhraseSelection, ServerEvent};
use voicepuncher_e2e_test::machine_load::{self, LoadAllowance};
use voicepuncher_e2e_test::voice_commands::{
    silence, spoken_with_pauses, without_trailing_silence,
};

use super::{busy, every_phrase, run, shared, ENGINE_RUNS, SAMPLE_RATE};
use crate::steps::off_thread;
use crate::E2eWorld;

/// 20 ms: about how often the app sends audio.
const CHUNK_SAMPLES: usize = 320;
/// The silence after the words: enough for the utterance to close.
const AFTER_SECS: f32 = 1.5;

/// When the words ended and when each command was reported, by the wall clock.
#[derive(Debug, Default)]
pub struct Timed {
    words_end: Option<Instant>,
    reports: Vec<(String, Instant)>,
}

impl Timed {
    /// The seconds from the end of the words to the first report of `command`.
    fn delay_of(&self, command: &str) -> Option<f32> {
        let end = self.words_end?;
        let (_, at) = self.reports.iter().find(|(c, _)| c == command)?;
        Some(at.saturating_duration_since(end).as_secs_f32())
    }
}

/// Streams `words` and silence after them in real time, polling between chunks.
fn stream_in_real_time(words: &str) -> Result<Timed> {
    let spoken = without_trailing_silence(spoken_with_pauses(words)?);
    let words_secs = spoken.len() as f32 / SAMPLE_RATE as f32;
    let audio = [spoken, silence(AFTER_SECS)].concat();
    let _one_at_a_time = ENGINE_RUNS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut session = shared()?.session_for(0)?;
    session.select(&PhraseSelection { phrases: every_phrase(), ..PhraseSelection::default() })?;
    let began = Instant::now();
    let mut timed =
        Timed { words_end: Some(began + Duration::from_secs_f32(words_secs)), reports: Vec::new() };
    for (i, chunk) in audio.chunks(CHUNK_SAMPLES).enumerate() {
        let due =
            began + Duration::from_secs_f64(((i + 1) * CHUNK_SAMPLES) as f64 / SAMPLE_RATE as f64);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let mut events = session.accept(chunk);
        events.extend(session.poll());
        let at = Instant::now();
        for event in events {
            if let ServerEvent::Command { commands, .. } = event {
                timed.reports.extend(commands.into_iter().map(|c| (c, at)));
            }
        }
    }
    Ok(timed)
}

#[when(expr = "an app streams {string} followed by silence, in real time")]
async fn streams_in_real_time(world: &mut E2eWorld, words: String) -> Result<()> {
    off_thread(world, move |world| {
        run(world).timed = vec![stream_in_real_time(&words)?];
        Ok(())
    })
    .await
}

#[when(expr = "an app streams {string} followed by silence, in real time, {int} times")]
async fn streams_in_real_time_times(
    world: &mut E2eWorld,
    words: String,
    times: usize,
) -> Result<()> {
    off_thread(world, move |world| {
        run(world).timed = (0..times)
            .map(|_| stream_in_real_time(&words))
            .collect::<Result<_>>()?;
        Ok(())
    })
    .await
}

/// The delay of `command` in each run, sorted; fails naming a run that did not report it.
fn delays(world: &mut E2eWorld, command: &str) -> Result<Vec<f32>> {
    let mut delays = Vec::new();
    for timed in &run(world).timed {
        match timed.delay_of(command) {
            Some(delay) => delays.push(delay),
            None => bail!("{command:?} was not reported: {:?}", timed.reports),
        }
    }
    delays.sort_by(f32::total_cmp);
    Ok(delays)
}

/// `bound` stretched by the load of other programs on the machine, and what stretched it.
fn stretched(world: &mut E2eWorld, bound: f32) -> (f32, String) {
    let own_threads = run(world).busy.as_ref().map_or(0, busy::BusyCores::threads);
    let allowance = LoadAllowance::now(own_threads);
    let bound = Duration::from_secs_f32(bound);
    let why = format!("{}\n{}", allowance.describe(bound), machine_load::report());
    (allowance.stretch(bound).as_secs_f32(), why)
}

#[then(
    expr = "the streaming recognizer reports {string} within {float} s of the end of the words by the wall clock"
)]
fn reported_within(world: &mut E2eWorld, command: String, bound: f32) -> Result<()> {
    let delays = delays(world, &command)?;
    let (stretched, why) = stretched(world, bound);
    match delays.last() {
        Some(delay) if *delay <= stretched => Ok(()),
        Some(delay) => {
            bail!("{command:?} came {delay:.3} s after the words, over {bound} s; {why}")
        }
        None => bail!("no stream ran"),
    }
}

#[then(
    expr = "the streaming recognizer reports {string} within {float} s of the end of the words in half the runs and within {float} s in every run"
)]
fn reported_within_in_runs(
    world: &mut E2eWorld,
    command: String,
    median: f32,
    every: f32,
) -> Result<()> {
    let delays = delays(world, &command)?;
    let Some(middle) = delays.get(delays.len().saturating_sub(1) / 2).copied() else {
        bail!("no stream ran");
    };
    let slowest = delays.last().copied().unwrap_or_default();
    let (median_bound, why) = stretched(world, median);
    let (every_bound, _) = stretched(world, every);
    if middle > median_bound || slowest > every_bound {
        bail!("{command:?} came {delays:.3?} s after the words: over {median} s in half the runs or {every} s in one; {why}");
    }
    Ok(())
}
