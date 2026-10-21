//! Drives the real runtime with an assembler of stand-ins: every pipeline it builds records
//! what its parts were asked to do under the pipeline's generation, a fake keyboard feeds the
//! hotkey binding the runtime holds, and an observer records every runtime state in the same
//! ordered log.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::config::Config;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, FocusProbe, FocusWaitCfg, HotkeyListener,
    KeyPresser, PermissionProbe, PermissionsGate, PipelineInput, SoundPlayer, TextPaster,
    Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, FocusedElement, HotKey, HotkeyBinding, Permission,
    PermissionStatus, PttSignal, RawTranscript, RecordingMode, VoiceCommand,
};
use voicepuncher::runtime::{
    PermissionsWatch, PipelineAssembler, Reloader, Runtime, RuntimeObserver, RuntimeState,
};

use crate::IntegrationWorld;

pub(crate) mod models;
mod services;

const DICTATED: &str = "Meeting moved to Friday.";
const FIRST_HOTKEY: HotKey = HotKey::RightOption;
const SECOND_HOTKEY: HotKey = HotKey::F19;
const THIRD_HOTKEY: HotKey = HotKey::F18;
/// Not on loopback: the configuration does not validate.
const REMOTE_SERVICE: &str = "http://192.0.2.1:8081";
const SILENT_SERVICE_ERROR: &str =
    "speech recognition service is not reachable at http://127.0.0.1:8081";
const FOCUS_POLL: Duration = Duration::from_millis(10);
/// Longest a runtime or pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(2);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// How often the runtime checks the permissions in these scenarios.
const PERMISSIONS_CHECK: Duration = Duration::from_millis(20);

type Generation = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Assembled(Generation, HotKey),
    /// The start sound and whether the pipeline refines, as its configuration set them.
    Configured(Generation, String, bool),
    RecordingStarted(Generation),
    CancelPlayed(Generation),
    Pasted(Generation),
    ListeningOn(Generation),
    ListeningOff(Generation),
    PipelineDropped(Generation),
    Notice(Generation, String),
    State(RuntimeState),
    ServicesStopped,
}

type Events = Arc<Mutex<Vec<Event>>>;

fn push(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

/// What an app-runtime scenario shares with the runtime and its pipelines.
pub struct RuntimeRun {
    events: Events,
    config: Config,
    services_answer: Arc<Mutex<bool>>,
    text_field_focused: Arc<Mutex<bool>>,
    keyboard: FakeKeyboard,
    runtime: Option<Runtime>,
    started: Option<Result<()>>,
    process: u32,
    rejection_log: Vec<String>,
    permissions: Permissions,
    services: Option<services::FakeServices>,
    models: Option<models::ModelsFolder>,
}

impl std::fmt::Debug for RuntimeRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeRun")
            .field("events", &self.events())
            .finish()
    }
}

impl Default for RuntimeRun {
    fn default() -> Self {
        // the logger is the process's one; the voice-control steps install it
        crate::steps::voice_control::pipeline_log();
        Self {
            events: Events::default(),
            config: config_with(FIRST_HOTKEY),
            services_answer: Arc::new(Mutex::new(true)),
            text_field_focused: Arc::new(Mutex::new(true)),
            keyboard: FakeKeyboard::default(),
            runtime: None,
            started: None,
            process: std::process::id(),
            rejection_log: Vec::new(),
            permissions: Permissions::default(),
            services: None,
            models: None,
        }
    }
}

impl Drop for RuntimeRun {
    fn drop(&mut self) {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.stop();
        }
    }
}

fn config_with(hotkey: HotKey) -> Config {
    let mut config = Config::default();
    config.hotkey.key = hotkey;
    config.hotkey.min_press_ms = 0;
    config.hotkey.recording_mode = RecordingMode::Toggle;
    config
}

