//! The dictation sign next to the mouse pointer, driven by the overlay's signals.

use crate::core::application::live_transcript_view::LiveTranscriptView;
use crate::core::application::VisualIndicator;
use crate::core::domain::{FailureNotice, VoiceCommand};

/// The window that shows the dictation sign next to the pointer.
pub trait PointerSignView: Send {
    fn show(&self);
    fn hide(&self);
}

/// The overlay, plus the dictation sign while a recording runs: shown with the recording,
/// gone with the signals that end one; the others leave it as it is.
pub struct PointerSignIndicator<V, S> {
    overlay: V,
    sign: S,
}

impl<V: VisualIndicator, S: PointerSignView> PointerSignIndicator<V, S> {
    pub fn new(overlay: V, sign: S) -> Self {
        Self { overlay, sign }
    }
}

impl<V: VisualIndicator, S: PointerSignView> VisualIndicator for PointerSignIndicator<V, S> {
    fn show_recording(&self) {
        self.sign.show();
        self.overlay.show_recording();
    }

    fn show_recognizing_speech(&self) {
        self.sign.hide();
        self.overlay.show_recognizing_speech();
    }

    fn show_refining_text(&self) {
        self.sign.hide();
        self.overlay.show_refining_text();
    }

    fn show_waiting_for_focus(&self) {
        self.sign.hide();
        self.overlay.show_waiting_for_focus();
    }

    fn hide(&self) {
        self.sign.hide();
        self.overlay.hide();
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.overlay.show_failure_notice(notice);
    }

    fn show_command_sign(&self, command: VoiceCommand) {
        self.overlay.show_command_sign(command);
    }

    fn show_commands_unavailable(&self) {
        self.overlay.show_commands_unavailable();
    }

    fn hide_commands_unavailable(&self) {
        self.overlay.hide_commands_unavailable();
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        self.overlay.live_transcript_view()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::core::application::visual_indicator::MockVisualIndicator;

    #[derive(Clone, Default)]
    struct Sign(Arc<Mutex<Vec<&'static str>>>);

    impl PointerSignView for Sign {
        fn show(&self) {
            self.0.lock().unwrap().push("show");
        }

        fn hide(&self) {
            self.0.lock().unwrap().push("hide");
        }
    }

    fn quiet_overlay() -> MockVisualIndicator {
        let mut overlay = MockVisualIndicator::new();
        overlay.expect_show_command_sign().return_const(());
        overlay.expect_show_failure_notice().return_const(());
        overlay
    }

    #[test]
    fn should_leave_the_sign_alone_on_a_command_sign_or_a_failure_notice() {
        let sign = Sign::default();
        let indicator = PointerSignIndicator::new(quiet_overlay(), sign.clone());
        indicator.show_command_sign(VoiceCommand::PressEnter);
        indicator.show_failure_notice(&FailureNotice::VoiceCommandsUnavailable);
        assert!(sign.0.lock().unwrap().is_empty());
    }

    #[test]
    fn should_pass_the_commands_unavailable_sign_to_the_overlay_and_leave_the_sign_alone() {
        // Given
        let mut overlay = MockVisualIndicator::new();
        overlay
            .expect_show_commands_unavailable()
            .times(1)
            .return_const(());
        overlay
            .expect_hide_commands_unavailable()
            .times(1)
            .return_const(());
        let sign = Sign::default();
        let indicator = PointerSignIndicator::new(overlay, sign.clone());
        // When
        indicator.show_commands_unavailable();
        indicator.hide_commands_unavailable();
        // Then
        assert!(sign.0.lock().unwrap().is_empty());
    }
}
