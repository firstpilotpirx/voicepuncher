//! Drives the real pipeline on its own thread: a stand-in recorder records one second, a
//! stand-in speech recognition service answers a fixed transcript, the focus is scripted
//! and can change while the pipeline waits, and the paster and overlay record what they
//! were asked to do.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use crate::IntegrationWorld;
pub(crate) mod clipboard;
pub(crate) mod doubles;
pub(crate) mod filler_sounds;
mod focus_log;
use doubles::{
    is_left_in_clipboard, is_pasted, is_pasted_sign, FixedTranscript, OneSecondRecorder,
    RecordingIndicator, RecordingPaster, ScriptedFocus, Silent,
};

mod focus;
mod outcomes;
use outcomes::overlay_hides;

pub(crate) const DICTATED: &str = "Meeting moved to Friday.";
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
pub(crate) enum Event {
    RecordingStarted,
    WaitingShown,
    PastedSignShown,
    Hidden,
    FailureNotice(FailureNotice),
    Pasted(String),
    LeftInClipboard(String),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Focus {
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
                recorder: Box::new(OneSecondRecorder::new(&events, Event::RecordingStarted)),
                transcriber: Box::new(FixedTranscript),
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
    world
        .pipeline
        .insertion
        .get_or_insert_with(InsertionRun::default)
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
