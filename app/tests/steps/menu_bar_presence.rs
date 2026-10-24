//! Drives the real runtime with pipelines of stand-ins whose overlay goes through the status
//! item's decorator; the runtime tells the status item's observer its states, and a status
//! item double records every icon it is asked to draw.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::primary::menubar::macos::actions::{
    MacosMenuActions, MenuActions, SettingsOpener,
};
use voicepuncher::adapters::primary::menubar::macos::indicator::{
    MenuModel, StatusItemCell, StatusItemSurface,
};
use voicepuncher::adapters::primary::menubar::macos::mapping::IconState;
use voicepuncher::config::Config;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineParts};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, FocusProbe, FocusWaitCfg, HotkeyListener, PipelineInput,
    SoundPlayer, TextPaster, Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, CommandPlacement, FailureNotice, FocusedElement, HotKey,
    HotkeyBinding, PttSignal, RawTranscript, RecordingMode, SpottingEvent, VoiceCommand,
};
use voicepuncher::runtime::{
    PipelineAssembler, Runtime, RuntimeObserver, RuntimeState, SharedVisualIndicator,
};

use crate::steps::pipeline_config::pipeline_config;
use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::IntegrationWorld;

mod doubles;
mod services;
use doubles::{FakeKeyboard, QuietOverlay, StandInAssembler};

const DICTATED: &str = "Meeting moved to Friday.";
const SILENT_SERVICE_ERROR: &str =
    "speech recognition service is not reachable at http://127.0.0.1:8081";
/// How long the status item shows the warning of a failure notice in these scenarios.
const NOTICE_LIFETIME: Duration = Duration::from_millis(400);
/// How early the warning may go, as the timer and the recording of icons may differ.
const NOTICE_SLACK: Duration = Duration::from_millis(50);
const FOCUS_POLL: Duration = Duration::from_millis(10);
const FOCUS_WAIT_SECONDS: u64 = 5;
/// Longest a runtime or pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(5);

/// What a quit does, in the order it does it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum QuitEvent {
    RuntimeStopped,
    ProcessEnded(i32),
}

type Icons = Arc<Mutex<Vec<(Instant, IconState)>>>;
type Flag = Arc<Mutex<bool>>;
/// A start held while it assembles: the gate that lets it go on, and the thread it runs on.
type HeldStart = (Sender<bool>, JoinHandle<(Runtime, Result<()>)>);

/// Records every icon the status item draws, with the moment it was drawn.
struct SurfaceDouble(Icons);

impl StatusItemSurface for SurfaceDouble {
    fn show_icon(&self, icon: IconState) {
        self.0.lock().unwrap().push((Instant::now(), icon));
    }
}

/// What a menu-bar-presence scenario shares with the runtime, its pipelines and the status item.
pub struct MenuBarRun {
    icons: Icons,
    cell: StatusItemCell,
    config: Config,
    parts: Parts,
    keyboard: FakeKeyboard,
    runtime: Option<Runtime>,
    starting: Option<HeldStart>,
    started: Option<Result<()>>,
    menu: Option<MenuModel>,
    /// What happened when the user chose Quit, in order.
    quit_log: Arc<Mutex<Vec<QuitEvent>>>,
    /// The menu's actions once the user chose one; they own the runtime.
    actions: Option<MacosMenuActions>,
    settings_opens: Arc<Mutex<usize>>,
    /// How many icons were drawn before the scenario's action.
    drawn_before: usize,
    /// The services the menu's actions show.
    services_view: Option<Arc<services::ViewDouble>>,
    /// The Services submenu as the user opened it.
    service_menu: Option<services::ServiceMenu>,
}

/// What the stand-ins read and the scenario sets.
#[derive(Clone, Default)]
struct Parts {
    transcription_fails: Flag,
    transcription_held: Flag,
    text_field_focused: Flag,
    services_answer: Flag,
    cancel_played: Flag,
    pasted: Flag,
    inputs: Arc<Mutex<Option<Sender<PipelineInput>>>>,
}

impl std::fmt::Debug for MenuBarRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuBarRun")
            .field("icons", &self.icon_history())
            .finish()
    }
}

