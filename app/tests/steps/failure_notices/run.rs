//! The notices run: the pipeline with failing parts and the overlay log.

use super::{
    anyhow, bail, mpsc, Arc, ChunkingCfg, Context, Duration, Event, Events, FailureNotice, Instant,
    IntegrationWorld, NotingOverlay, PartsRecorder, Pipeline, PipelineConfig, PipelineInput,
    PipelineParts, PttSignal, QuietSpotter, Result, ScriptedKeys, ScriptedPaster,
    ScriptedTranscriber, Sender, Shown, Silent, VoiceCommand, LONG_DICTATION_THRESHOLD_SECS,
    REACTION, SETTLE,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Recognition {
    #[default]
    Answers,
    Fails,
    FailsOnFirstPart,
    /// No copy answers and another program holds the service's own port.
    PortTaken,
}

/// How the stand-ins behave in a scenario.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Setup {
    pub(crate) recognition: Recognition,
    pub(crate) paste_fails: bool,
    pub(crate) microphone: Microphone,
    pub(crate) long_dictation: bool,
    pub(crate) voice: VoiceSetup,
}

/// How the stand-in microphone behaves.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Microphone {
    #[default]
    Records,
    CannotOpen,
    /// It opens, but a dictation ends with nothing recorded yet.
    RecordsNothing,
}

/// Whether voice control is on, and which of its parts fails.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VoiceSetup {
    #[default]
    Off,
    On,
    KeysRefused,
    SpottingDown,
}

/// What a failure-notices scenario shares with the pipeline thread.
#[derive(Debug, Default)]
pub struct NoticesRun {
    events: Events,
    pub(crate) setup: Setup,
    inputs: Option<Sender<PipelineInput>>,
    /// Index in the event log and moment where the latest recording was ended.
    pub(crate) stop: Option<(usize, Instant)>,
    /// Moment the failure notice was first seen after the stop.
    pub(crate) notice_seen: Option<Instant>,
}

impl NoticesRun {
    pub(crate) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    fn after_stop(&self) -> Vec<Event> {
        let from = self.stop.map_or(0, |(at, _)| at);
        self.events()[from..].to_vec()
    }

    /// Starts the pipeline the way the app does, once.
    pub(crate) fn start(&mut self) {
        if self.inputs.is_none() {
            self.inputs = Some(start_pipeline(&self.events, self.setup));
        }
    }

    pub(crate) fn send(&mut self, input: PipelineInput) -> Result<()> {
        self.start();
        self.inputs
            .as_ref()
            .context("pipeline not running")?
            .send(input)
            .map_err(|_| anyhow!("the pipeline stopped; events: {:?}", self.events()))
    }

    pub(crate) fn press_hotkey(&mut self) -> Result<()> {
        self.send(PipelineInput::Hotkey(PttSignal::Pressed))
    }

    /// Starts a recording with the hotkey and waits until it runs.
    pub(crate) fn start_hotkey_dictation(&mut self) -> Result<()> {
        let from = self.events().len();
        self.press_hotkey()?;
        self.wait_for_after(from, &Event::Overlay(Shown::Recording))?;
        if self.setup.long_dictation {
            // the first part is transcribed while the user still speaks
            std::thread::sleep(SETTLE);
        }
        Ok(())
    }

    pub(crate) fn start_voice_dictation(&mut self) -> Result<()> {
        let from = self.events().len();
        self.say(VoiceCommand::StartDictation)?;
        self.wait_for_after(from, &Event::Overlay(Shown::Recording))
    }

    pub(crate) fn say(&mut self, command: VoiceCommand) -> Result<()> {
        let events = voicepuncher::test_doubles::said_alone(command.phrase(), Instant::now());
        for event in events {
            self.send(PipelineInput::Spotting(event))?;
        }
        Ok(())
    }

    pub(crate) fn mark_stop(&mut self) {
        self.stop = Some((self.events().len(), Instant::now()));
        self.notice_seen = None;
    }

