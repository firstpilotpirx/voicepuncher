//! The stand-ins of the dictation pointer scenarios.

use super::{
    bail, record, Arc, AtomicBool, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText,
    DictationSignState, Event, Events, FailureNotice, FocusProbe, FocusedElement, LiveChunk,
    Ordering, Overlay, PointerSignView, RawTranscript, Receiver, Result, Sign, SoundPlayer,
    TextPaster, TextRefiner, Transcriber, VisualIndicator, SENTENCE, SERVICE_TIME,
};
use voicepuncher::core::application::live_transcript_view::{
    HiddenLiveTranscript, LiveTranscriptView,
};
use voicepuncher::core::domain::VoiceCommand;

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

    fn show_refining_text(&self) {
        record(&self.0, Event::Overlay(Overlay::RefiningText));
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

/// Hands over one second of audio at the stop.
pub(super) struct OneSecondRecorder(pub(super) Events);

impl AudioRecorder for OneSecondRecorder {
    fn start(&mut self) -> Result<()> {
        record(&self.0, Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        Ok(AudioBuffer16k::new(vec![
            1_000i16;
            AudioBuffer16k::SAMPLE_RATE_HZ
                as usize
        ]))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}
    voicepuncher::records_whole_buffers_only!();
}

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

pub(super) struct SlowRefiner;

impl TextRefiner for SlowRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        std::thread::sleep(SERVICE_TIME);
        Ok(CleanedText::new(raw.as_str().to_owned())?)
    }
}

pub(super) struct QuietPaster;

impl TextPaster for QuietPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }
}

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

pub(super) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {}
    fn play_listening_off(&self) {}
}
