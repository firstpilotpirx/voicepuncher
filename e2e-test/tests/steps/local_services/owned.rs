//! Services the app starts as copies itself, through outages a step makes: a copy that
//! stops answering, every dictation copy killed during a chunk.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::core::domain::ServiceKind;
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use super::managed::{kill_copy, log_mark, logged_since, process_on, ManagedRun, Needed};
use super::{healthy, wait_until};
use crate::steps::off_thread;
use crate::E2eWorld;

/// Three failed checks 5 s apart, then the restart: within the 30 s the scenario names.
const RESTARTED_WITHIN: Duration = Duration::from_secs(30);
/// A restarted copy loads its model before it answers.
const ANSWERS_WITHIN: Duration = Duration::from_secs(90);
/// How long the chunk is on its way before its copies are killed.
const KILLED_AFTER: Duration = Duration::from_millis(300);
/// What the dictation client logs when it waits for a copy to come back.
const WAITING_FOR_A_COPY: &str = "speech recognition: no copy accepts connections — waiting";
/// About ten seconds of speech: the copies take a while to transcribe it.
const CHUNK_SPEECH: &str = "The quarterly report is ready for review. Please read the summary \
     before the meeting on Friday and send your comments to the team by Thursday evening.";
const CHUNK_WORD: &str = "quarterly";

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
    /// The confirmation thread a scenario asks confirmations of, without a service.
    pub(super) confirmations: Option<super::hangs::confirming::Confirmations>,
    /// The command spotting engine a scenario drives in this process, and what it reported.
    pub scheduling: crate::steps::voice_control::spotting_service::scheduling::SchedulingRun,
}

impl OwnedServices {
    pub fn run(&mut self) -> Result<&mut ManagedRun> {
        self.run.as_mut().context("the app started no services")
    }
}

fn owned(world: &mut E2eWorld) -> &mut OwnedServices {
    &mut world.services.owned_services
}

#[given(
    "the app started the speech recognition service for dictation as two copies and they are ready"
)]
async fn dictation_copies_ready(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        owned(world).run = Some(ManagedRun::start(Needed {
            dictation_copies: 2,
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
    off_thread(world, kill_every_copy_during_a_chunk).await
}

/// What `transcriber` heard in `speech`, or its error as text.
fn transcribed(
    transcriber: &mut OpenAiTranscriber,
    speech: Vec<i16>,
) -> std::result::Result<Option<String>, String> {
    transcriber
        .transcribe(&AudioBuffer16k::new(speech))
        .map(|heard| heard.map(|text| text.as_str().to_owned()))
        .map_err(|e| format!("{e:#}"))
}

/// Sends a chunk and kills every copy of the dictation service while it is transcribed.
fn kill_every_copy_during_a_chunk(world: &mut E2eWorld) -> Result<()> {
    let owned = owned(world);
    let run = owned.run()?;
    let speech =
        synthesized(&SayRequest { voice: None, words_per_minute: None, text: CHUNK_SPEECH })?;
    let mut transcriber = OpenAiTranscriber::connect(run.config.transcription.clone())?;
    let sent_at = log_mark();
    let chunk = std::thread::spawn(move || transcribed(&mut transcriber, speech));
    std::thread::sleep(KILLED_AFTER);
    let killed = run
        .ports(ServiceKind::Dictation)
        .into_iter()
        .try_for_each(|port| kill_copy(port).map(drop));
    let heard = chunk.join();
    killed?;
    owned.sent_at = sent_at;
    owned.heard = Some(heard.map_err(|_| anyhow::anyhow!("the chunk panicked"))?);
    Ok(())
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