impl RuntimeRun {
    /// Counts the dictations pasted so far whenever it is called, from any thread.
    pub(crate) fn paste_counter(&self) -> impl Fn() -> usize + Send + 'static {
        let events = Arc::clone(&self.events);
        move || {
            let events = events.lock().unwrap();
            events
                .iter()
                .filter(|e| matches!(e, Event::Pasted(_)))
                .count()
        }
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn start(&mut self) {
        let assembler = self.assembler();
        let mut runtime = Runtime::new(Box::new(assembler), Box::new(self.keyboard.clone()))
            .with_permissions(PermissionsWatch {
                gate: self.permissions.gate(),
                while_ready: PERMISSIONS_CHECK,
                while_waiting: PERMISSIONS_CHECK,
            });
        if let Some(services) = &self.services {
            runtime = runtime.with_services(Box::new(services.clone()));
        }
        if let Some(models) = &self.models {
            runtime = runtime.with_models(models.gate());
        }
        runtime.observe(Box::new(RecordingObserver(Arc::clone(&self.events))));
        let mut runtime = runtime;
        self.started = Some(runtime.start(self.config.clone()));
        self.runtime = Some(runtime);
    }

    pub(crate) fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    pub(crate) fn launch(&mut self) {
        self.start();
    }

    pub(crate) fn start_waiting(&mut self) -> Result<()> {
        self.start();
        self.state_becomes(|s| matches!(s, RuntimeState::WaitingForPermissions { .. }))
    }

    /// The start returned, and the process still runs it.
    pub(crate) fn started_ok(&mut self) -> Result<()> {
        self.started.take().context("the runtime was not started")?
    }

