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
}
