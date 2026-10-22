//! Drives the real pipeline on its own thread: a stand-in recorder records one second, a
//! stand-in speech recognition service answers a fixed transcript, the focus is scripted
//! and can change while the pipeline waits, and the paster and overlay record what they
//! were asked to do.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::textrefinement::passthrough::PassthroughTextRefiner;
use voicepuncher::core::application::audio_recorder::LiveChunk;
use voicepuncher::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
use voicepuncher::core::application::{
    AudioRecorder, ChunkingCfg, FocusProbe, FocusWaitCfg, PipelineInput, SoundPlayer, TextPaster,
    Transcriber, VisualIndicator,
};
use voicepuncher::core::domain::{
    AudioBuffer16k, CleanedText, FailureNotice, FocusedElement, PttSignal, RawTranscript,
};

use crate::IntegrationWorld;
mod doubles;
use doubles::{
    FixedTranscript, OneSecondRecorder, RecordingIndicator, RecordingPaster, ScriptedFocus, Silent,
};

const DICTATED: &str = "Meeting moved to Friday.";
const WAIT_TIME: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(1);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// How late the pipeline's thread records the last event of a reaction, as a busy CPU makes it.
/// It stays so a step that reads the log right after an earlier event of the reaction fails on
/// every run instead of now and then.
const PIPELINE_LAG: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    RecordingStarted,
    WaitingShown,
    PastedSignShown,
    Hidden,
    FailureNotice(FailureNotice),
    Pasted(String),
    LeftInClipboard(String),
}

#[derive(Debug, Clone, Copy)]
enum Focus {
    TextField,
    Button,
    WebPage,
    WebPageWithSelectedText,
    WebTextInput,
    WebEditable,
    TerminalWindow,
    ListWithTextSelection,
    Unreported,
}

/// What a text-insertion scenario shares with the pipeline thread.
pub struct InsertionRun {
    focus: Arc<Mutex<Focus>>,
    events: Arc<Mutex<Vec<Event>>>,
    wait_time: Duration,
    signals: Option<Sender<PipelineInput>>,
    waiting_shown_at: Arc<Mutex<Option<Instant>>>,
    left_in_clipboard_at: Arc<Mutex<Option<Instant>>>,
}

impl std::fmt::Debug for InsertionRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsertionRun")
            .field("events", &self.events())
            .finish()
    }
}

impl Default for InsertionRun {
    fn default() -> Self {
        Self {
            focus: Arc::new(Mutex::new(Focus::TextField)),
            events: Arc::default(),
            wait_time: WAIT_TIME,
            signals: None,
            waiting_shown_at: Arc::default(),
            left_in_clipboard_at: Arc::default(),
        }
    }
}