    pub(crate) fn assembled(&self, generation: Generation) -> Result<()> {
        self.wait_for(|e| matches!(e, Event::Assembled(g, _) if *g == generation))?;
        self.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))
    }

    pub(crate) fn same_process(&self) -> bool {
        self.process == std::process::id()
    }

    pub(crate) fn ever_waited_for_permissions(&self) -> bool {
        self.count(|e| matches!(e, Event::State(RuntimeState::WaitingForPermissions { .. }))) > 0
    }

    pub(crate) fn waits_for(&self, missing: &[Permission], revoked: &[Permission]) -> Result<()> {
        self.state_becomes(|s| {
            *s == RuntimeState::WaitingForPermissions {
                missing: missing.to_vec(),
                revoked: revoked.to_vec(),
            }
        })
    }

    pub(crate) fn pipeline_dropped(&self, generation: Generation) -> Result<()> {
        self.wait_for(|e| *e == Event::PipelineDropped(generation))?;
        Ok(())
    }

    pub(crate) fn cancelled_and_nothing_pasted(&self) -> Result<()> {
        self.wait_for(|e| *e == Event::CancelPlayed(1))?;
        std::thread::sleep(SETTLE);
        if self.count(|e| matches!(e, Event::Pasted(_))) > 0 {
            bail!("text was pasted: {:?}", self.events());
        }
        Ok(())
    }

    pub(crate) fn notice(&self, generation: Generation) -> Result<String> {
        match self.wait_for(|e| matches!(e, Event::Notice(g, _) if *g == generation))? {
            Event::Notice(_, message) => Ok(message),
            _ => unreachable!("filtered above"),
        }
    }

    pub(crate) fn no_pipeline_runs(&self) -> Result<()> {
        if self.recordings_after(FIRST_HOTKEY) != 0 || self.latest_generation().is_some() {
            bail!("a pipeline runs: {:?}", self.events());
        }
        Ok(())
    }

    /// Transcribes a recording through a pipeline of the same stand-ins, as file mode does.
    pub(crate) fn transcribe_file(&self) -> Result<()> {
        let (inputs, _) = std::sync::mpsc::channel();
        let mut pipeline = self.assembler().assemble(&self.config, inputs)?;
        pipeline.run_once(AudioBuffer16k::new(vec![
            1_000i16;
            AudioBuffer16k::SAMPLE_RATE_HZ
                as usize
        ]))
    }

    pub(crate) fn printed(&self) -> Result<()> {
        self.wait_for(|e| matches!(e, Event::Pasted(_)))?;
        Ok(())
    }

    fn assembler(&self) -> StandInAssembler {
        StandInAssembler {
            events: Arc::clone(&self.events),
            services_answer: Arc::clone(&self.services_answer),
            text_field_focused: Arc::clone(&self.text_field_focused),
            generation: 0,
        }
    }

    fn start_ready(&mut self) -> Result<()> {
        self.start();
        self.wait_for(|e| matches!(e, Event::State(RuntimeState::Ready { .. })))?;
        Ok(())
    }

    fn runtime(&self) -> Result<&Runtime> {
        self.runtime.as_ref().context("the runtime was not started")
    }

    fn reload(&self, config: Config) -> Result<()> {
        self.runtime()?.reload(config)
    }

    fn wait_for(&self, wanted: impl Fn(&Event) -> bool) -> Result<Event> {
        self.wait_for_nth(1, wanted)
    }

    fn wait_for_nth(&self, nth: usize, wanted: impl Fn(&Event) -> bool) -> Result<Event> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(event) = self.events().into_iter().filter(&wanted).nth(nth - 1) {
                return Ok(event);
            }
            if Instant::now() >= deadline {
                bail!("not seen within {REACTION:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn position(&self, wanted: impl Fn(&Event) -> bool) -> Result<usize> {
        self.events()
            .iter()
            .position(wanted)
            .with_context(|| format!("not in the log: {:?}", self.events()))
    }

    fn count(&self, wanted: impl Fn(&Event) -> bool) -> usize {
        self.events().iter().filter(|e| wanted(e)).count()
    }

    fn last_state(&self) -> Option<RuntimeState> {
        self.events().into_iter().rev().find_map(|e| match e {
            Event::State(state) => Some(state),
            _ => None,
        })
    }

    pub(crate) fn state_becomes(&self, wanted: impl Fn(&RuntimeState) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        loop {
            if self.last_state().as_ref().is_some_and(&wanted) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "runtime state is {:?}; events: {:?}",
                    self.last_state(),
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Starts the runtime from `config` and waits until it is Ready.
    pub(crate) fn start_from(&mut self, config: Config) -> Result<()> {
        self.config = config;
        self.start_ready()
    }

    pub(crate) fn reloader(&self) -> Result<Reloader> {
        self.runtime()?.reloader()
    }

    /// Whether a press of `key` starts a recording on the running pipeline.
    pub(crate) fn hotkey_records(&self, key: HotKey) -> bool {
        self.recordings_after(key) == 1
    }

    /// Ends the recording `key` started and waits until it was pasted.
    pub(crate) fn finish_dictation(&self, key: HotKey) -> Result<()> {
        let before = self.count(|e| matches!(e, Event::Pasted(_)));
        self.keyboard.press(key);
        self.wait_for_nth(before + 1, |e| matches!(e, Event::Pasted(_)))?;
        Ok(())
    }

    /// The start sound and the refinement switch of the newest pipeline.
    pub(crate) fn newest_setup(&self) -> Option<(String, bool)> {
        self.events().into_iter().rev().find_map(|e| match e {
            Event::Configured(_, sound, refines) => Some((sound, refines)),
            _ => None,
        })
    }

    /// From now on every speech recognition service the runtime probes is silent.
    pub(crate) fn services_stop_answering(&self) {
        *self.services_answer.lock().unwrap() = false;
    }

    fn latest_generation(&self) -> Option<Generation> {
        self.events().into_iter().rev().find_map(|e| match e {
            Event::Assembled(generation, _) => Some(generation),
            _ => None,
        })
    }

    /// Starts a recording on the running pipeline and waits until it records.
    fn start_recording(&self) -> Result<Generation> {
        let before = self.count(|e| matches!(e, Event::RecordingStarted(_)));
        self.keyboard.press(self.config.hotkey.key);
        self.wait_for_nth(before + 1, |e| matches!(e, Event::RecordingStarted(_)))
            .map(|e| match e {
                Event::RecordingStarted(generation) => generation,
                _ => unreachable!("filtered above"),
            })
    }

    fn recordings_after(&self, key: HotKey) -> usize {
        let before = self.count(|e| matches!(e, Event::RecordingStarted(_)));
        self.keyboard.press(key);
        std::thread::sleep(SETTLE);
        self.count(|e| matches!(e, Event::RecordingStarted(_))) - before
    }
}

pub(crate) fn run(world: &mut IntegrationWorld) -> &mut RuntimeRun {
    world.runtime.get_or_insert_with(RuntimeRun::default)
}

#[given("a valid configuration with voice control off")]
#[given("a valid configuration")]
fn valid_configuration(world: &mut IntegrationWorld) {
    let run = run(world);
    run.config = config_with(FIRST_HOTKEY);
    run.config.voice_control.enabled = false;
}

#[given("the speech recognition service and the text refinement service answer")]
fn services_answer(world: &mut IntegrationWorld) {
    *run(world).services_answer.lock().unwrap() = true;
}

#[given("the speech recognition service does not answer")]
fn service_silent(world: &mut IntegrationWorld) {
    *run(world).services_answer.lock().unwrap() = false;
}

#[given("the runtime is Ready with the first hotkey")]
#[given("the runtime is Ready")]
fn runtime_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_ready()
}

#[given("the runtime is Ready with voice control on")]
fn runtime_ready_with_voice_control(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = true;
    run.start_ready()?;
    run.wait_for(|e| matches!(e, Event::ListeningOn(_)))?;
    Ok(())
}

#[given("a recording is in progress")]
fn recording_in_progress(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.runtime.is_none() {
        run.start_ready()?;
    }
    run.start_recording()?;
    Ok(())
}

#[given("a dictation waits for a text input to gain focus")]
fn dictation_waits_for_focus(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    *run.text_field_focused.lock().unwrap() = false;
    run.start_recording()?;
    run.keyboard.press(run.config.hotkey.key);
    std::thread::sleep(SETTLE);
    if run.count(|e| matches!(e, Event::Pasted(_))) > 0 {
        bail!("the dictation was pasted without focus: {:?}", run.events());
    }
    Ok(())
}

#[when("the runtime starts")]
fn runtime_starts(world: &mut IntegrationWorld) {
    run(world).start();
}

#[when("the runtime reloads a valid configuration with a second hotkey")]
#[when("the runtime reloads a valid configuration")]
fn reload_second_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.reload(config_with(SECOND_HOTKEY))
}

