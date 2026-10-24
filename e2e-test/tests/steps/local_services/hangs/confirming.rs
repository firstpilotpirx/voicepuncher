//! Steps of `local_services.feature` about confirmations that are slow or hung: a command
//! spotting service streamed to while one confirmation hangs, and a confirmation thread of
//! the service asked confirmations by a speech recognition service that is late.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::core::domain::VoiceCommand;
use voicepuncher_command_spotting::confirm::{Hear, Heard};
use voicepuncher_command_spotting::worker::{Answer, ConfirmationWorker, Job, JobId, JobKind};
use voicepuncher_e2e_test::voice_commands::{
    recognized_commands, spoken_with_pauses, spotting_events,
};

use super::super::confirmations::{log_len, log_since_speech, GIVEN_UP};
use super::super::services;
use super::speech_as_f32;
use crate::steps::off_thread;
use crate::E2eWorld;

const PRESS_ENTER: &str = r#"{"text":" press enter","segments":[]}"#;
/// How long the confirmation thread waits for one attempt.
const DEADLINE: Duration = Duration::from_millis(200);
/// How long the steps wait for an answer of the confirmation thread.
const ANSWER_WAIT: Duration = Duration::from_secs(2);
/// How late the slow speech recognition service answers.
const LATE: Duration = Duration::from_millis(30);
/// More confirmations than a stalled thread may hold waiting.
const MORE_THAN_THE_BOUND: u64 = 8;
const CONNECTION: u64 = 1;

/// A confirmation thread of the command spotting service, and what it answered.
pub struct Confirmations {
    worker: ConfirmationWorker,
    answers: Vec<Answer>,
}

impl std::fmt::Debug for Confirmations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Confirmations")
            .field("answers", &self.answers.len())
            .finish_non_exhaustive()
    }
}

/// A speech recognition service for commands on `listener` that never answers its first
/// request and hears "press enter" in every later one.
fn hanging_once(listener: TcpListener) {
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for mut stream in listener.incoming().flatten() {
            let mut buf = vec![0u8; 65536];
            let _ = stream.read(&mut buf);
            if held.is_empty() {
                // kept open and unanswered until the scenario ends
                held.push(stream);
                continue;
            }
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{PRESS_ENTER}",
                PRESS_ENTER.len()
            );
        }
    });
}

#[given("the speech recognition service for commands never answers one confirmation")]
fn one_confirmation_hangs(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    hanging_once(TcpListener::bind(("127.0.0.1", services.command_stt_port))?);
    Ok(())
}

#[when(
    expr = "an app streams speech that is no command and then {string} to the command spotting service"
)]
async fn streams_after_speech(world: &mut E2eWorld, phrase: String) -> Result<()> {
    off_thread(world, move |world| {
        let services = services(world)?;
        services.log_before_speech = Some(log_len(services)?);
        let mut audio = speech_as_f32()?;
        audio.extend(spoken_with_pauses(&phrase)?);
        // every stretch of audio waits for the service's answer, so a service that stops
        // answering while the confirmation hangs fails the step
        let events = spotting_events(
            &services.command_spotting_url(),
            &audio,
            &VoiceCommand::STANDBY,
        )?;
        world.recognized = recognized_commands(events);
        Ok(())
    })
    .await
}

#[then(
    "the connection answered the app throughout, and the hung confirmation is logged as given up"
)]
fn hung_given_up(world: &mut E2eWorld) -> Result<()> {
    let log = log_since_speech(services(world)?)?;
    ensure!(
        log.contains(GIVEN_UP),
        "no confirmation was given up:\n{log}"
    );
    Ok(())
}

/// Hears "press enter", after failing the first `failures` attempts at their deadline.
struct LateAtFirst {
    failures: u32,
    calls: AtomicU32,
    late: Duration,
}

impl Hear for LateAtFirst {
    fn hear(&self, _: &[f32], deadline: Duration) -> Result<Heard> {
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.failures {
            std::thread::sleep(deadline);
            return Err(anyhow!("timed out reading response"));
        }
        std::thread::sleep(self.late);
        Ok(Heard { text: " press enter".to_owned(), words: Vec::new() })
    }
}

fn confirmations_with(world: &mut E2eWorld, hear: LateAtFirst) -> Result<()> {
    let worker = ConfirmationWorker::start(CONNECTION, Arc::new(hear), DEADLINE)?;
    world.owned_services.confirmations = Some(Confirmations { worker, answers: Vec::new() });
    Ok(())
}

fn confirmations(world: &mut E2eWorld) -> Result<&mut Confirmations> {
    world
        .owned_services
        .confirmations
        .as_mut()
        .context("no confirmation thread runs")
}

fn job(n: u64, kind: JobKind) -> Job {
    Job { id: JobId { conn: CONNECTION, n }, kind, audio: vec![0.0; 1_600] }
}

#[given("the speech recognition service for commands does not answer the first confirmation in time and answers the second")]
fn late_first(world: &mut E2eWorld) -> Result<()> {
    confirmations_with(
        world,
        LateAtFirst { failures: 1, calls: AtomicU32::new(0), late: Duration::ZERO },
    )
}

#[when(expr = "the confirmation of {string} said alone is asked")]
fn said_alone(world: &mut E2eWorld, phrase: String) -> Result<()> {
    VoiceCommand::from_phrase(&phrase).with_context(|| format!("{phrase:?} is no command"))?;
    let run = confirmations(world)?;
    // a command said alone is decided by the confirmation of its whole speech segment
    run.worker.submit(job(1, JobKind::Segment));
    run.answers.extend(run.worker.next_answer(ANSWER_WAIT));
    let heard: Vec<String> = run
        .answers
        .iter()
        .filter_map(|a| a.heard.as_ref())
        .map(|h| h.text.clone())
        .collect();
    world.recognized = heard
        .iter()
        .filter_map(|text| VoiceCommand::from_phrase(text.trim()))
        .collect();
    Ok(())
}

#[given("the speech recognition service for commands answers every confirmation, late")]
fn late_always(world: &mut E2eWorld) -> Result<()> {
    confirmations_with(
        world,
        LateAtFirst { failures: 0, calls: AtomicU32::new(0), late: LATE },
    )
}

#[when("more confirmations than the bound are asked on one thread at once")]
fn many_at_once(world: &mut E2eWorld) -> Result<()> {
    let run = confirmations(world)?;
    for n in 1..=MORE_THAN_THE_BOUND {
        run.worker.submit(job(n, JobKind::Spotted));
    }
    for _ in 1..=MORE_THAN_THE_BOUND {
        run.answers.extend(run.worker.next_answer(ANSWER_WAIT));
    }
    Ok(())
}

#[then("every one of them is answered with what was heard")]
fn every_one_answered(world: &mut E2eWorld) -> Result<()> {
    let answers = &confirmations(world)?.answers;
    ensure!(
        answers.len() as u64 == MORE_THAN_THE_BOUND,
        "{} answers",
        answers.len()
    );
    let refused = answers.iter().filter(|a| a.heard.is_none()).count();
    ensure!(
        refused == 0,
        "{refused} confirmations were refused while the thread kept answering"
    );
    Ok(())
}
