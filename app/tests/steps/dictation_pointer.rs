//! Drives the real pipeline on its own thread through the adapter that shows the dictation
//! sign next to the pointer, and watches in one ordered log what the overlay and the
//! dictation sign were asked to show.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    DictationSignState, PointerSignIndicator, PointerSignView, SignLifetimes,
};
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, CommandSpotter, FocusProbe, FocusWaitCfg, KeyPresser,
    PipelineInput, SoundPlayer, TextPaster, TextRefiner, Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, FocusedElement, PttSignal, RawTranscript,
    RecordingMode, SpottingEvent, VoiceCommand,
};

use crate::IntegrationWorld;

const SENTENCE: &str = "Meeting moved to Friday.";
/// How long each stand-in for a local service takes, so a stage lasts long enough to see.
const SERVICE_TIME: Duration = Duration::from_millis(100);
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long the pipeline holds the text back for a text input to get focus.
const FOCUS_WAIT: Duration = Duration::from_secs(2);
/// Long enough for every signal of a reaction to arrive.
const SETTLE: Duration = Duration::from_millis(200);
/// How long the failure notice, and so the failure badge, lasts in these scenarios.
const FAILURE_NOTICE_TIME: Duration = Duration::from_millis(400);
/// Longer than the one second of audio the recorder hands over: every recording is too short.
const TOO_SHORT_TO_KEEP_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    RecordingStarted,
    Overlay(Overlay),
    Sign(Sign),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Recording,
    RecognizingSpeech,
    RefiningText,
    WaitingForFocus,
    PastedSign,
    Hidden,
    FailureNotice,
    CommandsUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    Shown(DictationSignState),
    Gone,
}

type Events = Arc<Mutex<Vec<Event>>>;

fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

/// What a dictation-sign scenario shares with the pipeline thread.
#[derive(Default)]
pub struct PointerRun {
    events: Events,
    voice_control: bool,
    hold: bool,
    recognition_down: bool,
    /// A button has keyboard focus instead of a text field.
    button_focused: Arc<AtomicBool>,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log where the scenario's action began.
    acted_at: usize,
}

impl std::fmt::Debug for PointerRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PointerRun")
            .field("events", &self.events())
            .finish()
    }
}

