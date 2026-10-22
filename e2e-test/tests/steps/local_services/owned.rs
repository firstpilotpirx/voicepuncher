//! Services the app starts as copies itself, through outages a step makes: a copy that
//! stops answering, every dictation copy killed during a chunk, a refinement copy killed
//! while it refines.

use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::core::application::{TextRefiner, Transcriber};
use voicepuncher::core::domain::{AudioBuffer16k, RawTranscript, ServiceKind};
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use super::managed::{log_mark, logged_since, process_on, ManagedRun, Needed};
use super::{healthy, wait_until};
use crate::steps::off_thread;
use crate::E2eWorld;

/// Three failed checks 5 s apart, then the restart: within the 30 s the scenario names.
const RESTARTED_WITHIN: Duration = Duration::from_secs(30);
/// A restarted copy loads its model before it answers.
const ANSWERS_WITHIN: Duration = Duration::from_secs(90);
/// How long the chunk is on its way before its copies are killed.
const KILLED_AFTER: Duration = Duration::from_millis(300);
/// How long the refinement is on its way, at most, before the step looks for its copy.
const REFINING_WITHIN: Duration = Duration::from_secs(5);
/// What the dictation client logs when it waits for a copy to come back.
const WAITING_FOR_A_COPY: &str = "speech recognition: no copy accepts connections — waiting";
/// About ten seconds of speech: the copies take a while to transcribe it.
const CHUNK_SPEECH: &str = "The quarterly report is ready for review. Please read the summary \
     before the meeting on Friday and send your comments to the team by Thursday evening.";
const CHUNK_WORD: &str = "quarterly";
const TRANSCRIPT: &str = "so um the quarterly report is uh ready for review and please read \
     the summary before the meeting on friday and um send your comments to the team by \
     thursday evening because we want to uh close it out before the weekend";

/// What one scenario's services and outages left behind.
#[derive(Debug, Default)]
pub struct OwnedServices {
    pub run: Option<ManagedRun>,
    /// The copy a step stopped or killed: its port and its process.
    pub stopped: Option<(u16, u32)>,
    /// The other copies of that service: their ports and processes.
    others: Vec<(u16, u32)>,
    heard: Option<std::result::Result<Option<String>, String>>,
    /// Where the log stood when the chunk was sent.
    sent_at: usize,
    refined: Option<std::result::Result<String, String>>,
}

impl OwnedServices {
    pub fn run(&mut self) -> Result<&mut ManagedRun> {
        self.run.as_mut().context("the app started no services")
    }
}

fn owned(world: &mut E2eWorld) -> &mut OwnedServices {
    &mut world.owned_services
}

#[given(
    "the app started the speech recognition service for dictation as two copies and they are ready"
)]
async fn dictation_copies_ready(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        owned(world).run = Some(ManagedRun::start(Needed {
            dictation_copies: 2,
            refinement_copies: None,
            voice_control: false,
        })?);
        Ok(())
    })
    .await
}

#[given("the app started the text refinement service as two copies and they are ready")]
async fn refinement_copies_ready(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        owned(world).run = Some(ManagedRun::start(Needed {
            dictation_copies: 1,
            refinement_copies: Some(2),
            voice_control: false,
        })?);
        Ok(())
    })
    .await
}

#[when("one copy stops answering without exiting")]
fn copy_stops_answering(world: &mut E2eWorld) -> Result<()> {
    let owned = owned(world);
    let run = owned.run()?;
    let ports = run.ports(ServiceKind::Dictation);
    let (stopped, others) = ports.split_first().context("no dictation copy runs")?;
    let others: Vec<(u16, u32)> = others
        .iter()
        .filter_map(|port| process_on(*port).map(|pid| (*port, pid)))
        .collect();
    let pid = run.pause(*stopped)?;
    owned.stopped = Some((*stopped, pid));
    owned.others = others;
    Ok(())
}

