//! The stand-ins of the processing stages scenarios.

use super::{
    record, AudioBuffer16k, AudioRecorder, ChunkingCfg, Event, Events, FailureNotice, FocusProbe,
    FocusedElement, LiveChunk, Overlay, RawTranscript, Receiver, Result, Transcriber,
    VisualIndicator, PIPELINE_LAG, SENTENCE, SERVICE_TIME,
};

/// Hands over one second of audio at the stop; recorded in parts, it sends one part while
/// the user speaks and the last one when the recording is flushed.
pub(super) struct PartsRecorder {
    pub(super) events: Events,
    pub(super) in_parts: bool,
    pub(super) parts: voicepuncher::test_doubles::OneSecondParts,
}

impl AudioRecorder for PartsRecorder {
    fn start(&mut self) -> Result<()> {
        record(&self.events, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(voicepuncher::test_doubles::one_second_of_sound())
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(self.in_parts.then(|| self.parts.start()))
    }

    fn flush_live_chunks(&mut self) {
        self.parts.flush();
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

pub(super) type NotingPaster = voicepuncher::test_doubles::NotingPaster<Event>;

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
            Event::Overlay(Overlay::FailureNotice(notice.clone())),
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

pub(super) use voicepuncher::test_doubles::Silent;
