//! The app's speech recognition client against the stand-in service of `service`: what it
//! sends again, what it gives up and what it logs, through `OpenAiTranscriber`.

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher::{config::TranscriptionCfg, core::application::Transcriber};

mod requests;
mod service;

use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;
use service::{FakeService, Turn, ERROR, HEARD, HEARS, HEARS_NOTHING};

/// The request timeout of the scenarios whose first request hangs: a new attempt follows soon.
const HUNG_FIRST_TIMEOUT_SECS: u64 = 2;
/// The request timeout of the scenario that never gets an answer: shorter than the wait for
/// a short recording, so each attempt ends at it.
const NEVER_ANSWERED_TIMEOUT_SECS: u64 = 1;
/// How long the client waits for a refusing service to come back in these scenarios.
const RECOVERY_WAIT: Duration = Duration::from_millis(300);
/// Three attempts of `NEVER_ANSWERED_TIMEOUT_SECS` and their pauses.
const GIVEN_UP_WITHIN: Duration = Duration::from_secs(5);

/// What one transcription gave: the text, none for no speech, or the error.
pub type Outcome = Result<Option<String>, String>;

/// What a scenario of the speech recognition client keeps between its steps.
#[derive(Default)]
pub struct ServiceRun {
    pub service: Option<FakeService>,
    transcriber: Option<OpenAiTranscriber>,
    timeout_secs: Option<u64>,
    pub audio: Option<AudioBuffer16k>,
    pub outcomes: Vec<Outcome>,
    /// The warnings the scenario's thread logged during its last transcriptions.
    pub warnings: Vec<String>,
    elapsed: Duration,
    /// The scenario's verdict or report, once its When step made one.
    pub verdict: Option<String>,
}

impl std::fmt::Debug for ServiceRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceRun")
            .field("outcomes", &self.outcomes)
            .field("warnings", &self.warnings)
            .finish_non_exhaustive()
    }
}

pub fn service_run(world: &mut IntegrationWorld) -> Result<&mut ServiceRun> {
    world
        .extra
        .speech
        .service
        .as_mut()
        .context("no speech recognition service scenario")
}

fn serve(world: &mut IntegrationWorld, service: Result<FakeService>, timeout: Option<u64>) {
    let run = ServiceRun { service: service.ok(), timeout_secs: timeout, ..ServiceRun::default() };
    world.extra.speech.service = Some(run);
}

#[given("the speech recognition service drops the first two connections and then answers normally")]
fn drops_two(world: &mut IntegrationWorld) {
    serve(
        world,
        FakeService::single(vec![Turn::Drop, Turn::Drop, HEARS]),
        None,
    );
}

#[given("the speech recognition service drops every connection")]
fn drops_all(world: &mut IntegrationWorld) {
    serve(world, FakeService::single(vec![Turn::Drop]), None);
}

#[given("the speech recognition service answers every request with an error")]
fn errors_only(world: &mut IntegrationWorld) {
    serve(world, FakeService::single(vec![ERROR]), None);
}

#[given(
    "the speech recognition service runs as two copies, and the first copy to get a request drops it"
)]
fn copy_drops(world: &mut IntegrationWorld) {
    serve(world, FakeService::copies(2, vec![Turn::Drop, HEARS]), None);
}

#[given(
    "the speech recognition service never answers the first request and answers the next one normally"
)]
fn first_hangs(world: &mut IntegrationWorld) {
    let service = FakeService::single(vec![Turn::Hang, HEARS]);
    serve(world, service, Some(HUNG_FIRST_TIMEOUT_SECS));
}

#[given(
    "the speech recognition service answers the first request with an error and the next one normally"
)]
fn first_errs(world: &mut IntegrationWorld) {
    serve(world, FakeService::single(vec![ERROR, HEARS]), None);
}

#[given(
    "the speech recognition service runs as two copies, and the first copy to get a request never answers it"
)]
fn copy_hangs(world: &mut IntegrationWorld) {
    // The default timeout: the attempt's own deadline, a few seconds for a short recording,
    // passes first, so the copy is given up on as one that sent no answer
    serve(world, FakeService::copies(2, vec![Turn::Hang, HEARS]), None);
}