impl PointerRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn after_action(&self) -> Vec<Event> {
        self.events()[self.acted_at..].to_vec()
    }

    fn mark_action(&mut self) {
        self.acted_at = self.events().len();
    }

    fn send(&mut self, input: PipelineInput) -> Result<()> {
        self.ensure_started();
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)?;
        Ok(())
    }

    fn ensure_started(&mut self) {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline());
        }
    }

    fn start_pipeline(&self) -> Sender<PipelineInput> {
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            Box::new(OneSecondRecorder(Arc::clone(&self.events))),
            Box::new(SlowTranscriber {
                down: self.recognition_down,
            }),
            Box::new(SlowRefiner),
            Box::new(QuietPaster),
            Box::new(Silent),
            Box::new(self.indicator()),
            self.config(),
        )
        .with_focus_wait(
            Box::new(SwitchableFocus(Arc::clone(&self.button_focused))),
            FocusWaitCfg {
                timeout: FOCUS_WAIT,
                poll_interval: Duration::from_millis(20),
            },
        );
        if self.voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }

    fn indicator(&self) -> PointerSignIndicator<NotingOverlay, NotingSign> {
        let lifetimes = SignLifetimes {
            failure: FAILURE_NOTICE_TIME,
            ..SignLifetimes::default()
        };
        PointerSignIndicator::with_lifetimes(
            NotingOverlay(Arc::clone(&self.events)),
            NotingSign(Arc::clone(&self.events)),
            lifetimes,
        )
    }

    fn config(&self) -> PipelineConfig {
        PipelineConfig {
            min_press_ms: if self.hold { TOO_SHORT_TO_KEEP_MS } else { 0 },
            recording_mode: if self.hold {
                RecordingMode::Hold
            } else {
                RecordingMode::Toggle
            },
            chunking: ChunkingCfg {
                chunk_threshold_secs: 0,
                ..ChunkingCfg::default()
            },
        }
    }

    fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Reports `command` as the command spotting service does when the user says it alone.
    fn say(&mut self, command: VoiceCommand) -> Result<()> {
        let segment_start = Instant::now();
        let start = segment_start + Duration::from_millis(100);
        let end = start + Duration::from_millis(800);
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(
            segment_start,
        )))?;
        self.send(PipelineInput::Spotting(SpottingEvent::PhraseSpotted {
            phrase: command.phrase().to_owned(),
            start,
            end,
            timing: None,
        }))?;
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(
            end + Duration::from_millis(100),
        )))
    }

    /// Starts a recording with the hotkey and waits until the dictation sign shows.
    fn start_recording(&mut self) -> Result<()> {
        self.press_hotkey()?;
        self.wait_for(|events| events.contains(&RECORDING_STATE))
    }

    /// Waits until `done` holds for the events since the action, then for the rest of the
    /// reaction to arrive.
    fn wait_for(&self, done: impl Fn(&[Event]) -> bool) -> Result<()> {
        self.wait_until(done)?;
        std::thread::sleep(SETTLE);
        Ok(())
    }

    /// Waits until `done` holds for the events since the action.
    fn wait_until(&self, done: impl Fn(&[Event]) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !done(&self.after_action()) {
            if Instant::now() >= deadline {
                bail!("not seen in time; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

const RECORDING_STATE: Event = Event::Sign(Sign::Shown(DictationSignState::Recording));

fn run(world: &mut IntegrationWorld) -> &mut PointerRun {
    world.pointer.get_or_insert_with(PointerRun::default)
}

fn sign(events: &[Event]) -> Vec<Sign> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Sign(s) => Some(*s),
            _ => None,
        })
        .collect()
}

fn voice_command(phrase: &str) -> Result<VoiceCommand> {
    VoiceCommand::from_phrase(phrase).with_context(|| format!("{phrase:?} is no voice command"))
}

#[given("no dictation runs yet for the dictation sign")]
fn no_dictation(world: &mut IntegrationWorld) {
    run(world);
}

#[given("voice control is turned on and no dictation runs for the dictation sign")]
fn voice_on_idle(world: &mut IntegrationWorld) {
    run(world).voice_control = true;
}

#[given("listening is on and no dictation runs for the dictation sign")]
fn listening_on_idle(world: &mut IntegrationWorld) {
    let run = run(world);
    run.voice_control = true;
    run.ensure_started();
}

#[given("text refinement is enabled and a button has keyboard focus for the dictation sign")]
fn button_focused(world: &mut IntegrationWorld) {
    run(world).button_focused.store(true, Ordering::SeqCst);
}

#[given("a recording runs and the dictation sign shows")]
fn recording_runs(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_recording()
}

#[given(
    "the speech recognition service is not running while a recording runs with the dictation sign"
)]
fn recognition_down(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.recognition_down = true;
    run.start_recording()
}

#[given("voice control is turned on, a recording runs and the dictation sign shows")]
fn voice_recording_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.start_recording()
}

#[given("the user holds the hotkey down to record and the dictation sign shows")]
fn holds_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.hold = true;
    run.start_recording()
}

#[when("the user starts a recording with the hotkey")]
fn starts_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.press_hotkey()
}

#[when(expr = "the user says {string} to start a dictation")]
fn says_to_start(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = voice_command(&phrase)?;
    let run = run(world);
    run.mark_action();
    run.say(command)
}

#[when("the user records a dictation with speech in it, ends it and then clicks into a text field")]
fn records_ends_and_clicks(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.press_hotkey()?;
    run.wait_until(|events| events.contains(&RECORDING_STATE))?;
    run.press_hotkey()?;
    run.wait_until(|events| events.contains(&Event::Overlay(Overlay::WaitingForFocus)))?;
    run.button_focused.store(false, Ordering::SeqCst);
    Ok(())
}

