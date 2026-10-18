/// Shows the live transcript while the user dictates; handed to the thread that
/// transcribes the recording's chunks.
#[cfg_attr(test, mockall::automock)]
pub trait LiveTranscriptView: Send {
    fn show_live_transcript(&self, text: &str);
}

/// A view for an overlay that does not show the live transcript.
pub struct HiddenLiveTranscript;

impl LiveTranscriptView for HiddenLiveTranscript {
    fn show_live_transcript(&self, _text: &str) {}
}