#[when("the runtime reloads two valid configurations one right after the other")]
fn reload_twice(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.reload(config_with(SECOND_HOTKEY))?;
    run.reload(config_with(THIRD_HOTKEY))
}

#[when("the runtime reloads a configuration that does not validate")]
fn reload_invalid(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let mut config = config_with(SECOND_HOTKEY);
    config.transcription.base_url = REMOTE_SERVICE.to_owned();
    let log = crate::steps::voice_control::pipeline_log();
    let mark = log.warnings.lock().unwrap().len();
    let here = std::thread::current().id();
    if run.reload(config).is_ok() {
        bail!("the runtime accepted a configuration that does not validate");
    }
    run.rejection_log = log.warnings.lock().unwrap()[mark..]
        .iter()
        .filter(|(thread, _)| *thread == here)
        .map(|(_, line)| line.clone())
        .collect();
    Ok(())
}

#[when("the runtime stops")]
fn runtime_stops(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.runtime
        .as_mut()
        .context("the runtime was not started")?
        .stop();
    Ok(())
}

#[then("the dictation pipeline runs with that configuration")]
fn pipeline_runs_with_configuration(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.started
        .take()
        .context("the runtime was not started")??;
    run.wait_for(|e| *e == Event::Assembled(1, FIRST_HOTKEY))?;
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!(
            "the configured hotkey started no recording: {:?}",
            run.events()
        );
    }
    Ok(())
}