impl Default for MenuBarRun {
    fn default() -> Self {
        let icons = Icons::default();
        let cell =
            StatusItemCell::new(Box::new(SurfaceDouble(Arc::clone(&icons))), NOTICE_LIFETIME);
        let parts = Parts::default();
        *parts.text_field_focused.lock().unwrap() = true;
        *parts.services_answer.lock().unwrap() = true;
        Self {
            icons,
            cell,
            config: config_with(HotKey::RightOption, RecordingMode::Toggle),
            parts,
            keyboard: FakeKeyboard::default(),
            runtime: None,
            starting: None,
            started: None,
            menu: None,
            quit_log: Arc::default(),
            actions: None,
            settings_opens: Arc::default(),
            drawn_before: 0,
            services_view: None,
            service_menu: None,
        }
    }
}

impl Drop for MenuBarRun {
    fn drop(&mut self) {
        *self.parts.transcription_held.lock().unwrap() = false;
        if let Some((gate, starting)) = self.starting.take() {
            let _ = gate.send(true);
            if let Ok((runtime, _)) = starting.join() {
                self.runtime = Some(runtime);
            }
        }
        if let Some(mut runtime) = self.runtime.take() {
            runtime.stop();
        }
    }
}

fn config_with(hotkey: HotKey, mode: RecordingMode) -> Config {
    let mut config = Config::default();
    config.hotkey.key = hotkey;
    config.hotkey.min_press_ms = 0;
    config.hotkey.recording_mode = mode;
    config.paste.focus_wait_seconds = FOCUS_WAIT_SECONDS;
    config
}

impl MenuBarRun {
    fn runtime_parts(&self, gate: Option<Receiver<bool>>) -> Runtime {
        let overlay = SharedVisualIndicator::new(Box::new(self.cell.indicator(QuietOverlay)));
        let assembler = StandInAssembler { parts: self.parts.clone(), overlay, gate };
        let runtime = Runtime::new(Box::new(assembler), Box::new(self.keyboard.clone()));
        runtime.observe(Box::new(self.cell.observer()));
        runtime.observe(Box::new(QuitObserver(Arc::clone(&self.quit_log))));
        runtime
    }

    fn start(&mut self) {
        let mut runtime = self.runtime_parts(None);
        self.started = Some(runtime.start(self.config.clone()));
        self.runtime = Some(runtime);
    }

    fn start_ready(&mut self) -> Result<()> {
        self.start();
        self.started_ok()?;
        self.icon_becomes(|icon| matches!(icon, IconState::Ready | IconState::Listening))
    }

    fn started_ok(&self) -> Result<()> {
        match &self.started {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => bail!("the runtime did not start: {e:#}"),
            None => bail!("the runtime was not started"),
        }
    }

    /// Starts the runtime on a thread of its own, held while it assembles its pipeline.
    fn start_held(&mut self) {
        let (gate_tx, gate_rx) = mpsc::channel();
        let mut runtime = self.runtime_parts(Some(gate_rx));
        let config = self.config.clone();
        let starting = std::thread::spawn(move || {
            let started = runtime.start(config);
            (runtime, started)
        });
        self.starting = Some((gate_tx, starting));
    }

    /// Lets the held start go on: the services answer, or they do not.
    fn finish_start(&mut self, services_answer: bool) -> Result<()> {
        let (gate, starting) = self.starting.take().context("no start is held")?;
        gate.send(services_answer)?;
        let (runtime, started) = starting
            .join()
            .map_err(|_| anyhow::anyhow!("the start panicked"))?;
        self.runtime = Some(runtime);
        self.started = Some(started);
        Ok(())
    }

    fn runtime(&self) -> Result<&Runtime> {
        self.runtime.as_ref().context("the runtime was not started")
    }

    fn icon_history(&self) -> Vec<(Instant, IconState)> {
        self.icons.lock().unwrap().clone()
    }

    fn icons_since(&self, mark: usize) -> Vec<IconState> {
        self.icon_history()[mark..]
            .iter()
            .map(|(_, icon)| *icon)
            .collect()
    }

    fn mark(&self) -> usize {
        self.icons.lock().unwrap().len()
    }

    fn current_icon(&self) -> Option<IconState> {
        self.icon_history().last().map(|(_, icon)| *icon)
    }