#[when("the user ends the recording with the hotkey while the dictation sign shows")]
#[when("the user ends the recording and the text is pasted")]
fn ends_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.press_hotkey()
}

#[when(expr = "the user says {string} during the recording")]
fn says_during_recording(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = voice_command(&phrase)?;
    let run = run(world);
    run.mark_action();
    run.say(command)
}

#[when("the user cancels the dictation")]
fn cancels(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.send(PipelineInput::Hotkey(PttSignal::Cancel))
}

#[when("the user releases it before the shortest recording the app keeps")]
fn releases_too_soon(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.send(PipelineInput::Hotkey(PttSignal::Released))
}

#[when("the command spotting service stays away for longer than 5 seconds")]
fn spotting_stays_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingStillAway)
}

#[then("the dictation sign shows a red microphone")]
fn shows_microphone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::Overlay(Overlay::Recording)))?;
    let events = run.after_action();
    if sign(&events) != [Sign::Shown(DictationSignState::Recording)] {
        bail!("expected the red microphone shown once, never gone: {events:?}");
    }
    Ok(())
}

#[then(
    "the dictation sign shows a red microphone, then an ear with a turning ring, then sparkles with a turning ring, then an orange keyboard, then a green check"
)]
fn mirrors_every_stage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_until(|events| sign(events).contains(&Sign::Shown(DictationSignState::Pasted)))?;
    let events = run.after_action();
    let stages = [
        DictationSignState::Recording,
        DictationSignState::RecognizingSpeech,
        DictationSignState::RefiningText,
        DictationSignState::WaitingForFocus,
        DictationSignState::Pasted,
    ]
    .map(Sign::Shown);
    if sign(&events) != stages {
        bail!("the dictation sign did not mirror every stage: {events:?}");
    }
    for processing in [
        DictationSignState::RecognizingSpeech,
        DictationSignState::RefiningText,
    ] {
        if !processing.turns_activity_ring() {
            bail!("{processing:?} shows no turning ring");
        }
    }
    Ok(())
}

#[then("the dictation sign shows a yellow warning triangle until the failure notice hides")]
fn shows_warning_triangle(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let failure = Sign::Shown(DictationSignState::Failure);
    run.wait_until(|events| sign(events).contains(&failure))?;
    if sign(&run.after_action()).last() != Some(&failure) {
        bail!("the warning triangle did not stay: {:?}", run.events());
    }
    let shown_at = Instant::now();
    run.wait_until(|events| sign(events).last() == Some(&Sign::Gone))?;
    if shown_at.elapsed() < FAILURE_NOTICE_TIME / 2 {
        bail!("the warning triangle went before the failure notice hid");
    }
    let events = run.after_action();
    if !events.contains(&Event::Overlay(Overlay::FailureNotice)) {
        bail!("no failure notice shown: {events:?}");
    }
    Ok(())
}

#[then("the dictation sign keeps showing the red microphone")]
fn keeps_microphone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let events = run.after_action();
    if !sign(&events).is_empty() {
        bail!("the command sign changed the dictation sign: {events:?}");
    }
    if sign(&run.events()).last() != Some(&Sign::Shown(DictationSignState::Recording)) {
        bail!("the red microphone is not showing: {:?}", run.events());
    }
    Ok(())
}

#[then("the dictation sign is gone when the overlay hides after the Pasted sign")]
fn gone_after_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| sign(events).last() == Some(&Sign::Gone))?;
    let events = run.after_action();
    let expected = [
        Sign::Shown(DictationSignState::RecognizingSpeech),
        Sign::Shown(DictationSignState::RefiningText),
        Sign::Shown(DictationSignState::Pasted),
        Sign::Gone,
    ];
    if sign(&events) != expected {
        bail!("the dictation sign did not go after the Pasted sign: {events:?}");
    }
    Ok(())
}