#[when("every copy of it is killed while a dictation's chunk is being transcribed")]
async fn every_copy_killed_during_a_chunk(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let owned = owned(world);
        let run = owned.run()?;
        let speech =
            synthesized(&SayRequest { voice: None, words_per_minute: None, text: CHUNK_SPEECH })?;
        let mut transcriber = OpenAiTranscriber::connect(run.config.transcription.clone())?;
        let sent_at = log_mark();
        let chunk = std::thread::spawn(move || {
            transcriber
                .transcribe(&AudioBuffer16k::new(speech))
                .map(|heard| heard.map(|text| text.as_str().to_owned()))
                .map_err(|e| format!("{e:#}"))
        });
        std::thread::sleep(KILLED_AFTER);
        for port in run.ports(ServiceKind::Dictation) {
            run.kill(port)?;
        }
        owned.sent_at = sent_at;
        owned.heard = Some(
            chunk
                .join()
                .map_err(|_| anyhow::anyhow!("the chunk panicked"))?,
        );
        Ok(())
    })
    .await
}

#[when("the copy refining a transcript is killed before it answers")]
async fn refining_copy_killed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let owned = owned(world);
        let run = owned.run()?;
        let mut refiner = OpenAiTextRefiner::connect(run.config.refinement.clone())?;
        let refinement = std::thread::spawn(move || {
            let raw = RawTranscript::new(TRANSCRIPT.to_owned()).map_err(|e| format!("{e:?}"))?;
            refiner
                .refine(&raw)
                .map(|text| text.as_str().to_owned())
                .map_err(|e| format!("{e:#}"))
        });
        let ports = run.ports(ServiceKind::Refinement);
        let refining = std::cell::Cell::new(None);
        wait_until(REFINING_WITHIN, || {
            refining.set(run.connected_copy(&ports));
            refining.get().is_some()
        });
        let port = refining
            .get()
            .context("no copy of the text refinement service took the refinement")?;
        let pid = run.kill(port)?;
        owned.stopped = Some((port, pid));
        owned.refined = Some(
            refinement
                .join()
                .map_err(|_| anyhow::anyhow!("the refinement panicked"))?,
        );
        Ok(())
    })
    .await
}

#[then("within 30 seconds the app has killed that copy and started it again, and it answers on its port")]
async fn stopped_copy_restarted(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let (port, old) = owned(world).stopped.context("no copy was stopped")?;
        let replaced = wait_until(RESTARTED_WITHIN, || {
            process_on(port).is_some_and(|new| new != old)
        });
        ensure!(
            replaced,
            "the copy on port {port} (process {old}) was not started again within {RESTARTED_WITHIN:?}"
        );
        ensure!(
            wait_until(ANSWERS_WITHIN, || healthy(port)),
            "the restarted copy on port {port} does not answer"
        );
        Ok(())
    })
    .await
}

#[then("the other copies were never restarted")]
fn others_kept(world: &mut E2eWorld) -> Result<()> {
    let owned = owned(world);
    ensure!(!owned.others.is_empty(), "no other copy was watched");
    for (port, pid) in &owned.others {
        ensure!(
            process_on(*port) == Some(*pid),
            "the copy on port {port} was restarted as well"
        );
    }
    Ok(())
}

#[then("the chunk's text comes back once the app has started the copies again, and no failure is reported")]
fn chunk_text_back(world: &mut E2eWorld) -> Result<()> {
    match owned(world).heard.clone().context("no chunk was sent")? {
        Ok(Some(text)) if text.to_lowercase().contains(CHUNK_WORD) => Ok(()),
        Ok(heard) => bail!("the chunk came back as {heard:?}"),
        Err(e) => bail!("the chunk failed: {e}"),
    }
}

#[then("the wait for the service is logged")]
fn wait_logged(world: &mut E2eWorld) -> Result<()> {
    ensure!(
        !logged_since(owned(world).sent_at, WAITING_FOR_A_COPY).is_empty(),
        "the dictation client logged no wait for a copy"
    );
    Ok(())
}

#[then("the other copy refines the transcript, and the text is not reported as pasted without refinement")]
fn other_copy_refined(world: &mut E2eWorld) -> Result<()> {
    match owned(world)
        .refined
        .clone()
        .context("no refinement was sent")?
    {
        Ok(text) if !text.trim().is_empty() => Ok(()),
        Ok(text) => bail!("the refinement came back empty: {text:?}"),
        Err(e) => bail!("the refinement failed, so the raw text would be pasted: {e}"),
    }
}
