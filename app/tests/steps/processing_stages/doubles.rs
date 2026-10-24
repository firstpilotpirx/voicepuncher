//! The stand-ins of the processing stages scenarios.

use super::{
    mpsc, record, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText, Event, Events,
    FailureNotice, FocusProbe, FocusedElement, Instant, LiveChunk, Overlay, RawTranscript,
    Receiver, Result, SoundPlayer, TextPaster, Transcriber, VisualIndicator, PIPELINE_LAG,
    SENTENCE, SERVICE_TIME,
};

/// Hands over one second of audio at the stop; recorded in parts, it sends one part while
/// the user speaks and the last one when the recording is flushed.
pub(super) struct PartsRecorder {
    pub(super) events: Events,
    pub(super) in_parts: bool,
    pub(super) parts: Option<mpsc::Sender<LiveChunk>>,
}

impl PartsRecorder {
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
        record(&self.events, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(Self::one_second())
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        if !self.in_parts {
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

pub(super) struct NotingTranscriber {
    pub(super) events: Events,
    pub(super) no_speech: bool,
}

impl Transcriber for NotingTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        record(&self.events, Event::TranscriptionStarted);
        std::thread::sleep(SERVICE_TIME);
        record(&self.events, Event::TranscriptionFinished);
        if self.no_speech {
            return Ok(None);
        }
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
    }
}

pub(super) struct NotingPaster(pub(super) Events);

impl TextPaster for NotingPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        record(&self.0, Event::Pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub(super) struct NotingOverlay {
    pub(super) events: Events,
}

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        std::thread::sleep(PIPELINE_LAG);
        record(&self.events, Event::Overlay(Overlay::Recording));
    }

    fn show_recognizing_speech(&self) {
        record(&self.events, Event::Overlay(Overlay::RecognizingSpeech));
    }

    fn show_pasted(&self) {
        record(&self.events, Event::Overlay(Overlay::PastedSign));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.events, Event::Overlay(Overlay::WaitingForFocus));
    }

    fn hide(&self) {
        record(&self.events, Event::Overlay(Overlay::Hidden));
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        record(
            &self.events,
            Event::Overlay(Overlay::FailureNotice(*notice)),
        );
    }
    voicepuncher::shows_no_voice_control_signs!();
}

/// A button has focus: the text is held back until the wait runs out.
pub(super) struct FocusOnButton;

impl FocusProbe for FocusOnButton {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        Ok(Some(FocusedElement {
            role: "AXButton".to_owned(),
            has_text_selection: false,
            value_settable: false,
            in_web_content: false,
        }))
    }
}

pub(super) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {}
    fn play_listening_off(&self) {}
}
