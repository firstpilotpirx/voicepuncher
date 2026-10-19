use crate::core::application::live_transcript_view::{HiddenLiveTranscript, LiveTranscriptView};
use crate::core::domain::{FailureNotice, VoiceCommand};

/// The overlay: what the app is doing with a dictation, from the recording to the paste.
#[cfg_attr(test, mockall::automock)]
pub trait VisualIndicator: Send {
    fn show_recording(&self);
    /// Processing stage: speech recognition runs on the recording after it stopped.
    fn show_recognizing_speech(&self);
    /// Processing stage: text refinement corrects the transcript.
    fn show_refining_text(&self);
    fn show_waiting_for_focus(&self);
    fn hide(&self);
    /// Failure notice: an action the user started failed; shown in place of hiding the overlay.
    fn show_failure_notice(&self, notice: &FailureNotice);
    /// Command sign: names a voice command with no visible effect of its own ("press enter",
    /// "clear all text", "restart dictation") for about a second, then the overlay shows again
    /// what it showed before. Any later signal replaces it at once.
    fn show_command_sign(&self, _command: VoiceCommand) {}
    /// Commands-unavailable sign: voice commands cannot be heard for now. It shows beside
    /// whatever the overlay shows, and alone while the overlay is hidden, until hidden again.
    fn show_commands_unavailable(&self) {}
    /// Takes the commands-unavailable sign away; the overlay shows its state without it.
    fn hide_commands_unavailable(&self) {}
    /// A view of the live transcript that shows it while the recording runs.
    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(HiddenLiveTranscript)
    }
}
