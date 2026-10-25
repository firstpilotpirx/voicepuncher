//! The stages run: the pipeline with the overlay's stage log.

use super::{
    bail, mpsc, Arc, ChunkingCfg, Context, Duration, Event, FocusOnButton, FocusWaitCfg, Instant,
    IntegrationWorld, Mutex, NoKeys, NotingOverlay, NotingPaster, NotingTranscriber, Overlay,
    PartsRecorder, Pipeline, PipelineConfig, PipelineInput, PipelineParts, PttSignal, QuietSpotter,
    Result, Sender, Silent, VoiceCommand, FOCUS_WAIT, REACTION, SETTLE,
};

pub(super) type Events = Arc<Mutex<Vec<Event>>>;

/// Appends an event to the scenario's event log.
pub(super) fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}

/// What a processing-stages scenario shares with the pipeline thread.
/// What the stand-in microphone hands over.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Recorded {
    /// Speech, in one part at the stop.
    #[default]
    Speech,
    /// Speech, in parts transcribed while the user speaks.
    InParts,
    NoSpeech,
}

#[derive(Default)]
pub struct StagesRun {
    events: Events,
    pub(crate) recorded: Recorded,
    pub(crate) no_text_focus: bool,
    pub(crate) voice_control: bool,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log where the recording was ended.
    stop_at: usize,
}

impl std::fmt::Debug for StagesRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagesRun")
            .field("events", &self.events())
            .finish()
    }
}

impl StagesRun {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn after_stop(&self) -> Vec<Event> {
        self.events()[self.stop_at..].to_vec()
    }

    fn send(&mut self, input: PipelineInput) -> Result<()> {
        if self.inputs.is_none() {
            self.inputs = Some(self.start_pipeline());
        }
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)?;
        Ok(())
    }

    fn parts(&self) -> PipelineParts {
        let events = &self.events;
        PipelineParts {
            recorder: Box::new(PartsRecorder {
                events: Arc::clone(events),
                in_parts: self.recorded == Recorded::InParts,
                parts: voicepuncher::test_doubles::OneSecondParts::default(),
            }),
            transcriber: Box::new(NotingTranscriber {
                events: Arc::clone(events),
                no_speech: self.recorded == Recorded::NoSpeech,
            }),
            paster: Box::new(NotingPaster::new(events, |_| Event::Pasted)),
            sound_player: Box::new(Silent),
            visual_indicator: Box::new(NotingOverlay { events: Arc::clone(events) }),
        }
    }

    fn start_pipeline(&self) -> Sender<PipelineInput> {
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            self.parts(),
            PipelineConfig {
                chunking: ChunkingCfg {
                    chunk_threshold_secs: if self.recorded == Recorded::InParts {
                        5
                    } else {
                        0
                    },
                    ..ChunkingCfg::default()
                },
                ..PipelineConfig::default()
            },
        );
        if self.no_text_focus {
            pipeline = pipeline.with_focus_wait(
                Box::new(FocusOnButton),
                FocusWaitCfg { timeout: FOCUS_WAIT, poll_interval: Duration::from_millis(20) },
            );
        }
        if self.voice_control {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        std::thread::spawn(move || pipeline.run(rx));
        tx
    }

    /// Starts a recording with the hotkey and waits until it runs.
    pub(crate) fn start_recording(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
        self.wait_for(Event::RecordingStarted)?;
        self.wait_for(Event::Overlay(Overlay::Recording))?;
        if self.recorded == Recorded::InParts {
            // the first part is transcribed while the user still speaks
            self.wait_for(Event::TranscriptionFinished)?;
        }
        Ok(())
    }

    pub(crate) fn end_with_hotkey(&mut self) -> Result<()> {
        self.stop_at = self.events().len();
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    pub(crate) fn end_by_voice(&mut self) -> Result<()> {
        self.stop_at = self.events().len();
        let phrase = VoiceCommand::StopDictation.phrase();
        for event in voicepuncher::test_doubles::said_alone(phrase, Instant::now()) {
            self.send(PipelineInput::Spotting(event))?;
        }
        Ok(())
    }

    fn wait_for(&self, wanted: Event) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.events().contains(&wanted) {
            if Instant::now() >= deadline {
                bail!("{wanted:?} not seen; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    /// Waits until the dictation is over: the overlay hides, or shows the Pasted sign that
    /// takes itself down, after the stop.
    pub(crate) fn wait_until_over(&self) -> Result<Vec<Event>> {
        let deadline = Instant::now() + REACTION;
        let ends = |e: &Event| matches!(e, Event::Overlay(Overlay::Hidden | Overlay::PastedSign));
        while !self.after_stop().iter().any(ends) {
            if Instant::now() >= deadline {
                bail!("the overlay never hid; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(SETTLE);
        Ok(self.after_stop())
    }
}

pub(super) fn run(world: &mut IntegrationWorld) -> &mut StagesRun {
    world.pipeline.stages.get_or_insert_with(StagesRun::default)
}

pub(super) fn position(events: &[Event], wanted: Event) -> Result<usize> {
    events
        .iter()
        .position(|e| *e == wanted)
        .with_context(|| format!("{wanted:?} not seen after the stop; events: {events:?}"))
}

pub(super) fn last_position(events: &[Event], wanted: Event) -> Result<usize> {
    events
        .iter()
        .rposition(|e| *e == wanted)
        .with_context(|| format!("{wanted:?} not seen after the stop; events: {events:?}"))
}

/// The overlay signals in `events`, in order.
pub(super) fn overlay(events: &[Event]) -> Vec<Overlay> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Overlay(o) => Some(*o),
            _ => None,
        })
        .collect()
}

/// Speech recognition is shown before the transcription after the stop starts and stays
/// until it finished: the next overlay signal comes after it.
pub(super) fn recognition_covers_transcription(events: &[Event]) -> Result<()> {
    let shown = position(events, Event::Overlay(Overlay::RecognizingSpeech))?;
    let started = position(events, Event::TranscriptionStarted)?;
    let finished = last_position(events, Event::TranscriptionFinished)?;
    if shown > started {
        bail!("speech recognition shown after the transcription started: {events:?}");
    }
    let next_signal = events[shown + 1..]
        .iter()
        .position(|e| matches!(e, Event::Overlay(_)))
        .map(|at| at + shown + 1);
    if next_signal.is_some_and(|next| next < finished) {
        bail!("the overlay left speech recognition before the transcript was ready: {events:?}");
    }
    Ok(())
}
