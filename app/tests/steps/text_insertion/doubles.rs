//! The stand-ins of the text insertion scenarios.

use super::{
    bail, Arc, AudioBuffer16k, CleanedText, Event, FailureNotice, Focus, FocusProbe,
    FocusedElement, Instant, Mutex, RawTranscript, Result, TextPaster, Transcriber,
    VisualIndicator, DICTATED, PIPELINE_LAG,
};

pub(crate) struct ScriptedFocus(pub(crate) Arc<Mutex<Focus>>);

impl FocusProbe for ScriptedFocus {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        let focus = *self.0.lock().unwrap();
        let Some((role, has_text_selection, value_settable, in_web_content)) = reported(focus)
        else {
            bail!("AX error -25204: cannot complete");
        };
        Ok(Some(FocusedElement {
            role: role.to_owned(),
            has_text_selection,
            value_settable,
            in_web_content,
        }))
    }
}

/// The role, text selection, settable value and web content `focus` reports; none when it
/// reports nothing.
const fn reported(focus: Focus) -> Option<(&'static str, bool, bool, bool)> {
    Some(match focus {
        Focus::TextField => ("AXTextField", true, true, false),
        Focus::Button => ("AXButton", false, false, false),
        Focus::WebPage => ("AXWebArea", false, false, true),
        Focus::WebPageWithSelectedText => ("AXWebArea", true, false, true),
        Focus::WebTextInput => ("AXTextField", true, true, true),
        Focus::WebEditable => ("AXGroup", true, true, true),
        Focus::TerminalWindow => ("AXWindow", false, false, false),
        Focus::ListWithTextSelection => ("AXList", true, false, false),
        Focus::Unreported => return None,
    })
}

pub(crate) type OneSecondRecorder = voicepuncher::test_doubles::OneSecondRecorder<Event>;

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
        self.0
            .lock()
            .unwrap()
            .push(Event::FailureNotice(notice.clone()));
    }
    voicepuncher::shows_no_voice_control_signs!();
}

pub(crate) use voicepuncher::test_doubles::Silent;

pub(crate) fn is_pasted_sign(e: &Event) -> bool {
    matches!(e, Event::PastedSignShown)
}

pub(crate) fn is_pasted(e: &Event) -> bool {
    matches!(e, Event::Pasted(_))
}

pub(crate) fn is_left_in_clipboard(e: &Event) -> bool {
    matches!(e, Event::LeftInClipboard(_))
}
