//! The overlay of file mode: nothing is shown.

use crate::core::application::live_transcript_view::{HiddenLiveTranscript, LiveTranscriptView};
use crate::core::application::VisualIndicator;
use crate::core::domain::{FailureNotice, VoiceCommand};

/// An overlay that shows nothing: file mode has no screen to show a dictation on.
pub struct QuietVisualIndicator;

impl VisualIndicator for QuietVisualIndicator {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
    fn show_command_sign(&self, _command: VoiceCommand) {}
    fn show_commands_unavailable(&self) {}
    fn hide_commands_unavailable(&self) {}
    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(HiddenLiveTranscript)
    }
}