#[then("the dictation sign is gone")]
fn gone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::Sign(Sign::Gone)))?;
    let events = run.after_action();
    if sign(&events).last() != Some(&Sign::Gone) {
        bail!("the dictation sign shows again: {events:?}");
    }
    Ok(())
}

#[then("the dictation sign shows next to the pointer for the new recording")]
fn shows_for_new_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::RecordingStarted))?;
    let events = run.after_action();
    let started = events
        .iter()
        .position(|e| *e == Event::RecordingStarted)
        .context("no new recording")?;
    if sign(&events[started..]).last() != Some(&Sign::Shown(DictationSignState::Recording)) {
        bail!("the dictation sign does not show for the new recording: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows the commands-unavailable sign and no dictation sign shows")]
fn commands_unavailable_without_sign(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::Overlay(Overlay::CommandsUnavailable)))?;
    let events = run.events();
    if !sign(&events).is_empty() {
        bail!("the dictation sign showed: {events:?}");
    }
    Ok(())
}

/// The dictation sign's window as the adapter sees it.
struct NotingSign(Events);

impl PointerSignView for NotingSign {
    fn show(&self, badge: DictationSignState) {
        record(&self.0, Event::Sign(Sign::Shown(badge)));
    }

    fn hide(&self) {
        record(&self.0, Event::Sign(Sign::Gone));
    }
}

struct NotingOverlay(Events);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        record(&self.0, Event::Overlay(Overlay::Recording));
    }

    fn show_recognizing_speech(&self) {
        record(&self.0, Event::Overlay(Overlay::RecognizingSpeech));
    }

    fn show_refining_text(&self) {
        record(&self.0, Event::Overlay(Overlay::RefiningText));
    }

    fn show_pasted(&self) {
        record(&self.0, Event::Overlay(Overlay::PastedSign));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.0, Event::Overlay(Overlay::WaitingForFocus));
    }

    fn hide(&self) {
        record(&self.0, Event::Overlay(Overlay::Hidden));
    }

    fn show_failure_notice(&self, _notice: &FailureNotice) {
        record(&self.0, Event::Overlay(Overlay::FailureNotice));
    }

    fn show_commands_unavailable(&self) {
        record(&self.0, Event::Overlay(Overlay::CommandsUnavailable));
    }
}

/// Hands over one second of audio at the stop.
struct OneSecondRecorder(Events);

impl AudioRecorder for OneSecondRecorder {
    fn start(&mut self) -> Result<()> {
        record(&self.0, Event::RecordingStarted);
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

/// The speech recognition service: answers after a moment, or refuses the connection when
/// it is down.
struct SlowTranscriber {
    down: bool,
}

impl Transcriber for SlowTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        std::thread::sleep(SERVICE_TIME);
        if self.down {
            bail!("connection refused");
        }
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
    }
}

struct SlowRefiner;

impl TextRefiner for SlowRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        std::thread::sleep(SERVICE_TIME);
        Ok(CleanedText::new(raw.as_str().to_owned())?)
    }
}

struct QuietPaster;

impl TextPaster for QuietPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }
}

/// A text field has keyboard focus, or a button while the flag is set.
struct SwitchableFocus(Arc<AtomicBool>);

impl FocusProbe for SwitchableFocus {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        let button = self.0.load(Ordering::SeqCst);
        Ok(Some(FocusedElement {
            role: if button { "AXButton" } else { "AXTextField" }.to_owned(),
            has_text_selection: !button,
            value_settable: !button,
            in_web_content: false,
        }))
    }
}

/// The command spotting service as the pipeline sees it; the scenario feeds the phrase it
/// reports straight into the pipeline's input channel.
struct QuietSpotter;

impl CommandSpotter for QuietSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {}
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

struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {}
    fn play_listening_off(&self) {}
}
