use std::sync::{Arc, Mutex};

use super::OverlayState;

/// Characters of the live transcript the text window shows: two lines of the 14 pt system
/// font in its width.
pub(super) const LIVE_TRANSCRIPT_CHARS: usize = 110;
/// Marks a live transcript whose earlier words are left out.
const LEFT_OUT: &str = "…";

/// The latest whole words of `text` that fit `budget` characters, after `…` when earlier
/// words are left out; a single word longer than the budget keeps its end.
pub(super) fn latest_words(text: &str, budget: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= budget {
        return text.to_owned();
    }
    let room = budget.saturating_sub(LEFT_OUT.chars().count());
    let mut kept: Vec<&str> = Vec::new();
    let mut used = 0usize;
    for word in text.split_whitespace().rev() {
        let needed = word.chars().count() + usize::from(!kept.is_empty());
        if used + needed > room {
            break;
        }
        used += needed;
        kept.push(word);
    }
    if kept.is_empty() {
        let tail: String = text
            .chars()
            .rev()
            .take(room)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return format!("{LEFT_OUT}{tail}");
    }
    kept.reverse();
    format!("{LEFT_OUT}{}", kept.join(" "))
}

/// The live transcript of the running recording, shared by the overlay and its view.
#[derive(Clone, Default)]
pub(super) struct LiveTranscriptText(Arc<Mutex<String>>);

impl LiveTranscriptText {
    /// Forgets the text of an earlier recording.
    pub(super) fn clear(&self) {
        self.0.lock().unwrap().clear();
    }

    /// Keeps `text` while the overlay shows the recording; an update that arrives after
    /// the recording ended is dropped.
    pub(super) fn update(&self, state: OverlayState, text: &str) {
        if state == OverlayState::Recording {
            *self.0.lock().unwrap() = text.to_owned();
        }
    }

    /// Whether the live transcript decides what the text window shows in `state`: only while
    /// recording; every other state puts its own text there.
    pub(super) fn draws_text_window(state: OverlayState) -> bool {
        state == OverlayState::Recording
    }

    /// What the text window shows in `state`; `None` hides it.
    pub(super) fn shown(&self, state: OverlayState) -> Option<String> {
        let text = self.0.lock().unwrap();
        (state == OverlayState::Recording && !text.trim().is_empty())
            .then(|| latest_words(&text, LIVE_TRANSCRIPT_CHARS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{FailingPart, FailureNotice};

    #[test]
    fn should_keep_a_text_that_fits() {
        assert_eq!(latest_words("  one two three ", 20), "one two three");
    }

    #[test]
    fn should_keep_the_latest_whole_words_after_an_ellipsis() {
        assert_eq!(latest_words("one two three four five", 12), "…four five");
    }

    #[test]
    fn should_keep_the_end_of_a_word_longer_than_the_budget() {
        assert_eq!(latest_words("one abcdefghijklmnop", 6), "…lmnop");
    }

    fn recording_with(text: &str) -> LiveTranscriptText {
        let live = LiveTranscriptText::default();
        live.update(OverlayState::Recording, text);
        live
    }

    #[test]
    fn should_show_the_text_while_recording() {
        let live = recording_with("we moved the meeting");
        assert_eq!(
            live.shown(OverlayState::Recording).as_deref(),
            Some("we moved the meeting")
        );
    }

    #[test]
    fn should_show_nothing_outside_the_recording() {
        let live = recording_with("we moved the meeting");
        for state in [
            OverlayState::RecognizingSpeech,
            OverlayState::RefiningText,
            OverlayState::WaitingForFocus,
            OverlayState::Hidden,
        ] {
            assert_eq!(live.shown(state), None, "{state:?}");
        }
    }

    #[test]
    fn should_leave_the_text_window_to_a_failure_notice() {
        let notice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
        assert!(!LiveTranscriptText::draws_text_window(
            OverlayState::FailureNotice(notice)
        ));
    }

    #[test]
    fn should_draw_the_text_window_while_recording() {
        assert!(LiveTranscriptText::draws_text_window(
            OverlayState::Recording
        ));
    }

    #[test]
    fn should_leave_the_text_window_to_the_refinement_preview_and_the_wait() {
        for state in [
            OverlayState::RecognizingSpeech,
            OverlayState::RefiningText,
            OverlayState::WaitingForFocus,
            OverlayState::Hidden,
        ] {
            assert!(!LiveTranscriptText::draws_text_window(state), "{state:?}");
        }
    }

    #[test]
    fn should_show_nothing_once_a_new_recording_starts() {
        let live = recording_with("an earlier dictation");
        live.clear();
        assert_eq!(live.shown(OverlayState::Recording), None);
    }

    #[test]
    fn should_drop_an_update_that_arrives_after_the_recording_ended() {
        let live = LiveTranscriptText::default();
        live.update(OverlayState::RecognizingSpeech, "the last chunk");
        assert_eq!(live.shown(OverlayState::Recording), None);
    }
}