    fn icon_becomes(&self, wanted: impl Fn(IconState) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        loop {
            if self.current_icon().is_some_and(&wanted) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "the icon is {:?}; drawn: {:?}",
                    self.current_icon(),
                    self.icon_history()
                );
            }
            std::thread::sleep(POLL);
        }
    }

    /// Waits until the icons drawn since the scenario's action are `expected`.
    fn drawn_since_becomes(&self, expected: &[IconState]) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        loop {
            let drawn = self.icons_since(self.drawn_before);
            if drawn == expected {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!("drawn since the action: {drawn:?}");
            }
            std::thread::sleep(POLL);
        }
    }

    fn icon_is(&self, wanted: IconState) -> Result<()> {
        self.icon_becomes(|icon| icon == wanted)
    }

    fn send(&self, input: PipelineInput) -> Result<()> {
        self.parts
            .inputs
            .lock()
            .unwrap()
            .as_ref()
            .context("no pipeline runs")?
            .send(input)?;
        Ok(())
    }

    /// Presses the hotkey: a recording starts in Toggle mode, or stops when one runs.
    fn press(&self) {
        self.keyboard.signal(PttSignal::Pressed);
        self.keyboard.signal(PttSignal::Released);
    }

    fn start_recording(&self) -> Result<()> {
        match self.config.hotkey.recording_mode {
            RecordingMode::Toggle => self.press(),
            RecordingMode::Hold => self.keyboard.signal(PttSignal::Pressed),
        }
        self.icon_is(IconState::Recording)
    }

    fn open_menu(&mut self) {
        self.menu = Some(self.cell.menu());
    }

    fn status_line(&self) -> Result<&str> {
        Ok(&self
            .menu
            .as_ref()
            .context("the menu was not opened")?
            .status_line)
    }
}

/// Notes when the runtime stops, so a quit shows it stopped before the process ended.
struct QuitObserver(Arc<Mutex<Vec<QuitEvent>>>);

impl RuntimeObserver for QuitObserver {
    fn state_changed(&self, state: &RuntimeState) {
        if *state == RuntimeState::Stopped {
            self.0.lock().unwrap().push(QuitEvent::RuntimeStopped);
        }
    }
}

fn run(world: &mut IntegrationWorld) -> &mut MenuBarRun {
    world.menu_bar.get_or_insert_with(MenuBarRun::default)
}

fn set(flag: &Flag, value: bool) {
    *flag.lock().unwrap() = value;
}

fn recording_mode(name: &str) -> Result<RecordingMode> {
    match name {
        "hold" => Ok(RecordingMode::Hold),
        "toggle" => Ok(RecordingMode::Toggle),
        other => bail!("no recording mode {other:?}"),
    }
}

#[given("the app was just launched and its runtime is still starting")]
#[given("the status item shows the waiting icon while the runtime starts")]
#[given("the runtime is starting")]
fn runtime_starting(world: &mut IntegrationWorld) {
    run(world).start_held();
}

#[given("the runtime is ready and no dictation runs")]
fn runtime_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_ready()
}

#[given("a button has keyboard focus while the runtime is ready")]
fn button_focused(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.text_field_focused, false);
    run.start_ready()
}

#[given("the speech recognition service is not running while a recording runs")]
fn speech_service_down(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.transcription_fails, true);
    run.start_ready()?;
    run.start_recording()
}

#[given("voice control is turned on and a recording runs")]
fn voice_control_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = true;
    run.start_ready()?;
    run.start_recording()
}

#[given("voice control is turned on and no dictation runs yet")]
fn voice_control_on(world: &mut IntegrationWorld) {
    run(world).config.voice_control.enabled = true;
}

#[given("voice control is turned on, the runtime is ready and no dictation runs")]
fn voice_control_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = true;
    run.start_ready()
}

#[given("the commands-unavailable sign shows")]
fn commands_unavailable_shows(world: &mut IntegrationWorld) -> Result<()> {
    voice_control_ready(world)?;
    commands_away(world)?;
    run(world).icon_is(IconState::CommandsUnavailable)
}

#[given(expr = "the runtime is ready with the hotkey {string} in the recording mode {string}")]
fn ready_with_hotkey(world: &mut IntegrationWorld, key: String, mode: String) -> Result<()> {
    let run = run(world);
    run.config = config_with(key.parse()?, recording_mode(&mode)?);
    run.start_ready()
}