impl InsertionRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn set_focus(&self, focus: Focus) {
        *self.focus.lock().unwrap() = focus;
    }

    /// Sends a hotkey press to the pipeline, starting the pipeline on first use.
    fn press_hotkey(&mut self) -> Result<()> {
        if self.signals.is_none() {
            self.signals = Some(self.start_pipeline());
        }
        self.signals
            .as_ref()
            .context("pipeline not running")?
            .send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        Ok(())
    }

    fn start_pipeline(&mut self) -> Sender<PipelineInput> {
        let (tx, rx) = mpsc::channel();
        let events = Arc::clone(&self.events);
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(OneSecondRecorder(Arc::clone(&events))),
                transcriber: Box::new(FixedTranscript),
                refiner: Box::new(PassthroughTextRefiner),
                paster: Box::new(RecordingPaster(
                    Arc::clone(&events),
                    Arc::clone(&self.left_in_clipboard_at),
                )),
                sound_player: Box::new(Silent),
                visual_indicator: Box::new(RecordingIndicator(
                    Arc::clone(&events),
                    Arc::clone(&self.waiting_shown_at),
                )),
            },
            PipelineConfig::default(),
        )
        .with_focus_wait(
            Box::new(ScriptedFocus(Arc::clone(&self.focus))),
            FocusWaitCfg { timeout: self.wait_time, poll_interval: POLL_INTERVAL },
        );
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }

    fn finish_dictation(&mut self) -> Result<()> {
        self.press_hotkey()?;
        self.wait_for(|e| *e == Event::RecordingStarted, REACTION)?;
        self.press_hotkey()
    }

    fn wait_for(&self, wanted: impl Fn(&Event) -> bool, within: Duration) -> Result<Event> {
        let deadline = Instant::now() + within;
        loop {
            if let Some(event) = self.events().into_iter().find(&wanted) {
                return Ok(event);
            }
            if Instant::now() >= deadline {
                bail!("not seen within {within:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn count(&self, wanted: impl Fn(&Event) -> bool) -> usize {
        self.events().iter().filter(|e| wanted(e)).count()
    }
}

fn run(world: &mut IntegrationWorld) -> &mut InsertionRun {
    world.insertion.get_or_insert_with(InsertionRun::default)
}

/// Checks that the overlay ends the delivered dictation on the Pasted sign, which is handed no
/// text to show under it: nothing follows the sign, which takes itself down.
pub(crate) fn ends_on_pasted_sign(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(is_pasted, REACTION)?;
    run.wait_for(is_pasted_sign, REACTION)?;
    std::thread::sleep(SETTLE);
    let events = run.events();
    let pasted = events.iter().position(is_pasted).unwrap();
    let after_paste = &events[pasted + 1..];
    if after_paste.last() != Some(&Event::PastedSignShown) {
        bail!("expected the overlay to end on the Pasted sign: {events:?}");
    }
    Ok(())
}

/// Checks that the overlay hides after the text was left in the clipboard and never shows
/// the Pasted sign.
pub(crate) fn hidden_without_pasted_sign(world: &mut IntegrationWorld) -> Result<()> {
    overlay_hides(world)?;
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(is_pasted_sign) > 0 {
        bail!("the overlay showed the Pasted sign: {:?}", run.events());
    }
    Ok(())
}

fn is_pasted_sign(e: &Event) -> bool {
    matches!(e, Event::PastedSignShown)
}

fn is_pasted(e: &Event) -> bool {
    matches!(e, Event::Pasted(_))
}

fn is_left_in_clipboard(e: &Event) -> bool {
    matches!(e, Event::LeftInClipboard(_))
}

#[given("a text field has keyboard focus")]
fn text_field_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::TextField);
}

#[given("the focused app does not report its focused element")]
fn focus_unreported(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::Unreported);
}

#[given("a text input on a web page has keyboard focus")]
fn web_text_input_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebTextInput);
}

#[given("an editable element on a web page has keyboard focus")]
fn web_editable_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebEditable);
}

#[given("a web page has keyboard focus and no text input on it has")]
fn web_page_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebPage);
}

#[given("a web page with selected text has keyboard focus and no text input on it has")]
fn web_page_with_selected_text_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebPageWithSelectedText);
}

#[given("the focused element is a terminal's window outside web content")]
fn terminal_window_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::TerminalWindow);
}

#[given("the focused element is a list that offers a text selection")]
fn list_with_selection_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::ListWithTextSelection);
}

#[given("a button has keyboard focus")]
fn button_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::Button);
}

#[given("the config file sets the wait time to 0")]
fn wait_time_zero(world: &mut IntegrationWorld) {
    run(world).wait_time = Duration::ZERO;
}

#[given("the app holds the text back because a button has focus")]
fn text_held_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.set_focus(Focus::Button);
    run.finish_dictation()?;
    run.wait_for(|e| *e == Event::WaitingShown, REACTION)?;
    Ok(())
}

#[given("a hotkey press ended a wait for focus")]
fn wait_ended_by_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    text_held_back(world)?;
    let run = run(world);
    run.press_hotkey()?;
    run.wait_for(is_left_in_clipboard, REACTION)?;
    Ok(())
}

