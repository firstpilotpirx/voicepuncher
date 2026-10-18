use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::application::chunking::merge_chunk_texts;
use crate::core::application::LiveTranscriptView;

/// The live transcript of one recording: shown while the recording runs, never after it
/// ended, though the chunks flushed at its end are still being transcribed.
pub(super) struct LiveTranscript {
    view: Box<dyn LiveTranscriptView>,
    recording: Arc<AtomicBool>,
}

impl LiveTranscript {
    /// The live transcript and the switch that ends it with its recording.
    pub(super) fn new(view: Box<dyn LiveTranscriptView>) -> (Self, RecordingEnd) {
        let recording = Arc::new(AtomicBool::new(true));
        let end = RecordingEnd(Arc::clone(&recording));
        (Self { view, recording }, end)
    }

    /// Shows the chunk texts transcribed so far, stitched as the pasted transcript is.
    pub(super) fn show(&self, texts: &[String]) {
        if self.recording.load(Ordering::SeqCst) {
            self.view
                .show_live_transcript(&merge_chunk_texts(texts.to_vec()));
        }
    }
}

pub(super) struct RecordingEnd(Arc<AtomicBool>);

impl RecordingEnd {
    pub(super) fn signal(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::live_transcript_view::MockLiveTranscriptView;

    #[test]
    fn should_show_nothing_once_the_recording_ended() {
        // Given
        let mut view = MockLiveTranscriptView::new();
        view.expect_show_live_transcript().never();
        let (live_transcript, recording_end) = LiveTranscript::new(Box::new(view));
        // When
        recording_end.signal();
        live_transcript.show(&["the last chunk".to_owned()]);
        // Then the view was never asked to show it
    }
}
