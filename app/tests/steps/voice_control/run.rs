//! The voice run: the pipeline under test with its doubles and its event log.

use super::{
    bail, mpsc, Arc, CommandPlacement, Cut, Duration, Event, Events, Instant, IntegrationWorld,
    NotingRecorder, OnceLock, Phrases, Pipeline, PipelineConfig, PipelineInput, PipelineLog,
    PipelineParts, PttSignal, RecordingKeyPresser, RecordingOverlay, RecordingPaster,
    RecordingSounds, Result, ScriptedSpotter, Sender, Speech, SpeechBeforeCut, SpottingEvent,
    ThreadId, VoiceCommand, PHRASE_LEAD, PHRASE_LENGTH, REACTION, SEGMENT_TAIL, SENTENCE,
    SENTENCE_AROUND_PHRASE, SETTLE,
};
use voicepuncher::test_doubles::StartedOnDemand;

pub(super) static PIPELINE_LOG: OnceLock<&'static PipelineLog> = OnceLock::new();

pub(crate) fn pipeline_log() -> &'static PipelineLog {
    PIPELINE_LOG.get_or_init(|| {
        let log: &'static PipelineLog = Box::leak(Box::default());
        if log::set_logger(log).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
        log
    })
}

/// What a voice-control scenario shares with the pipeline thread.
#[derive(Default)]
pub struct VoiceRun {
    pub(crate) voice_control: bool,
    events: Events,
    speech: Speech,
    cut: Cut,
    phrases: Phrases,
    pub(crate) inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log where the scenario's `When` started.
    pub(crate) mark: usize,
    /// When the command spotting service went away, if it did.
    pub(crate) away_since: Option<Instant>,
    /// The thread the pipeline runs on: what it logs is the scenario's.
    pub(crate) pipeline_thread: Option<ThreadId>,
    /// How much microphone audio never reached the command spotting service.
    pub(crate) audio_lost: Duration,
    /// When the words of the scenario's command ended.
    pub(crate) command_end: Option<Instant>,
}

impl std::fmt::Debug for VoiceRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceRun")
            .field("events", &self.events())
            .finish()
    }
}

impl VoiceRun {
    pub(crate) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub(crate) fn since_mark(&self) -> Vec<Event> {
        self.events()[self.mark..].to_vec()
    }

    pub(crate) fn start_when(&mut self) {
        self.mark = self.events.lock().unwrap().len();
    }

