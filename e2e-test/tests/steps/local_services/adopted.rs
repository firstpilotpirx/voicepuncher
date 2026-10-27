//! Copies an app run left running when it ended, with nothing watching them any more: the app
//! adopts them, and starts its own copy when one of them stops.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::adapters::secondary::localservices::catalog::{self, Needs};
use voicepuncher::adapters::secondary::localservices::process::{HttpProbe, ProcessLauncher};
use voicepuncher::config::Config;
use voicepuncher::core::application::{ProbeAnswer, ServiceLauncher, ServiceProbe, ServiceProcess};
use voicepuncher::core::domain::{ServiceKind, ServiceState, SpottingEvent, VoiceCommand};
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use super::managed::{kill_copy, log_mark, logged_since, machine, process_on, ManagedRun, Needed};
use super::owned::owned;
use super::{healthy, wait_until};
use crate::steps::off_thread;
use crate::E2eWorld;

/// Loading the models of every copy takes up to a minute on a busy machine.
const READY_WITHIN: Duration = Duration::from_secs(240);
/// Three failed checks 5 s apart, then the start of the app's own copy.
const REPLACED_WITHIN: Duration = Duration::from_secs(30);
/// How long the replaced copy may take to report the command it heard.
const HEARD_WITHIN: Duration = Duration::from_secs(10);
const COMMAND: VoiceCommand = VoiceCommand::PressEnter;
const FRAME_SAMPLES: usize = 1_600;
/// Silence after the command, so the service hears it end.
const TRAILING_SILENCE: usize = 16_000;

/// The copies an app run left running, and the one a step ended.
#[derive(Default)]
pub struct AdoptedCopies {
    left: Vec<Box<dyn ServiceProcess>>,
    /// Holds the copies' logs until they end.
    _home: Option<tempfile::TempDir>,
    /// The copy that ended: its port and process.
    pub ended: Option<(u16, u32)>,
    /// Where the log stood when the copy ended.
    ended_at: usize,
}

impl std::fmt::Debug for AdoptedCopies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdoptedCopies")
            .field("left", &self.left.len())
            .field("ended", &self.ended)
            .finish_non_exhaustive()
    }
}

impl Drop for AdoptedCopies {
    fn drop(&mut self) {
        for copy in &mut self.left {
            copy.terminate();
        }
    }
}

impl AdoptedCopies {
    /// Starts every copy `config` needs as the app would, one after the other once the one
    /// before answers, and leaves them without anything watching them.
    fn left_running(config: &Config) -> Result<Self> {
        let home = tempfile::tempdir()?;
        let machine = machine(home.path());
        let probe = HttpProbe::new();
        let mut copies = Self { left: Vec::new(), _home: Some(home), ended: None, ended_at: 0 };
        for spec in catalog::needed(config, Needs::of(config), &machine, &probe) {
            copies.left.push(ProcessLauncher.spawn(&spec)?);
            let answered = wait_until(READY_WITHIN, || {
                probe.answer(spec.kind, spec.port) == ProbeAnswer::Ready
            });
            ensure!(
                answered,
                "{} did not answer within {READY_WITHIN:?}",
                spec.label
            );
        }
        Ok(copies)
    }
}

#[given("voice control is on and the app adopted both copies of the command spotting service an app run left running")]
async fn spotting_copies_adopted(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let config = ManagedRun::config_for(Needed { dictation_copies: 1, voice_control: true })?;
        let left = AdoptedCopies::left_running(&config)?;
        let run = ManagedRun::start_with(config)?;
        let spotting: Vec<_> = run
            .statuses()
            .into_iter()
            .filter(|status| status.kind == ServiceKind::CommandSpotting)
            .collect();
        ensure!(
            spotting.len() == 2 && spotting.iter().all(|s| s.state == ServiceState::Adopted),
            "the app did not adopt both command spotting copies: {spotting:?}"
        );
        let owned = owned(world);
        owned.adopted = left;
        owned.run = Some(run);
        Ok(())
    })
    .await
}