#[then("the runtime state is Ready with voice control off")]
fn ready_voice_control_off(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| {
        *s == RuntimeState::Ready {
            hotkey: FIRST_HOTKEY,
            recording_mode: RecordingMode::Toggle,
            voice_control: false,
        }
    })
}

#[then("the runtime state is Failed with a reason naming the speech recognition service")]
fn failed_naming_service(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    match run.started.take().context("the runtime was not started")? {
        Ok(()) => bail!("the runtime started without the speech recognition service"),
        Err(e) if format!("{e:#}").contains("speech recognition service") => {}
        Err(e) => bail!("the start failed for another reason: {e:#}"),
    }
    run.state_becomes(
        |s| matches!(s, RuntimeState::Failed(reason) if reason.contains("speech recognition service")),
    )
}

#[then("no dictation pipeline runs")]
fn no_pipeline(world: &mut IntegrationWorld) -> Result<()> {
    run(world).no_pipeline_runs()
}

#[then("the runtime state is Ready again")]
fn ready_again(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(
        |s| matches!(s, RuntimeState::Ready { hotkey, .. } if *hotkey == SECOND_HOTKEY),
    )
}

#[then("a press of the second hotkey starts a recording")]
fn second_hotkey_records(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.recordings_after(SECOND_HOTKEY) != 1 {
        bail!("the second hotkey started no recording: {:?}", run.events());
    }
    // stop it, so the next press starts no recording only because of its key
    run.keyboard.press(SECOND_HOTKEY);
    run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    Ok(())
}

#[then("a press of the first hotkey starts no recording")]
fn first_hotkey_silent(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.recordings_after(FIRST_HOTKEY) != 0 {
        bail!("the first hotkey still records: {:?}", run.events());
    }
    Ok(())
}

#[then("the process is the same one")]
fn same_process(world: &mut IntegrationWorld) -> Result<()> {
    if run(world).process != std::process::id() {
        bail!("the process changed");
    }
    Ok(())
}

#[then("the pipeline runs with the second configuration")]
fn runs_with_second(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(
        |s| matches!(s, RuntimeState::Ready { hotkey, .. } if *hotkey == THIRD_HOTKEY),
    )?;
    let generation = run.latest_generation().context("no pipeline assembled")?;
    if run
        .position(|e| *e == Event::Assembled(generation, THIRD_HOTKEY))
        .is_err()
    {
        bail!(
            "the last pipeline is not the second configuration's: {:?}",
            run.events()
        );
    }
    if run.recordings_after(THIRD_HOTKEY) != 1 {
        bail!(
            "the second configuration's hotkey started no recording: {:?}",
            run.events()
        );
    }
    Ok(())
}

#[then("the runtime state is Ready")]
#[then("the runtime state stays Ready")]
fn state_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))
}

#[then("the runtime state is Restarting")]
fn state_restarting(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| *s == RuntimeState::Restarting)
}

#[then("the recording keeps running until the hotkey stops it")]
fn recording_keeps_running(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let ended = run.count(|e| matches!(e, Event::CancelPlayed(1) | Event::PipelineDropped(1)));
    if ended > 0 || run.last_state() != Some(RuntimeState::Restarting) {
        bail!(
            "the restart did not wait for the recording: {:?}",
            run.events()
        );
    }
    run.keyboard.press(FIRST_HOTKEY);
    Ok(())
}

#[then("the dictation is pasted")]
fn dictation_pasted(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::Pasted(1))?;
    Ok(())
}

#[then("only then the pipeline restarts and the runtime state is Ready")]
fn restarts_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    restarted_after_the_dictation(run(world))
}