    pub(crate) fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Starts a dictation with the hotkey and says the sentence into it.
    pub(crate) fn dictate_with_hotkey(&mut self) -> Result<()> {
        let started = self.events().len();
        self.press_hotkey()?;
        let deadline = Instant::now() + REACTION;
        while !self.events()[started..].contains(&Event::RecordingStarted) {
            if Instant::now() >= deadline {
                bail!(
                    "the hotkey started no dictation; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.speak(SENTENCE, Instant::now());
        Ok(())
    }

    /// Says `words` as a whole utterance, with a pause before and after.
    pub(crate) fn say_alone(&mut self, words: &str) -> Result<()> {
        let segment_start = Instant::now();
        let phrase_start = segment_start + PHRASE_LEAD;
        let phrase_end = phrase_start + PHRASE_LENGTH;
        self.speak(words, phrase_start);
        self.send_segment(
            words,
            segment_start..phrase_end + SEGMENT_TAIL,
            phrase_start..phrase_end,
        )
    }

    /// Says `sentence`, which holds a command phrase, without pausing around the phrase.
    pub(crate) fn say_in_sentence(&mut self, sentence: &str, phrase: &str) -> Result<()> {
        let segment_start = Instant::now();
        let phrase_start = segment_start + SENTENCE_AROUND_PHRASE;
        let phrase_end = phrase_start + PHRASE_LENGTH;
        self.speak(sentence, segment_start);
        let segment_end = phrase_end + SENTENCE_AROUND_PHRASE;
        self.send_segment(phrase, segment_start..segment_end, phrase_start..phrase_end)
    }

    /// Reports `commands` said without a pause at `start..end`, as the service does.
    pub(crate) fn say_commands(
        &mut self,
        commands: &[VoiceCommand],
        start: Instant,
        end: Instant,
        placement: CommandPlacement,
    ) -> Result<()> {
        self.send(PipelineInput::Spotting(SpottingEvent::CommandsSaid {
            commands: commands.to_vec(),
            start,
            end,
            placement,
            timing: None,
        }))
    }

    pub(crate) fn speak(&self, words: &str, at: Instant) {
        self.speech.lock().unwrap().push((at, words.to_owned()));
    }

    /// Reports a segment of speech over `segment` with `phrase` spoken over `spoken`, the
    /// phrase only while the service listens for it.
    pub(crate) fn send_segment(
        &mut self,
        phrase: &str,
        segment: std::ops::Range<Instant>,
        spoken: std::ops::Range<Instant>,
    ) -> Result<()> {
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(
            segment.start,
        )))?;
        if self.listened_for(phrase) {
            let phrase = phrase.to_owned();
            self.send(PipelineInput::Spotting(SpottingEvent::PhraseSpotted {
                phrase,
                start: spoken.start,
                end: spoken.end,
                timing: None,
            }))?;
        }
        self.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(
            segment.end,
        )))
    }

    /// Whether the service listens for `phrase` now. The pipeline may still be switching
    /// the phrase set after an earlier input, so this waits for it to settle first.
    fn listened_for(&self, phrase: &str) -> bool {
        std::thread::sleep(Duration::from_millis(20));
        VoiceCommand::from_phrase(phrase)
            .is_some_and(|command| self.phrases.lock().unwrap().contains(&command))
    }

    pub(crate) fn wait_for(&self, wanted: impl Fn(&Event) -> bool) -> Result<usize> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(at) = self.events().iter().position(&wanted) {
                return Ok(at);
            }
            if Instant::now() >= deadline {
                bail!("not seen within {REACTION:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Waits until `wanted` happens after the scenario's `When` started.
    pub(crate) fn wait_since_mark(&self, wanted: impl Fn(&Event) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.since_mark().iter().any(&wanted) {
            if Instant::now() >= deadline {
                bail!(
                    "not seen after the action within {REACTION:?}; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    pub(crate) fn never_since_mark(
        &self,
        unwanted: impl Fn(&Event) -> bool,
        what: &str,
    ) -> Result<()> {
        std::thread::sleep(SETTLE);
        if let Some(event) = self.since_mark().into_iter().find(unwanted) {
            bail!("{what}: {event:?}; events: {:?}", self.events());
        }
        Ok(())
    }

    pub(crate) fn listening(&self) -> bool {
        self.events()
            .iter()
            .rev()
            .find(|e| matches!(e, Event::ListeningStarted | Event::ListeningStopped))
            == Some(&Event::ListeningStarted)
    }

    pub(crate) fn pasted(&self) -> Result<String> {
        let at = self.wait_for(|e| matches!(e, Event::Pasted(_)))?;
        match &self.events()[at] {
            Event::Pasted(text) => Ok(text.clone()),
            other => bail!("expected a paste, got {other:?}"),
        }
    }
}

impl StartedOnDemand for VoiceRun {
    fn inputs(&mut self) -> &mut Option<Sender<PipelineInput>> {
        &mut self.inputs
    }
    fn start_pipeline(&mut self) -> Sender<PipelineInput> {
        pipeline_log();
        let (tx, rx) = mpsc::channel();
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(NotingRecorder {
                    events: Arc::clone(&self.events),
                    speech: Arc::clone(&self.speech),
                    cut: Arc::clone(&self.cut),
                    said_before: Vec::new(),
                }),
                transcriber: Box::new(SpeechBeforeCut {
                    speech: Arc::clone(&self.speech),
                    cut: Arc::clone(&self.cut),
                }),
                paster: Box::new(RecordingPaster::new(&self.events, Event::Pasted)),
                sound_player: Box::new(RecordingSounds(Arc::clone(&self.events))),
                visual_indicator: Box::new(RecordingOverlay(Arc::clone(&self.events))),
            },
            PipelineConfig::default(),
        );
        if self.voice_control {
            pipeline = pipeline.with_voice_control(
                Box::new(ScriptedSpotter {
                    events: Arc::clone(&self.events),
                    phrases: Arc::clone(&self.phrases),
                }),
                Box::new(RecordingKeyPresser(Arc::clone(&self.events))),
            );
        }
        let pipeline_thread = std::thread::spawn(move || pipeline.run(rx));
        self.pipeline_thread = Some(pipeline_thread.thread().id());
        tx
    }
}

pub(crate) fn run(world: &mut IntegrationWorld) -> &mut VoiceRun {
    world.pipeline.voice.get_or_insert_with(VoiceRun::default)
}

pub(super) fn is_key_press(e: &Event) -> bool {
    matches!(
        e,
        Event::ReturnPressed | Event::AllTextCleared | Event::PasteKeyPressed
    )
}
