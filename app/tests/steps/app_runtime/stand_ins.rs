//! The stand-in pipeline parts the runtime assembles.

use super::{
    bail, failures, pipeline_config, push, Arc, AudioBuffer16k, AudioRecorder, Config, Duration,
    Event, Events, FocusWaitCfg, Generation, Mutex, NoKeys, NoticeOverlay, Pipeline,
    PipelineAssembler, PipelineInput, PipelineParts, RawTranscript, Result, ScriptedFocus, Sender,
    StandInPaster, StandInSounds, StandInSpotter, Transcriber, DICTATED, FOCUS_POLL,
    SILENT_SERVICE_ERROR,
};

/// Builds every pipeline from stand-ins that log under the pipeline's generation.
pub(super) struct StandInAssembler {
    pub(crate) events: Events,
    pub(crate) services_answer: Arc<Mutex<bool>>,
    pub(crate) text_field_focused: Arc<Mutex<bool>>,
    pub(crate) panics: failures::PanicPlan,
    pub(crate) generation: Generation,
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
            Event::Configured(generation, config.sounds.start_sound.clone()),
        );
        let part = || Part { events: Arc::clone(&self.events), generation };
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(StandInRecorder(part(), Arc::clone(&self.panics))),
                transcriber: Box::new(FixedTranscript),
                paster: Box::new(StandInPaster(part())),
                sound_player: Box::new(StandInSounds(part())),
                visual_indicator: Box::new(NoticeOverlay(part())),
            },
            pipeline_config(config),
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
pub(super) struct Part {
    pub(crate) events: Events,
    pub(crate) generation: Generation,
}

impl Part {
    pub(crate) fn log(&self, event: fn(Generation) -> Event) {
        push(&self.events, event(self.generation));
    }
}

pub(super) struct StandInRecorder(Part, failures::PanicPlan);

impl AudioRecorder for StandInRecorder {
    fn start(&mut self) -> Result<()> {
        if let Some(message) = failures::panic_of(&self.1, self.0.generation) {
            panic!("{message}");
        }
        self.0.log(Event::RecordingStarted);
        Ok(())
    }

    voicepuncher::hands_over_one_second!();
}

/// The recorder lives as long as its pipeline: its drop is the pipeline's end.
impl Drop for StandInRecorder {
    fn drop(&mut self) {
        self.0.log(Event::PipelineDropped);
    }
}

pub(super) struct FixedTranscript;

impl Transcriber for FixedTranscript {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(DICTATED.to_owned())?))
    }
}
