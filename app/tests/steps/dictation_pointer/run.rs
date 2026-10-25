//! The pointer run: the pipeline with the dictation sign's log.

use super::{
    bail, mpsc, Arc, AtomicBool, Context, DictationSignState, Duration, Event, FocusWaitCfg,
    Instant, IntegrationWorld, Mutex, NoKeys, NotingOverlay, NotingSign, OneSecondRecorder,
    Pipeline, PipelineConfig, PipelineInput, PipelineParts, PointerSignIndicator, PttSignal,
    QuietPaster, QuietSpotter, RecordingMode, Result, Sender, Sign, SignLifetimes, Silent,
    SlowTranscriber, SwitchableFocus, VoiceCommand, FAILURE_NOTICE_TIME, FOCUS_WAIT, REACTION,
    SETTLE, TOO_SHORT_TO_KEEP_MS,
};
use voicepuncher::test_doubles::StartedOnDemand;

pub(super) type Events = Arc<Mutex<Vec<Event>>>;

pub(super) fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

/// What a dictation-sign scenario shares with the pipeline thread.
#[derive(Default)]
pub struct PointerRun {
    events: Events,
    pub(crate) voice_control: bool,
    pub(crate) hold: bool,
    pub(crate) recognition_down: bool,
    /// A button has keyboard focus instead of a text field.
    pub(crate) button_focused: Arc<AtomicBool>,
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
    pub(crate) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub(crate) fn after_action(&self) -> Vec<Event> {
        self.events()[self.acted_at..].to_vec()
    }

    pub(crate) fn mark_action(&mut self) {
        self.acted_at = self.events().len();
    }

    fn indicator(&self) -> PointerSignIndicator<NotingOverlay, NotingSign> {
        let lifetimes = SignLifetimes { failure: FAILURE_NOTICE_TIME, ..SignLifetimes::default() };
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
            ..PipelineConfig::default()
        }
    }

    pub(crate) fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Reports `command` as the command spotting service does when the user says it alone.
    pub(crate) fn say(&mut self, command: VoiceCommand) -> Result<()> {
        for event in voicepuncher::test_doubles::said_alone(command.phrase(), Instant::now()) {
            self.send(PipelineInput::Spotting(event))?;
        }
        Ok(())
    }

    /// Starts a recording with the hotkey and waits until the dictation sign shows.
    pub(crate) fn start_recording(&mut self) -> Result<()> {
        self.press_hotkey()?;
        self.wait_for(|events| events.contains(&RECORDING_STATE))
    }

    /// Waits until `done` holds for the events since the action, then for the rest of the
    /// reaction to arrive.
    pub(crate) fn wait_for(&self, done: impl Fn(&[Event]) -> bool) -> Result<()> {
        self.wait_until(done)?;
        std::thread::sleep(SETTLE);
        Ok(())
    }

    /// Waits until `done` holds for the events since the action.
    pub(crate) fn wait_until(&self, done: impl Fn(&[Event]) -> bool) -> Result<()> {
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

pub(super) const RECORDING_STATE: Event = Event::Sign(Sign::Shown(DictationSignState::Recording));

pub(super) fn run(world: &mut IntegrationWorld) -> &mut PointerRun {
    world
        .pipeline
        .pointer
        .get_or_insert_with(PointerRun::default)
}

pub(super) fn sign(events: &[Event]) -> Vec<Sign> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Sign(s) => Some(*s),
            _ => None,
        })
        .collect()
}

pub(super) fn voice_command(phrase: &str) -> Result<VoiceCommand> {
    VoiceCommand::from_phrase(phrase).with_context(|| format!("{phrase:?} is no voice command"))
}

impl StartedOnDemand for PointerRun {
    fn inputs(&mut self) -> &mut Option<Sender<PipelineInput>> {
        &mut self.inputs
    }
    fn start_pipeline(&mut self) -> Sender<PipelineInput> {
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(OneSecondRecorder::new(
                    &self.events,
                    Event::RecordingStarted,
                )),
                transcriber: Box::new(SlowTranscriber { down: self.recognition_down }),
                paster: Box::new(QuietPaster),
                sound_player: Box::new(Silent),
                visual_indicator: Box::new(self.indicator()),
            },
            self.config(),
        )
        .with_focus_wait(
            Box::new(SwitchableFocus(Arc::clone(&self.button_focused))),
            FocusWaitCfg { timeout: FOCUS_WAIT, poll_interval: Duration::from_millis(20) },
        );
        if self.voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }
}