#[when("the process of one adopted command spotting copy ends")]
fn spotting_copy_ends(world: &mut E2eWorld) -> Result<()> {
    let owned = owned(world);
    let port = *owned
        .run()?
        .ports(ServiceKind::CommandSpotting)
        .first()
        .context("no command spotting copy runs")?;
    owned.adopted.ended_at = log_mark();
    let pid = kill_copy(port)?;
    owned.adopted.ended = Some((port, pid));
    Ok(())
}

#[then("within 30 seconds the app has started its own command spotting copy on that port and it answers")]
async fn own_spotting_copy_answers(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let owned = owned(world);
        let (port, old) = owned.adopted.ended.context("no adopted copy ended")?;
        let replaced = wait_until(REPLACED_WITHIN, || {
            healthy(port) && process_on(port).is_some_and(|new| new != old)
        });
        ensure!(
            replaced,
            "no copy of the app's own answered on port {port} within {REPLACED_WITHIN:?}"
        );
        let status = owned
            .run()?
            .statuses()
            .into_iter()
            .find(|status| status.port == port)
            .context("the app no longer lists the copy")?;
        ensure!(
            status.owned,
            "the copy on port {port} is not the app's own: {status:?}"
        );
        Ok(())
    })
    .await
}

#[then("the log names the command spotting copy, its port and that it stopped")]
fn ending_logged(world: &mut E2eWorld) -> Result<()> {
    let owned = owned(world);
    let (port, _) = owned.adopted.ended.context("no adopted copy ended")?;
    let label = owned
        .run()?
        .statuses()
        .into_iter()
        .find(|status| status.port == port)
        .map(|status| status.label)
        .context("the app no longer lists the copy")?;
    let named = format!("{label} at port {port} stopped");
    let lines = logged_since(owned.adopted.ended_at, &named);
    ensure!(
        lines.len() == 1,
        "expected one line naming {named:?}, got {lines:?}"
    );
    Ok(())
}

#[then("a voice command said to that copy is heard")]
async fn command_heard(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let owned = owned(world);
        let (port, _) = owned.adopted.ended.context("no adopted copy ended")?;
        let url = format!("ws://127.0.0.1:{port}");
        let heard = heard_on(&url)?;
        ensure!(
            heard.contains(&COMMAND),
            "the copy on port {port} heard {heard:?}"
        );
        Ok(())
    })
    .await
}

/// The commands the command spotting copy at `url` hears in the spoken command.
fn heard_on(url: &str) -> Result<Vec<VoiceCommand>> {
    let speech = spoken_command()?;
    let mut client = SpottingClient::connect(url, Instant::now())?;
    client.listen_for(&VoiceCommand::STANDBY)?;
    for frame in speech.chunks(FRAME_SAMPLES) {
        client.send_audio(frame)?;
    }
    let events = client.events_for_sent_audio(HEARD_WITHIN)?;
    client.close();
    Ok(events.into_iter().flat_map(commands_in).collect())
}

/// The command, spoken, with silence after it.
fn spoken_command() -> Result<Vec<f32>> {
    let request = SayRequest { voice: None, words_per_minute: None, text: COMMAND.phrase() };
    let mut speech: Vec<f32> = synthesized(&request)?
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect();
    speech.extend(std::iter::repeat_n(0.0, TRAILING_SILENCE));
    Ok(speech)
}

/// The commands one event of the command spotting service reports.
fn commands_in(event: SpottingEvent) -> Vec<VoiceCommand> {
    match event {
        SpottingEvent::PhraseSpotted { phrase, .. } => {
            VoiceCommand::from_phrase(&phrase).into_iter().collect()
        }
        SpottingEvent::CommandsSaid { commands, .. } => commands,
        _ => Vec::new(),
    }
}