#[given("the app holds the text back because a web page has focus without a text input")]
fn text_held_back_on_web_page(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.set_focus(Focus::WebPage);
    run.finish_dictation()?;
    run.wait_for(|e| *e == Event::WaitingShown, REACTION)?;
    Ok(())
}

#[when("a dictation finishes")]
fn dictation_finishes(world: &mut IntegrationWorld) -> Result<()> {
    run(world).finish_dictation()
}

#[when("the user clicks into a text field")]
fn click_into_text_field(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::TextField);
}

#[when("the user clicks into a text input on that page")]
fn click_into_web_text_input(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebTextInput);
}

#[when("the wait time passes and the button still has focus")]
fn wait_time_passes(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(is_left_in_clipboard, run.wait_time + REACTION)?;
    Ok(())
}

#[when("the user presses the hotkey")]
#[when("the user presses the hotkey again")]
fn press_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).press_hotkey()
}

#[then("the text is pasted at once")]
fn pasted_at_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(is_pasted, REACTION)?;
    if run.count(|e| *e == Event::WaitingShown) > 0 {
        bail!("the app waited for focus first: {:?}", run.events());
    }
    Ok(())
}

#[then("the overlay never shows that it waits for focus")]
fn never_waited(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(|e| *e == Event::WaitingShown) > 0 {
        bail!("the overlay showed the wait: {:?}", run.events());
    }
    Ok(())
}

#[then("nothing is pasted yet")]
#[then("nothing is pasted")]
fn nothing_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.count(is_pasted) > 0 {
        bail!("text was pasted: {:?}", run.events());
    }
    Ok(())
}

#[then("the overlay shows that the app waits for focus")]
fn waiting_shown(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::WaitingShown, REACTION)?;
    Ok(())
}

#[then("the text is pasted into it")]
fn pasted_after_wait(world: &mut IntegrationWorld) -> Result<()> {
    let event = run(world).wait_for(is_pasted, REACTION)?;
    if event != Event::Pasted(DICTATED.to_owned()) {
        bail!("pasted something else: {event:?}");
    }
    Ok(())
}

#[then("the overlay hides")]
fn overlay_hides(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let outcome = run.wait_for(|e| is_pasted(e) || is_left_in_clipboard(e), REACTION)?;
    let deadline = Instant::now() + REACTION;
    loop {
        let events = run.events();
        let delivered = events.iter().position(|e| *e == outcome).unwrap();
        if events[delivered..].contains(&Event::Hidden) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("overlay not hidden after {outcome:?}: {events:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[then("the clipboard holds the text")]
fn clipboard_holds_text(world: &mut IntegrationWorld) -> Result<()> {
    let event = run(world).wait_for(is_left_in_clipboard, REACTION)?;
    if event != Event::LeftInClipboard(DICTATED.to_owned()) {
        bail!("clipboard holds something else: {event:?}");
    }
    Ok(())
}

#[then("no recording starts")]
fn no_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let started = run.count(|e| *e == Event::RecordingStarted);
    if started != 1 {
        bail!("{started} recordings started, expected only the dictation's");
    }
    let waiting_at = run
        .waiting_shown_at
        .lock()
        .unwrap()
        .context("the app did not wait for focus")?;
    let left_at = run
        .left_in_clipboard_at
        .lock()
        .unwrap()
        .context("the text was not left in the clipboard")?;
    if left_at.duration_since(waiting_at) >= run.wait_time {
        bail!("the wait ended by its time, not by the hotkey press");
    }
    Ok(())
}

#[then("a recording starts")]
fn recording_starts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let deadline = Instant::now() + REACTION;
    while run.count(|e| *e == Event::RecordingStarted) < 2 {
        if Instant::now() >= deadline {
            bail!("no new recording: {:?}", run.events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
