//! One overlay for the life of the process: every pipeline gets a forwarding indicator over
//! the shared one, which stays alive across restarts.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::core::application::live_transcript_view::LiveTranscriptView;
use crate::core::application::VisualIndicator;
use crate::core::domain::{FailureNotice, VoiceCommand};

#[derive(Clone)]
pub struct SharedVisualIndicator {
    inner: Arc<Mutex<Box<dyn VisualIndicator>>>,
}

impl SharedVisualIndicator {
    pub fn new(indicator: Box<dyn VisualIndicator>) -> Self {
        Self { inner: Arc::new(Mutex::new(indicator)) }
    }

    fn overlay(&self) -> MutexGuard<'_, Box<dyn VisualIndicator>> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl VisualIndicator for SharedVisualIndicator {
    fn show_recording(&self) {
        self.overlay().show_recording();
    }

    fn show_recognizing_speech(&self) {
        self.overlay().show_recognizing_speech();
    }

    fn show_waiting_for_focus(&self) {
        self.overlay().show_waiting_for_focus();
    }

    fn show_pasted(&self) {
        self.overlay().show_pasted();
    }

    fn hide(&self) {
        self.overlay().hide();
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.overlay().show_failure_notice(notice);
    }

    fn show_command_sign(&self, command: VoiceCommand) {
        self.overlay().show_command_sign(command);
    }

    fn show_commands_unavailable(&self) {
        self.overlay().show_commands_unavailable();
    }

    fn hide_commands_unavailable(&self) {
        self.overlay().hide_commands_unavailable();
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        self.overlay().live_transcript_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::live_transcript_view::MockLiveTranscriptView;
    use crate::core::application::visual_indicator::MockVisualIndicator;

    #[test]
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn should_forward_every_call_to_the_shared_overlay() {
        // Given
        let mut overlay = MockVisualIndicator::new();
        overlay.expect_show_recording().once().return_const(());
        overlay
            .expect_show_recognizing_speech()
            .once()
            .return_const(());
        overlay
            .expect_show_waiting_for_focus()
            .once()
            .return_const(());
        overlay.expect_show_pasted().once().return_const(());
        overlay.expect_hide().once().return_const(());
        overlay
            .expect_show_failure_notice()
            .withf(|notice| *notice == FailureNotice::NotPasted(None))
            .once()
            .return_const(());
        overlay
            .expect_show_command_sign()
            .withf(|command| *command == VoiceCommand::PressEnter)
            .once()
            .return_const(());
        overlay
            .expect_show_commands_unavailable()
            .once()
            .return_const(());
        overlay
            .expect_hide_commands_unavailable()
            .once()
            .return_const(());
        overlay
            .expect_live_transcript_view()
            .once()
            .returning(|| Box::new(MockLiveTranscriptView::new()));
        let shared = SharedVisualIndicator::new(Box::new(overlay));
        let forwarding: Box<dyn VisualIndicator> = Box::new(shared.clone());
        // When
        forwarding.show_recording();
        forwarding.show_recognizing_speech();
        forwarding.show_waiting_for_focus();
        forwarding.show_pasted();
        forwarding.hide();
        forwarding.show_failure_notice(&FailureNotice::NotPasted(None));
        forwarding.show_command_sign(VoiceCommand::PressEnter);
        forwarding.show_commands_unavailable();
        forwarding.hide_commands_unavailable();
        let _view = forwarding.live_transcript_view();
        // Then — the mock checks every call once when it drops with `shared`
        drop(forwarding);
        drop(shared);
    }
}