#[given(expr = "a recording runs with the hotkey {string} in the recording mode {string}")]
fn recording_with_hotkey(world: &mut IntegrationWorld, key: String, mode: String) -> Result<()> {
    ready_with_hotkey(world, key, mode)?;
    run(world).start_recording()
}

#[given("the runtime failed because the speech recognition service does not answer")]
fn runtime_failed(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.services_answer, false);
    run.start();
    run.icon_is(IconState::Failed)
}

#[given("the menu was opened and closed while the runtime was ready")]
fn menu_opened_while_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_ready()?;
    run.open_menu();
    Ok(())
}

#[when("the menu bar is looked at")]
fn menu_bar_looked_at(_world: &mut IntegrationWorld) {}

#[when("the runtime becomes ready")]
fn runtime_becomes_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.starting.is_some() {
        run.finish_start(true)
    } else {
        run.start();
        run.started_ok()
    }
}

#[when("the runtime restarts with a new configuration and becomes ready again")]
fn runtime_restarts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.drawn_before = run.mark();
    let config = config_with(HotKey::F19, RecordingMode::Toggle);
    run.runtime()?.reload(config)
}

#[when("the runtime fails because a local service does not answer")]
fn runtime_fails(world: &mut IntegrationWorld) -> Result<()> {
    run(world).finish_start(false)
}

#[when("the user presses the hotkey to start a recording")]
fn user_starts_recording(world: &mut IntegrationWorld) {
    run(world).press();
}

#[when("the user dictates a sentence, ends the dictation and then clicks into a text field")]
fn dictation_through_every_stage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.drawn_before = run.mark();
    run.start_recording()?;
    run.press();
    run.icon_is(IconState::WaitingForFocus)?;
    set(&run.parts.text_field_focused, true);
    Ok(())
}

#[when("the user stops the dictation with the hotkey")]
fn user_stops_dictation(world: &mut IntegrationWorld) {
    run(world).press();
}

#[when("the user clears the text by voice")]
fn user_clears_by_voice(world: &mut IntegrationWorld) -> Result<()> {
    let now = Instant::now();
    run(world).send(PipelineInput::Spotting(SpottingEvent::CommandsSaid {
        commands: vec![VoiceCommand::ClearAllText],
        start: now,
        end: now,
        placement: CommandPlacement::Alone,
        timing: None,
    }))
}

#[when("the command spotting service has been away long enough for the commands-unavailable sign")]
fn commands_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingStillAway)
}

#[when("the user opens the menu")]
fn user_opens_menu(world: &mut IntegrationWorld) {
    run(world).open_menu();
}

#[when("the user opens the menu again during speech recognition")]
fn menu_during_recognition(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.transcription_held, true);
    run.start_recording()?;
    run.press();
    run.icon_is(IconState::Processing)?;
    run.open_menu();
    Ok(())
}

#[when("the user chooses Quit in the menu")]
fn user_quits(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let runtime = run.runtime.take().context("the runtime was not started")?;
    let quit_log = Arc::clone(&run.quit_log);
    let end_process = move |code| quit_log.lock().unwrap().push(QuitEvent::ProcessEnded(code));
    let scratch = std::env::temp_dir();
    let actions = MacosMenuActions::ending_with(
        runtime,
        Box::new(SettingsOpens(Arc::default())),
        scratch.join("logs"),
        Box::new(end_process),
    );
    actions.quit();
    run.actions = Some(actions);
    Ok(())
}

/// Counts how often the menu opened the settings window.
struct SettingsOpens(Arc<Mutex<usize>>);

impl SettingsOpener for SettingsOpens {
    fn open(&self) {
        *self.0.lock().unwrap() += 1;
    }
}

#[when("the user chooses Settings in the menu")]
fn user_chooses_settings(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let runtime = run.runtime.take().context("the runtime was not started")?;
    let opens = Arc::clone(&run.settings_opens);
    let actions = MacosMenuActions::ending_with(
        runtime,
        Box::new(SettingsOpens(opens)),
        std::env::temp_dir().join("logs"),
        Box::new(|_| {}),
    );
    actions.open_settings();
    run.actions = Some(actions);
    Ok(())
}

