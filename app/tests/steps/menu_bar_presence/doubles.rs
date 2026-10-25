//! The stand-ins of the menu bar presence scenarios.

use super::{
    bail, pipeline_config, set, Arc, AudioBuffer16k, AudioRecorder, CleanedText, Config, Duration,
    Flag, FocusProbe, FocusWaitCfg, FocusedElement, HotkeyBinding, HotkeyListener, JoinHandle,
    Mutex, NoKeys, Parts, Pipeline, PipelineAssembler, PipelineInput, PipelineParts, PttSignal,
    QuietSpotter, RawTranscript, Receiver, Result, Sender, SharedVisualIndicator, SoundPlayer,
    TextPaster, Transcriber, VisualIndicator, DICTATED, FOCUS_POLL, POLL, SILENT_SERVICE_ERROR,
};

/// Builds every pipeline from stand-ins over the overlay the status item decorates.
pub(super) struct StandInAssembler {
    pub(super) parts: Parts,
    pub(super) overlay: SharedVisualIndicator,
    /// Holds the first assembly until the scenario says whether the services answer.
    pub(super) gate: Option<Receiver<bool>>,
}

impl PipelineAssembler for StandInAssembler {
    fn assemble(&mut self, config: &Config, inputs: Sender<PipelineInput>) -> Result<Pipeline> {
        let answer = match self.gate.take() {
            Some(gate) => gate.recv().unwrap_or(false),
            None => *self.parts.services_answer.lock().unwrap(),
        };
        if !answer {
            bail!("{SILENT_SERVICE_ERROR}");
        }
        *self.parts.inputs.lock().unwrap() = Some(inputs);
        let mut pipeline = Pipeline::new(
            PipelineParts {
                recorder: Box::new(StandInRecorder),
                transcriber: Box::new(StandInTranscriber(self.parts.clone())),
                paster: Box::new(StandInPaster(Arc::clone(&self.parts.pasted))),
                sound_player: Box::new(StandInSounds(Arc::clone(&self.parts.cancel_played))),
                visual_indicator: Box::new(self.overlay.clone()),
            },
            pipeline_config(config),
        )
        .with_focus_wait(
            Box::new(ScriptedFocus(Arc::clone(&self.parts.text_field_focused))),
            FocusWaitCfg {
                timeout: Duration::from_secs(config.paste.focus_wait_seconds),
                poll_interval: FOCUS_POLL,
            },
        );
        if config.voice_control.enabled {
            pipeline = pipeline.with_voice_control(Box::new(QuietSpotter), Box::new(NoKeys));
        }
        Ok(pipeline)
    }
}

pub(super) struct StandInRecorder;

impl AudioRecorder for StandInRecorder {
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    voicepuncher::hands_over_one_second!();
}

/// Answers with the dictated sentence, fails when the service is down, and waits while held.
pub(super) struct StandInTranscriber(pub(super) Parts);

impl Transcriber for StandInTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        while *self.0.transcription_held.lock().unwrap() {
            std::thread::sleep(POLL);
        }
        if *self.0.transcription_fails.lock().unwrap() {
            bail!("{SILENT_SERVICE_ERROR}");
        }
        Ok(Some(RawTranscript::new(DICTATED.to_owned())?))
    }
}

pub(super) struct StandInPaster(pub(super) Flag);

impl TextPaster for StandInPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        set(&self.0, true);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }
}

pub(super) struct StandInSounds(pub(super) Flag);

impl SoundPlayer for StandInSounds {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {
        set(&self.0, true);
    }
}

pub(super) struct QuietOverlay;

impl VisualIndicator for QuietOverlay {
    voicepuncher::shows_no_dictation_states!();
    voicepuncher::shows_no_voice_control_signs!();
}

/// A focus a scenario switches between a text field and a button.
pub(crate) struct ScriptedFocus(pub(crate) Flag);

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

/// The keyboard the runtime's listener reads.
#[derive(Clone, Default)]
pub(crate) struct FakeKeyboard {
    pub(super) wired: Arc<Mutex<Option<Sender<PttSignal>>>>,
}

impl FakeKeyboard {
    pub(super) fn signal(&self, signal: PttSignal) {
        if let Some(tx) = self.wired.lock().unwrap().as_ref() {
            let _ = tx.send(signal);
        }
    }
}

impl HotkeyListener for FakeKeyboard {
    fn spawn(
        self: Box<Self>,
        _binding: HotkeyBinding,
        tx: Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        *self.wired.lock().unwrap() = Some(tx);
        Ok(std::thread::spawn(|| {}))
    }
}