#[then("the pipeline restarts only after that dictation was pasted or given up")]
fn restarts_after_focus(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(|e| matches!(e, Event::PipelineDropped(1))) > 0 {
        bail!(
            "the pipeline restarted while the dictation waited: {:?}",
            run.events()
        );
    }
    *run.text_field_focused.lock().unwrap() = true;
    run.wait_for(|e| *e == Event::Pasted(1))?;
    restarted_after_the_dictation(run)
}

/// Checks the order Restarting, the old pipeline's paste, its end, Ready.
pub(crate) fn restarted_after_the_dictation(run: &RuntimeRun) -> Result<()> {
    run.wait_for(|e| *e == Event::PipelineDropped(1))?;
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    let pasted = run.position(|e| *e == Event::Pasted(1))?;
    let dropped = run.position(|e| *e == Event::PipelineDropped(1))?;
    let restarting = run.position(|e| *e == Event::State(RuntimeState::Restarting))?;
    let ready = run.events().len()
        - 1
        - run
            .events()
            .iter()
            .rev()
            .position(|e| matches!(e, Event::State(RuntimeState::Ready { .. })))
            .unwrap();
    if !(restarting < pasted && pasted < dropped && dropped < ready) {
        bail!(
            "expected Restarting, the paste, the old pipeline's end, Ready: {:?}",
            run.events()
        );
    }
    Ok(())
}

#[then("a press of the first hotkey still starts a recording")]
fn first_hotkey_still_records(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!("the first hotkey no longer records: {:?}", run.events());
    }
    if run.latest_generation() != Some(1) {
        bail!("the pipeline was replaced: {:?}", run.events());
    }
    Ok(())
}

#[then("the log names the reason the configuration was rejected")]
fn log_names_reason(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let named = run.rejection_log.iter().any(|line| {
        line.contains("configuration rejected") && line.contains("transcription.base_url")
    });
    if !named {
        bail!(
            "no rejection with its reason in the log: {:?}",
            run.rejection_log
        );
    }
    Ok(())
}

#[then("the pipeline ends")]
fn pipeline_ends(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::PipelineDropped(1))?;
    Ok(())
}

#[then("voice control listening stops")]
fn listening_stops(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::ListeningOff(1))?;
    Ok(())
}

#[then("the runtime state is Stopped")]
fn state_stopped(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| *s == RuntimeState::Stopped)
}

#[then("the recording is cancelled and nothing is pasted")]
fn recording_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    run(world).cancelled_and_nothing_pasted()
}

#[given("an observer registered before the runtime starts")]
fn observer_registered(world: &mut IntegrationWorld) {
    // every run registers its recording observer before it starts the runtime
    run(world);
}

#[given("Input Monitoring is not granted")]
fn input_monitoring_not_granted(world: &mut IntegrationWorld) {
    run(world)
        .permissions
        .set(Permission::InputMonitoring, PermissionStatus::Denied);
}

#[then("the observer is told Waiting for permissions naming Input Monitoring")]
fn told_waiting_input_monitoring(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for(&[Permission::InputMonitoring], &[])
}

#[given("the runtime is Waiting for permissions")]
fn runtime_waiting(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.permissions
        .set(Permission::Accessibility, PermissionStatus::Denied);
    run.start_waiting()
}

#[when("the last missing permission is granted")]
fn last_missing_granted(world: &mut IntegrationWorld) {
    run(world)
        .permissions
        .set(Permission::Accessibility, PermissionStatus::Granted);
}

#[then("the observer is told Starting and then Ready")]
fn told_starting_then_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    let waiting =
        run.position(|e| matches!(e, Event::State(RuntimeState::WaitingForPermissions { .. })))?;
    let states: Vec<_> = run.events()[waiting + 1..]
        .iter()
        .filter_map(|e| match e {
            Event::State(state) => Some(state.clone()),
            _ => None,
        })
        .collect();
    if !matches!(
        states.as_slice(),
        [RuntimeState::Starting, RuntimeState::Ready { .. }]
    ) {
        bail!("expected Starting and then Ready, got {states:?}");
    }
    Ok(())
}

