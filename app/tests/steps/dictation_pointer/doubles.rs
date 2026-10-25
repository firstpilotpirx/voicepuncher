//! The stand-ins of the dictation pointer scenarios.

use super::{
    bail, record, Arc, AtomicBool, AudioBuffer16k, DictationSignState, Event, Events,
    FailureNotice, FocusProbe, FocusedElement, Ordering, Overlay, PointerSignView, RawTranscript,
    Result, Sign, Transcriber, VisualIndicator, SENTENCE, SERVICE_TIME,
};
use voicepuncher::core::application::live_transcript_view::{
    HiddenLiveTranscript, LiveTranscriptView,
};
use voicepuncher::test_doubles::prelude::*;

/// The dictation sign's window as the adapter sees it.
pub(super) struct NotingSign(pub(super) Events);

impl PointerSignView for NotingSign {
    fn show(&self, badge: DictationSignState) {
        record(&self.0, Event::Sign(Sign::Shown(badge)));
    }

    fn hide(&self) {
        record(&self.0, Event::Sign(Sign::Gone));
    }
}

pub(super) struct NotingOverlay(pub(super) Events);

impl VisualIndicator for NotingOverlay {
    fn show_recording(&self) {
        record(&self.0, Event::Overlay(Overlay::Recording));
    }

    fn show_recognizing_speech(&self) {
        record(&self.0, Event::Overlay(Overlay::RecognizingSpeech));
    }

    fn show_pasted(&self) {
        record(&self.0, Event::Overlay(Overlay::PastedSign));
    }

    fn show_waiting_for_focus(&self) {
        record(&self.0, Event::Overlay(Overlay::WaitingForFocus));
    }

    fn hide(&self) {
        record(&self.0, Event::Overlay(Overlay::Hidden));
    }

    fn show_failure_notice(&self, _notice: &FailureNotice) {
        record(&self.0, Event::Overlay(Overlay::FailureNotice));
    }

    fn show_commands_unavailable(&self) {
        record(&self.0, Event::Overlay(Overlay::CommandsUnavailable));
    }
    fn show_command_sign(&self, _command: VoiceCommand) {}
    fn hide_commands_unavailable(&self) {}
    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(HiddenLiveTranscript)
    }
}

pub(super) type OneSecondRecorder = voicepuncher::test_doubles::OneSecondRecorder<Event>;

/// The speech recognition service: answers after a moment, or refuses the connection when
/// it is down.
pub(super) struct SlowTranscriber {
    pub(super) down: bool,
}

impl Transcriber for SlowTranscriber {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        std::thread::sleep(SERVICE_TIME);
        if self.down {
            bail!("connection refused");
        }
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
    }
}

pub(super) use voicepuncher::test_doubles::DiscardingPaster as QuietPaster;

/// A text field has keyboard focus, or a button while the flag is set.
pub(super) struct SwitchableFocus(pub(super) Arc<AtomicBool>);

impl FocusProbe for SwitchableFocus {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        let button = self.0.load(Ordering::SeqCst);
        Ok(Some(FocusedElement {
            role: if button { "AXButton" } else { "AXTextField" }.to_owned(),
            has_text_selection: !button,
            value_settable: !button,
            in_web_content: false,
        }))
    }
}

pub(super) use voicepuncher::test_doubles::Silent;
