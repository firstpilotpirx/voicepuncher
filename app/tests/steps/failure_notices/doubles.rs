//! The stand-ins of the failure notices scenarios.

use super::{
    bail, mpsc, record, AccessibilityMissing, AudioBuffer16k, AudioRecorder, ChunkingCfg,
    CleanedText, CommandSpotter, Event, Events, FailureNotice, Instant, KeyPresser, LiveChunk,
    RawTranscript, Receiver, Recognition, Result, Setup, Shown, SoundPlayer, TextPaster,
    Transcriber, VisualIndicator, VoiceCommand, RAW_TRANSCRIPT,
};

/// Hands over one second of audio at the stop; for a long dictation it sends one part while
/// the user speaks and the last one when the recording is flushed.
pub(super) struct PartsRecorder {
    pub(super) events: Events,
    pub(super) setup: Setup,
    pub(super) parts: Option<mpsc::Sender<LiveChunk>>,
}

impl PartsRecorder {
    pub(super) fn new(events: Events, setup: Setup) -> Self {
        Self { events, setup, parts: None }
    }

    pub(super) fn one_second() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![1_000i16; AudioBuffer16k::SAMPLE_RATE_HZ as usize])
    }

    pub(super) fn send_part(&self) {
        if let Some(parts) = &self.parts {
            let _ =
                parts.send(LiveChunk { audio: Self::one_second(), enqueued_at: Instant::now() });
        }
    }
}

impl AudioRecorder for PartsRecorder {
    fn start(&mut self) -> Result<()> {
        if self.setup.microphone_fails {
            bail!("the microphone cannot be opened");
        }
        record(&self.events, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        if self.setup.nothing_recorded {
            return Ok(AudioBuffer16k::new(Vec::new()));
        }
        Ok(Self::one_second())
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        if !self.setup.long_dictation {
            return Ok(None);
        }
        let (tx, rx) = mpsc::channel();
        self.parts = Some(tx);
        self.send_part();
        Ok(Some(rx))
    }

    fn flush_live_chunks(&mut self) {
        self.send_part();
        // the transcription of the parts ends with the last one
        self.parts = None;
    }
    voicepuncher::records_whole_buffers_only!();
}

pub(super) struct ScriptedTranscriber {
    pub(super) recognition: Recognition,
    pub(super) calls: usize,
}

impl ScriptedTranscriber {
    pub(super) fn new(recognition: Recognition) -> Self {
        Self { recognition, calls: 0 }
    }
}

impl Transcriber for ScriptedTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        self.calls += 1;
        let fails = match self.recognition {
            Recognition::Answers => false,
            Recognition::Fails => true,
            Recognition::FailsOnFirstPart => self.calls == 1,
        };
        if fails {
            bail!("connection refused");
        }
        Ok(Some(RawTranscript::new(RAW_TRANSCRIPT.to_owned())?))
    }
}

pub(super) struct ScriptedPaster {
    pub(super) events: Events,
    pub(super) fails: bool,
}

impl TextPaster for ScriptedPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        if self.fails {
            bail!("the paste was refused");
        }
        record(&self.events, Event::Pasted(text.as_str().to_owned()));
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub(super) struct NotingOverlay(pub(super) Events);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        record(&self.0, Event::Overlay(Shown::Recording));
    }

    fn show_recognizing_speech(&self) {
        record(&self.0, Event::Overlay(Shown::Stage));
    }

    fn show_pasted(&self) {
        record(&self.0, Event::Overlay(Shown::PastedSign));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.0, Event::Overlay(Shown::Stage));
    }

    fn hide(&self) {
        record(&self.0, Event::Overlay(Shown::Hidden));
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        record(&self.0, Event::Overlay(Shown::Notice(*notice)));
    }
    voicepuncher::shows_no_voice_control_signs!();
}

/// The command spotting service as the pipeline sees it; the scenario feeds the phrases it
/// reports straight into the pipeline's input channel.
pub(super) struct QuietSpotter {
    /// The service is not running: listening cannot start.
    pub(super) down: bool,
}

impl CommandSpotter for QuietSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        if self.down {
            bail!("connection refused");
        }
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {}
    voicepuncher::holds_no_heard_sound!();
}

/// Presses keys, or is refused them the way macOS refuses an app without the
/// Accessibility permission.
pub(super) struct ScriptedKeys {
    pub(super) refused: bool,
}

impl ScriptedKeys {
    pub(super) fn press(&self) -> Result<()> {
        if self.refused {
            return Err(AccessibilityMissing.into());
        }
        Ok(())
    }
}

impl KeyPresser for ScriptedKeys {
    fn press_enter(&mut self) -> Result<()> {
        self.press()
    }

    fn clear_all_text(&mut self) -> Result<()> {
        self.press()
    }

    fn paste(&mut self) -> Result<()> {
        self.press()
    }
}

pub(super) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}