#[then("the dictation pipeline runs")]
fn pipeline_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.assembled(1)?;
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!("the hotkey started no recording: {:?}", run.events());
    }
    Ok(())
}

#[when("Accessibility is revoked")]
fn accessibility_revoked(world: &mut IntegrationWorld) {
    run(world)
        .permissions
        .set(Permission::Accessibility, PermissionStatus::Denied);
}

#[then("the observer is told Waiting for permissions naming Accessibility")]
fn told_waiting_accessibility(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for(&[Permission::Accessibility], &[Permission::Accessibility])
}

/// Builds every pipeline from stand-ins that log under the pipeline's generation.
struct StandInAssembler {
    events: Events,
    services_answer: Arc<Mutex<bool>>,
    text_field_focused: Arc<Mutex<bool>>,
    generation: Generation,
}

impl PipelineAssembler for StandInAssembler {
    fn assemble(&mut self, config: &Config, _inputs: Sender<PipelineInput>) -> Result<Pipeline> {
        if !*self.services_answer.lock().unwrap() {
            bail!("{SILENT_SERVICE_ERROR}");
        }
        self.generation += 1;
        let generation = self.generation;
        push(
            &self.events,
            Event::Assembled(generation, config.hotkey.key),
        );
        push(
            &self.events,
            Event::Configured(
                generation,
                config.sounds.start_sound.clone(),
                config.refinement.enabled,
            ),
        );
        let part = || Part {
            events: Arc::clone(&self.events),
            generation,
        };
        let mut pipeline = Pipeline::new(
            Box::new(StandInRecorder(part())),
            Box::new(FixedTranscript),
            Box::new(PassthroughTextRefiner),
            Box::new(StandInPaster(part())),
            Box::new(StandInSounds(part())),
            Box::new(NoticeOverlay(part())),
            PipelineConfig {
                min_press_ms: config.hotkey.min_press_ms,
                recording_mode: config.hotkey.recording_mode,
                chunking: ChunkingCfg {
                    chunk_threshold_secs: 0,
                    ..ChunkingCfg::default()
                },
            },
        )
        .with_focus_wait(
            Box::new(ScriptedFocus(Arc::clone(&self.text_field_focused))),
            FocusWaitCfg {
                timeout: Duration::from_secs(config.paste.focus_wait_seconds),
                poll_interval: FOCUS_POLL,
            },
        );
        if config.voice_control.enabled {
            pipeline =
                pipeline.with_voice_control(Box::new(StandInSpotter(part())), Box::new(NoKeys));
        }
        Ok(pipeline)
    }
}

/// A pipeline part: what it is asked goes to the log under its pipeline's generation.
struct Part {
    events: Events,
    generation: Generation,
}

impl Part {
    fn log(&self, event: fn(Generation) -> Event) {
        push(&self.events, event(self.generation));
    }
}

struct StandInRecorder(Part);

impl AudioRecorder for StandInRecorder {
    fn start(&mut self) -> Result<()> {
        self.0.log(Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(AudioBuffer16k::new(vec![
            1_000i16;
            AudioBuffer16k::SAMPLE_RATE_HZ
                as usize
        ]))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}
}

/// The recorder lives as long as its pipeline: its drop is the pipeline's end.
impl Drop for StandInRecorder {
    fn drop(&mut self) {
        self.0.log(Event::PipelineDropped);
    }
}

struct FixedTranscript;

impl Transcriber for FixedTranscript {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(DICTATED.to_owned())?))
    }
}

struct StandInPaster(Part);

impl TextPaster for StandInPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        self.0.log(Event::Pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }
}

struct StandInSounds(Part);

impl SoundPlayer for StandInSounds {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {
        self.0.log(Event::CancelPlayed);
    }
}

/// Shows nothing; a failure notice goes to the log.
struct NoticeOverlay(Part);