#[given("the speech recognition service never answers any request")]
fn hangs_always(world: &mut IntegrationWorld) {
    let service = FakeService::single(vec![Turn::Hang]);
    serve(world, service, Some(NEVER_ANSWERED_TIMEOUT_SECS));
}

#[given("the speech recognition service returns no text for a recording")]
fn hears_nothing(world: &mut IntegrationWorld) {
    serve(world, FakeService::single(vec![HEARS_NOTHING]), None);
}

#[given("the speech recognition service answers every request")]
fn answers_all(world: &mut IntegrationWorld) {
    serve(world, FakeService::single(vec![HEARS]), None);
}

#[given("a recording was transcribed through the speech recognition service")]
fn transcribed_once(world: &mut IntegrationWorld) -> Result<()> {
    answers_all(world);
    transcribe(world, second(1), 1)
}

#[given("the speech recognition service has since closed the connection that recording used")]
fn connection_closed(_world: &mut IntegrationWorld) {
    // The stand-in closes every connection once it answered; the pause lets it finish
    std::thread::sleep(Duration::from_millis(200));
}

/// `seconds` of steady sound.
fn second(seconds: usize) -> AudioBuffer16k {
    AudioBuffer16k::new(vec![300; seconds * AudioBuffer16k::SAMPLE_RATE_HZ as usize])
}

impl ServiceRun {
    fn transcriber(&mut self) -> Result<&mut OpenAiTranscriber> {
        if self.transcriber.is_none() {
            let service = self
                .service
                .as_ref()
                .context("the stand-in service did not start")?;
            let mut cfg = TranscriptionCfg {
                base_url: service.base_url.clone(),
                ..TranscriptionCfg::default()
            };
            cfg.timeout_secs = self.timeout_secs.unwrap_or(cfg.timeout_secs);
            let transcriber = OpenAiTranscriber::connect(cfg)?.with_recovery_wait(RECOVERY_WAIT);
            self.transcriber = Some(transcriber);
        }
        self.transcriber.as_mut().context("no transcriber")
    }
}

/// Transcribes `audio` `times` times and keeps what came of it, with the warnings this
/// thread logged meanwhile.
fn transcribe(world: &mut IntegrationWorld, audio: AudioBuffer16k, times: usize) -> Result<()> {
    let log = pipeline_log();
    let run = service_run(world)?;
    let before = log.warnings.lock().unwrap().len();
    let started = Instant::now();
    for _ in 0..times {
        let outcome = run.transcriber()?.transcribe(&audio);
        let outcome = outcome
            .map(|text| text.map(|t| t.as_str().to_owned()))
            .map_err(|e| format!("{e:#}"));
        run.outcomes.push(outcome);
    }
    run.elapsed = started.elapsed();
    let me = std::thread::current().id();
    run.warnings = log.warnings.lock().unwrap()[before..]
        .iter()
        .filter(|(thread, _)| *thread == me)
        .map(|(_, line)| line.clone())
        .collect();
    Ok(())
}

#[when("a recording is transcribed through the speech recognition service")]
fn transcribe_one(world: &mut IntegrationWorld) -> Result<()> {
    transcribe(world, second(1), 1)
}

#[when("a 5-second recording is transcribed through the speech recognition service")]
fn transcribe_five_seconds(world: &mut IntegrationWorld) -> Result<()> {
    transcribe(world, second(5), 1)
}

#[when("the recording is transcribed twice in a row")]
fn transcribe_twice(world: &mut IntegrationWorld) -> Result<()> {
    let audio = service_run(world)?.audio.take().context("no recording")?;
    transcribe(world, audio, 2)
}

fn last_outcome(world: &mut IntegrationWorld) -> Result<Outcome> {
    service_run(world)?
        .outcomes
        .last()
        .cloned()
        .context("nothing was transcribed")
}

/// The warnings of the last transcriptions that contain `fragment`.
fn logged(world: &mut IntegrationWorld, fragment: &str) -> Result<usize> {
    Ok(service_run(world)?
        .warnings
        .iter()
        .filter(|w| w.contains(fragment))
        .count())
}