#[then("the settings window opens in front")]
fn settings_window_opens(world: &mut IntegrationWorld) -> Result<()> {
    let opens = *run(world).settings_opens.lock().unwrap();
    if opens != 1 {
        bail!("the settings window was opened {opens} times");
    }
    Ok(())
}

#[then("the runtime stops and the process ends with exit code 0")]
fn process_ends(world: &mut IntegrationWorld) -> Result<()> {
    let log = run(world).quit_log.lock().unwrap().clone();
    if log != [QuitEvent::RuntimeStopped, QuitEvent::ProcessEnded(0)] {
        bail!("the quit did: {log:?}");
    }
    Ok(())
}

#[then("the recording is cancelled with the cancel sound and nothing is pasted")]
fn recording_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if !*run.parts.cancel_played.lock().unwrap() {
        bail!("no cancel sound played");
    }
    if *run.parts.pasted.lock().unwrap() {
        bail!("text was pasted");
    }
    Ok(())
}

#[then("the status item shows the waiting icon")]
fn shows_waiting(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Waiting)
}

#[then("the status item shows the ready icon")]
fn shows_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Ready)
}

#[then("the status item shows the waiting icon during the restart and the ready icon afterwards")]
fn waiting_during_restart(world: &mut IntegrationWorld) -> Result<()> {
    run(world).drawn_since_becomes(&[IconState::Waiting, IconState::Ready])
}

#[then("the status item shows the failed icon and the start reports the failure the app exits on")]
fn shows_failed(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.icon_is(IconState::Failed)?;
    match &run.started {
        Some(Err(_)) => Ok(()),
        other => bail!("the start did not fail: {other:?}"),
    }
}

#[then("the status item shows the recording icon")]
fn shows_recording(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Recording)
}

#[then(
    "the status item shows the recording icon, then the waveform, then the keyboard, then the ready icon after the paste"
)]
fn shows_every_stage(world: &mut IntegrationWorld) -> Result<()> {
    run(world).drawn_since_becomes(&[
        IconState::Recording,
        IconState::Processing,
        IconState::WaitingForFocus,
        IconState::Ready,
    ])
}

#[then(
    "the status item shows the warning icon while the failure notice shows and the ready icon once it hides"
)]
fn warning_while_notice(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.icon_is(IconState::Warning)?;
    run.icon_is(IconState::Ready)?;
    let history = run.icon_history();
    let warned = history
        .iter()
        .rposition(|(_, icon)| *icon == IconState::Warning)
        .context("no warning drawn")?;
    let shown_for = history[warned + 1].0 - history[warned].0;
    if shown_for + NOTICE_SLACK < NOTICE_LIFETIME {
        bail!("the warning showed only {shown_for:?}");
    }
    Ok(())
}

#[then("the status item keeps showing the recording icon")]
fn keeps_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    match run.current_icon() {
        Some(IconState::Recording) => Ok(()),
        other => bail!("the icon is {other:?}; drawn: {:?}", run.icon_history()),
    }
}

#[then("the status item shows the listening icon")]
fn shows_listening(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::Listening)
}

#[then("the status item shows the ear with a warning badge until the sign goes")]
fn ear_with_badge(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.icon_is(IconState::CommandsUnavailable)?;
    run.send(PipelineInput::SpottingBack)?;
    run.icon_is(IconState::Listening)
}

#[then("the menu's first line names the app and its version")]
fn first_line(world: &mut IntegrationWorld) -> Result<()> {
    let menu = run(world).menu.clone().context("the menu was not opened")?;
    let expected = format!("voicepuncher {}", env!("CARGO_PKG_VERSION"));
    if menu.title_line != expected {
        bail!("the first line reads {:?}", menu.title_line);
    }
    Ok(())
}

#[then(expr = "the status line reads {string}")]
fn status_line_reads(world: &mut IntegrationWorld, expected: String) -> Result<()> {
    let line = run(world).status_line()?;
    if line != expected {
        bail!("the status line reads {line:?}");
    }
    Ok(())
}

#[then(expr = "the status line starts with {string} and names the speech recognition service")]
fn status_line_failed(world: &mut IntegrationWorld, prefix: String) -> Result<()> {
    let line = run(world).status_line()?;
    if !line.starts_with(&prefix) || !line.contains("speech recognition service") {
        bail!("the status line reads {line:?}");
    }
    Ok(())
}
