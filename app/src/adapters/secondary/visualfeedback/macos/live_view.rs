//! The live transcript shown under the pill.

use super::{
    lock, on_main, Arc, LiveTranscriptText, LiveTranscriptView, Mutex, OverlayState, Rc,
    SharedWindows, WindowSet,
};

/// Draws the live transcript under the pill of every screen while the recording runs.
pub(super) struct MacosLiveTranscriptView {
    pub(crate) state: Arc<Mutex<OverlayState>>,
    pub(crate) main: SharedWindows,
    pub(crate) live_transcript: LiveTranscriptText,
}

impl LiveTranscriptView for MacosLiveTranscriptView {
    fn show_live_transcript(&self, text: &str) {
        self.live_transcript.update(*lock(&self.state), text);
        let state = Arc::clone(&self.state);
        let main = self.main.clone();
        let live_transcript = self.live_transcript.clone();
        on_main(move |mtm| {
            // WHY: read on the main queue, so a stage shown after this update was queued wins
            let state = *lock(&state);
            if LiveTranscriptText::draws_text_window(state) {
                apply_live_transcript(&main.windows(mtm), live_transcript.shown(state).as_ref());
            }
        });
    }
}

pub(super) fn apply_live_transcript(windows: &[Rc<WindowSet>], shown: Option<&String>) {
    for ws in windows {
        ws.show_live_transcript(shown.map(std::string::String::as_str));
    }
}
