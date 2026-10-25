//! Steps of `local_services.feature` about requests that arrive at once at the copies of the
//! speech recognition service for dictation: the copies share them, on an idle and on a busy
//! machine, and a request that gets no transcript is named with the machine's load.

use std::num::NonZeroUsize;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher_e2e_test::machine_load;
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use super::super::{services, services_dir, ScenarioServices, SPEECH_RECOGNITION};
use super::started_with;
use crate::steps::off_thread;
use crate::E2eWorld;

/// How many copies the service scripts start when no number is set.
const DEFAULT_COPIES: usize = 3;
const REQUESTS_AT_ONCE: usize = 9;
/// How long a shared request may take: three queued short transcriptions on a loaded GPU, up
/// to 10 s each; the app's own deadline for the utterance (about 5.4 s) covers them only on an
/// idle machine.
const LOADED_REQUEST_DEADLINE: Duration = Duration::from_secs(30);
/// A deadline no transcription meets: the request that "gets no transcript in time".
const MISSED_DEADLINE: Duration = Duration::from_millis(50);
/// What a copy logs for every request it transcribes.
const TRANSCRIBED_LOG_LINE: &str = "whisper_vad: detected";
const SHORT_UTTERANCE: &str = "press enter";
/// Cores to keep busy when the machine does not say how many it has.
const CORES_WHEN_UNKNOWN: usize = 8;

/// Keeps every logical core busy while held: one `yes` writing to nowhere per core, as
/// another run beside the scenario would.
#[derive(Debug)]
pub(super) struct BusyProcessor(Vec<Child>);

impl BusyProcessor {
    fn start() -> Result<Self> {
        let cores =
            std::thread::available_parallelism().map_or(CORES_WHEN_UNKNOWN, NonZeroUsize::get);
        (0..cores)
            .map(|_| {
                Command::new("yes")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .context("could not start `yes` to keep a core busy")
            })
            .collect::<Result<Vec<_>>>()
            .map(Self)
    }
}

impl Drop for BusyProcessor {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// The transcript the service at `url` gives for `samples`, through the app's own client.
pub(in super::super) fn transcribe(url: &str, samples: Vec<i16>) -> Result<String> {
    transcribe_within(url, samples, None)
}

/// [`transcribe`], giving the request `deadline` instead of the one its audio length gives.
fn transcribe_within(url: &str, samples: Vec<i16>, deadline: Option<Duration>) -> Result<String> {
    let mut client = OpenAiTranscriber::connect(TranscriptionCfg {
        base_url: url.to_owned(),
        ..TranscriptionCfg::default()
    })?;
    if let Some(deadline) = deadline {
        client = client.with_request_deadline(deadline);
    }
    let transcript = client.transcribe(&AudioBuffer16k::new(samples))?;
    Ok(transcript
        .map(|t| t.as_str().to_owned())
        .unwrap_or_default())
}

/// Each of `count` short requests sent at once to `url`, each with `deadline` when one is
/// given, that got no transcript, numbered from 1, with its error chain or "empty transcript".
pub(super) fn unanswered_at_once(
    url: &str,
    count: usize,
    deadline: Option<Duration>,
) -> Result<Vec<String>> {
    let speech = short_speech()?;
    // Every request is started before the first one is joined: they arrive at once.
    let mut requests = Vec::with_capacity(count);
    for _ in 0..count {
        let (url, speech) = (url.to_owned(), speech.clone());
        requests.push(std::thread::spawn(move || {
            transcribe_within(&url, speech, deadline)
        }));
    }
    Ok(requests
        .into_iter()
        .enumerate()
        .filter_map(|(index, request)| {
            let failure = match request.join() {
                Ok(Ok(text)) if !text.is_empty() => return None,
                Ok(Ok(_)) => "empty transcript".to_owned(),
                Ok(Err(error)) => format!("{error:#}"),
                Err(_) => "its thread panicked".to_owned(),
            };
            Some(format!("request {}: {failure}", index + 1))
        })
        .collect())
}

/// Fails naming every request in `unanswered` when any of `sent` got no transcript, with the
/// machine's load behind it, so the failure says what the copies were busy with.
pub(super) fn ensure_all_answered(unanswered: &[String], sent: usize, when: &str) -> Result<()> {
    ensure!(
        unanswered.is_empty(),
        "{} of {sent} requests got no transcript{when}:\n{}\n{}",
        unanswered.len(),
        unanswered.join("\n"),
        machine_load::report()
    );
    Ok(())
}

pub(in super::super) fn short_speech() -> Result<Vec<i16>> {
    synthesized(&SayRequest { voice: None, words_per_minute: None, text: SHORT_UTTERANCE })
}

/// Sends the shared requests, each with `deadline` when one is given, and keeps what came back.
fn send_at_once(world: &mut E2eWorld, deadline: Option<Duration>) -> Result<()> {
    let services = services(world)?;
    services.pool.unanswered = Some(unanswered_at_once(
        &services.transcription_url(),
        REQUESTS_AT_ONCE,
        deadline,
    )?);
    Ok(())
}

/// Every shared request got its transcript and each of the three copies transcribed some.
fn check_shared(services: &ScenarioServices) -> Result<()> {
    let unanswered = services
        .pool
        .unanswered
        .as_deref()
        .context("no requests were sent")?;
    ensure_all_answered(unanswered, REQUESTS_AT_ONCE, "")?;
    ensure!(
        services.copy_ports.len() == DEFAULT_COPIES,
        "{} copies run, not {DEFAULT_COPIES}: {:?}",
        services.copy_ports.len(),
        services.copy_ports
    );
    for port in &services.copy_ports {
        let log = services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{port}.log"));
        let text =
            std::fs::read_to_string(&log).with_context(|| format!("log {}", log.display()))?;
        ensure!(
            text.contains(TRANSCRIBED_LOG_LINE),
            "the copy on port {port} transcribed none of the requests"
        );
    }
    Ok(())
}

#[given(
    "the service scripts started the speech recognition service for dictation without a number of copies, and another run keeps the machine's processor busy"
)]
async fn started_on_a_busy_machine(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        started_with(world, None)?;
        services(world)?.pool.busy = Some(BusyProcessor::start()?);
        Ok(())
    })
    .await
}

#[when("several transcription requests arrive at once")]
async fn requests_at_once(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        send_at_once(world, Some(LOADED_REQUEST_DEADLINE))
    })
    .await
}

#[when("several transcription requests arrive at once and one of them gets no transcript in time")]
async fn requests_at_once_one_missed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| send_at_once(world, Some(MISSED_DEADLINE))).await
}

#[then("every request gets its transcript, and each of the three copies transcribed some of them")]
fn shared_by_copies(world: &mut E2eWorld) -> Result<()> {
    check_shared(services(world)?)
}

#[then(
    "every request gets its transcript within the time the queued transcriptions take on the busy machine, and each of the three copies transcribed some of them"
)]
fn shared_on_a_busy_machine(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    services.pool.busy = None;
    check_shared(services)
}

#[then("the failure names that request, the copies it waited on and the busiest processes on the machine")]
fn failure_names_the_load(world: &mut E2eWorld) -> Result<()> {
    let failure = match check_shared(services(world)?) {
        Ok(()) => anyhow::bail!(
            "every request got its transcript although none could within {MISSED_DEADLINE:?}"
        ),
        Err(failure) => format!("{failure:#}"),
    };
    for expected in ["request ", "copies ", "busiest processes"] {
        ensure!(
            failure.contains(expected),
            "the failure does not name {expected:?}:\n{failure}"
        );
    }
    Ok(())
}