impl VisualIndicator for NoticeOverlay {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, notice: &FailureNotice) {
        push(
            &self.0.events,
            Event::Notice(self.0.generation, notice.message()),
        );
    }
}

struct ScriptedFocus(Arc<Mutex<bool>>);

impl FocusProbe for ScriptedFocus {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        let text_field = *self.0.lock().unwrap();
        Ok(Some(FocusedElement {
            role: if text_field {
                "AXTextField"
            } else {
                "AXButton"
            }
            .to_owned(),
            has_text_selection: text_field,
            value_settable: text_field,
            in_web_content: false,
        }))
    }
}

struct StandInSpotter(Part);

impl CommandSpotter for StandInSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        self.0.log(Event::ListeningOn);
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {
        self.0.log(Event::ListeningOff);
    }
}

struct NoKeys;

impl KeyPresser for NoKeys {
    fn press_enter(&mut self) -> Result<()> {
        Ok(())
    }
    fn clear_all_text(&mut self) -> Result<()> {
        Ok(())
    }
    fn paste(&mut self) -> Result<()> {
        Ok(())
    }
}

struct RecordingObserver(Events);

impl RuntimeObserver for RecordingObserver {
    fn state_changed(&self, state: &RuntimeState) {
        push(&self.0, Event::State(state.clone()));
    }
}

/// The keyboard the runtime's listener reads: a press reaches the runtime only when the key
/// is the one the binding holds at that moment, as the real listener matches every event.
#[derive(Clone, Default)]
struct FakeKeyboard {
    wired: Arc<Mutex<Option<Wiring>>>,
}

/// The binding the runtime handed the listener, and where its signals go.
type Wiring = (HotkeyBinding, Sender<PttSignal>);

impl FakeKeyboard {
    fn press(&self, key: HotKey) {
        if let Some((binding, tx)) = self.wired.lock().unwrap().as_ref() {
            if binding.current() == key {
                let _ = tx.send(PttSignal::Pressed);
                let _ = tx.send(PttSignal::Released);
            }
        }
    }
}

impl HotkeyListener for FakeKeyboard {
    fn spawn(
        self: Box<Self>,
        binding: HotkeyBinding,
        tx: Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        *self.wired.lock().unwrap() = Some((binding, tx));
        Ok(std::thread::spawn(|| {}))
    }
}

type Statuses = Arc<Mutex<[PermissionStatus; 3]>>;

/// The permissions as System Settings holds them, which the scenario changes, and how often
/// the runtime asked.
#[derive(Clone)]
pub(crate) struct Permissions {
    statuses: Statuses,
    checks: Arc<AtomicUsize>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            statuses: Arc::new(Mutex::new([PermissionStatus::Granted; 3])),
            checks: Arc::default(),
        }
    }
}

impl Permissions {
    pub(crate) fn set(&self, permission: Permission, status: PermissionStatus) {
        self.statuses.lock().unwrap()[index(permission)] = status;
    }

    pub(crate) fn checks(&self) -> usize {
        self.checks.load(Ordering::SeqCst)
    }

    /// A gate over the scenario's permissions, as the window asks it.
    pub(crate) fn gate(&self) -> Arc<PermissionsGate> {
        Arc::new(PermissionsGate::new(Box::new(ScriptedProbe(self.clone()))))
    }
}

fn index(permission: Permission) -> usize {
    Permission::ALL
        .iter()
        .position(|p| *p == permission)
        .expect("every permission is in ALL")
}

struct ScriptedProbe(Permissions);

impl PermissionProbe for ScriptedProbe {
    fn status(&self, permission: Permission) -> PermissionStatus {
        self.0.checks.fetch_add(1, Ordering::SeqCst);
        self.0.statuses.lock().unwrap()[index(permission)]
    }

    fn request(&self, _permission: Permission) {}

    fn settings_url(&self, permission: Permission) -> String {
        format!("settings:{permission}")
    }

    fn responsible_app(&self) -> Option<String> {
        None
    }
}