#[then("the transcript the service returns is passed on")]
fn transcript_passed_on(world: &mut IntegrationWorld) -> Result<()> {
    let outcome = last_outcome(world)?;
    if outcome.as_ref().ok().cloned().flatten().as_deref() != Some(HEARD) {
        bail!("the transcription gave {outcome:?} instead of {HEARD:?}");
    }
    Ok(())
}

#[then(expr = "each of the {int} dropped attempts is logged")]
fn drops_logged(world: &mut IntegrationWorld, count: usize) -> Result<()> {
    let logged = logged(world, "speech recognition service dropped the connection")?;
    if logged != count {
        bail!(
            "{logged} dropped attempts logged: {:?}",
            service_run(world)?.warnings
        );
    }
    Ok(())
}

#[then(expr = "the recording is sent {int} times in total and the transcription fails")]
fn sent_then_failed(world: &mut IntegrationWorld, count: usize) -> Result<()> {
    let outcome = last_outcome(world)?;
    let run = service_run(world)?;
    let sent = run.service.as_ref().map_or(0, |s| s.taken().len());
    if outcome.is_ok() || sent != count {
        bail!("sent {sent} times, the transcription gave {outcome:?}");
    }
    Ok(())
}

#[then("another copy answered, and the copy that failed the first request got no second attempt")]
fn other_copy_answered(world: &mut IntegrationWorld) -> Result<()> {
    let run = service_run(world)?;
    let taken = run.service.as_ref().context("no service")?.taken();
    let ports: Vec<u16> = taken.iter().map(|t| t.port).collect();
    if ports.len() != 2 || ports[0] == ports[1] {
        bail!(
            "the copies took the requests on {ports:?}: one each, the failed one first, expected"
        );
    }
    Ok(())
}

#[then("the hung attempt is logged")]
fn hang_logged(world: &mut IntegrationWorld) -> Result<()> {
    let logged = logged(world, "did not answer in time")? + logged(world, " sent no answer in ")?;
    if logged != 1 {
        bail!(
            "{logged} hung attempts logged: {:?}",
            service_run(world)?.warnings
        );
    }
    Ok(())
}

#[then("the error answer is logged")]
fn error_logged(world: &mut IntegrationWorld) -> Result<()> {
    let logged = logged(world, "speech recognition service answered with an error")?;
    if logged != 1 {
        bail!(
            "{logged} error answers logged: {:?}",
            service_run(world)?.warnings
        );
    }
    Ok(())
}

#[then("the transcription fails after 3 attempts, given up within 5 seconds")]
fn given_up_quickly(world: &mut IntegrationWorld) -> Result<()> {
    sent_then_failed(world, 3)?;
    let elapsed = service_run(world)?.elapsed;
    if elapsed >= GIVEN_UP_WITHIN {
        bail!("the transcription was given up after {elapsed:?}");
    }
    Ok(())
}

#[then("every attempt was logged")]
fn attempts_logged(world: &mut IntegrationWorld) -> Result<()> {
    let attempts = logged(world, "speech recognition service")?;
    let given_up = logged(world, "— giving up")?;
    if attempts != 3 || given_up != 1 {
        bail!(
            "{attempts} attempts and {given_up} give-ups logged: {:?}",
            service_run(world)?.warnings
        );
    }
    Ok(())
}

#[then("no text is produced and no failure is reported")]
fn nothing_heard(world: &mut IntegrationWorld) -> Result<()> {
    let outcome = last_outcome(world)?;
    if outcome != Ok(None) {
        bail!("the transcription gave {outcome:?} instead of no text");
    }
    Ok(())
}

#[then("no dropped attempt is logged, and each recording was sent once")]
fn no_drop_logged(world: &mut IntegrationWorld) -> Result<()> {
    let dropped = logged(world, "dropped the connection")?;
    let run = service_run(world)?;
    let sent = run.service.as_ref().map_or(0, |s| s.taken().len());
    if dropped != 0 || sent != 2 {
        bail!(
            "{dropped} drops logged, {sent} requests for 2 recordings: {:?}",
            run.warnings
        );
    }
    Ok(())
}