    pub(crate) fn wait_for_after(&self, from: usize, wanted: &Event) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while !self.events()[from..].contains(wanted) {
            if Instant::now() >= deadline {
                bail!("{wanted:?} not seen; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    /// Waits until the dictation is over: the overlay hides, or shows a notice or the Pasted
    /// sign that takes itself down, after the stop.
    pub(crate) fn wait_until_over(&mut self) -> Result<Vec<Event>> {
        let deadline = Instant::now() + REACTION;
        loop {
            let shown = overlay(&self.after_stop());
            if let Some(Shown::Notice(_)) = shown.last() {
                self.notice_seen.get_or_insert_with(Instant::now);
                break;
            }
            if matches!(shown.last(), Some(Shown::Hidden | Shown::PastedSign)) {
                break;
            }
            if Instant::now() >= deadline {
                bail!("the dictation never ended; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(SETTLE);
        Ok(self.after_stop())
    }

    /// Waits until the overlay settles after an action that does not end a dictation.
    pub(crate) fn settled(&self) -> Vec<Event> {
        std::thread::sleep(SETTLE);
        self.after_stop()
    }
}

pub(super) fn start_pipeline(events: &Events, setup: Setup) -> Sender<PipelineInput> {
    let (tx, rx) = mpsc::channel();
    let threshold = if setup.long_dictation {
        LONG_DICTATION_THRESHOLD_SECS
    } else {
        0
    };
    let mut pipeline = Pipeline::new(
        PipelineParts {
            recorder: Box::new(PartsRecorder::new(Arc::clone(events), setup)),
            transcriber: Box::new(ScriptedTranscriber::new(setup.recognition)),
            paster: Box::new(ScriptedPaster {
                events: Arc::clone(events),
                fails: setup.paste_fails,
            }),
            sound_player: Box::new(Silent),
            visual_indicator: Box::new(NotingOverlay(Arc::clone(events))),
        },
        PipelineConfig {
            chunking: ChunkingCfg { chunk_threshold_secs: threshold, ..ChunkingCfg::default() },
            ..PipelineConfig::default()
        },
    );
    if setup.voice != VoiceSetup::Off {
        pipeline = pipeline.with_voice_control(
            Box::new(QuietSpotter { down: setup.voice == VoiceSetup::SpottingDown }),
            Box::new(ScriptedKeys { refused: setup.voice == VoiceSetup::KeysRefused }),
        );
    }
    std::thread::spawn(move || pipeline.run(rx));
    tx
}

pub(super) fn run(world: &mut IntegrationWorld) -> &mut NoticesRun {
    world
        .pipeline
        .notices
        .get_or_insert_with(NoticesRun::default)
}

/// The overlay signals in `events`, in order.
pub(super) fn overlay(events: &[Event]) -> Vec<Shown> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Overlay(shown) => Some(shown.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn notices(events: &[Event]) -> Vec<FailureNotice> {
    overlay(events)
        .into_iter()
        .filter_map(|shown| match shown {
            Shown::Notice(notice) => Some(notice),
            _ => None,
        })
        .collect()
}

pub(super) fn pasted(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Pasted(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The one notice the dictation that just ended showed.
pub(super) fn shown_notice(run: &mut NoticesRun) -> Result<FailureNotice> {
    let events = run.wait_until_over()?;
    match &notices(&events)[..] {
        [notice] => Ok(notice.clone()),
        shown => bail!("expected one failure notice, got {shown:?}; events: {events:?}"),
    }
}

pub(super) fn expect_notice(world: &mut IntegrationWorld, expected: FailureNotice) -> Result<()> {
    let notice = shown_notice(run(world))?;
    if notice != expected {
        bail!("expected the notice {expected:?}, got {notice:?}");
    }
    Ok(())
}

/// Shows a notice that speech recognition failed: one hotkey dictation it does not answer.
pub(super) fn show_recognition_notice(run: &mut NoticesRun) -> Result<()> {
    run.setup.recognition = Recognition::Fails;
    run.start_hotkey_dictation()?;
    run.mark_stop();
    run.press_hotkey()?;
    shown_notice(run).map(drop)
}
