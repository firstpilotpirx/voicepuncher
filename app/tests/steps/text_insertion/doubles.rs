//! The stand-ins of the text insertion scenarios.

use super::{
    bail, Arc, AudioBuffer16k, AudioRecorder, ChunkingCfg, CleanedText, Event, FailureNotice,
    Focus, FocusProbe, FocusedElement, Instant, LiveChunk, Mutex, RawTranscript, Receiver, Result,
    SoundPlayer, TextPaster, Transcriber, VisualIndicator, DICTATED, PIPELINE_LAG,
};

pub(crate) struct ScriptedFocus(pub(crate) Arc<Mutex<Focus>>);

impl FocusProbe for ScriptedFocus {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        let element =
            |role: &str, has_text_selection: bool, value_settable: bool, in_web_content: bool| {
                Ok(Some(FocusedElement {
                    role: role.to_owned(),
                    has_text_selection,
                    value_settable,
                    in_web_content,
                }))
            };
        match *self.0.lock().unwrap() {
            Focus::TextField => element("AXTextField", true, true, false),
            Focus::Button => element("AXButton", false, false, false),
            Focus::WebPage => element("AXWebArea", false, false, true),
            Focus::WebPageWithSelectedText => element("AXWebArea", true, false, true),
            Focus::WebTextInput => element("AXTextField", true, true, true),
            Focus::WebEditable => element("AXGroup", true, true, true),
            Focus::TerminalWindow => element("AXWindow", false, false, false),
            Focus::ListWithTextSelection => element("AXList", true, false, false),
            Focus::Unreported => bail!("AX error -25204: cannot complete"),
        }
    }
}

pub(crate) struct OneSecondRecorder(pub(crate) Arc<Mutex<Vec<Event>>>);

impl AudioRecorder for OneSecondRecorder {
    fn start(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::RecordingStarted);
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

pub(crate) struct FixedTranscript;

impl Transcriber for FixedTranscript {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(DICTATED.to_owned())?))
    }
}

pub(crate) struct RecordingPaster(
    pub(crate) Arc<Mutex<Vec<Event>>>,
    pub(crate) Arc<Mutex<Option<Instant>>>,
);

impl TextPaster for RecordingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        self.0
            .lock()
            .unwrap()
            .push(Event::Pasted(text.as_str().to_owned()));
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        *self.1.lock().unwrap() = Some(Instant::now());
        self.0
            .lock()
            .unwrap()
            .push(Event::LeftInClipboard(text.as_str().to_owned()));
        Ok(())
    }
}

pub(crate) struct RecordingIndicator(
    pub(crate) Arc<Mutex<Vec<Event>>>,
    pub(crate) Arc<Mutex<Option<Instant>>>,
);

impl VisualIndicator for RecordingIndicator {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_pasted(&self) {
        self.0.lock().unwrap().push(Event::PastedSignShown);
    }
    fn show_waiting_for_focus(&self) {
        *self.1.lock().unwrap() = Some(Instant::now());
        self.0.lock().unwrap().push(Event::WaitingShown);
    }
    fn hide(&self) {
        std::thread::sleep(PIPELINE_LAG);
        self.0.lock().unwrap().push(Event::Hidden);
    }
    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.0.lock().unwrap().push(Event::FailureNotice(*notice));
    }
    voicepuncher::shows_no_voice_control_signs!();
}

pub(crate) struct Silent;

impl SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
}

pub(crate) fn is_pasted_sign(e: &Event) -> bool {
    matches!(e, Event::PastedSignShown)
}

pub(crate) fn is_pasted(e: &Event) -> bool {
    matches!(e, Event::Pasted(_))
}

pub(crate) fn is_left_in_clipboard(e: &Event) -> bool {
    matches!(e, Event::LeftInClipboard(_))
}
