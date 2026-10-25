use crate::core::application::DictationOutcome;
use crate::core::domain::{
    AccessibilityMissing, FailingPart, FailureNotice, ServicePortTaken, VoiceCommand,
};

use super::dictation::Dictation;
use super::Pipeline;

/// The recorder did not start; its failure notice is already shown.
#[derive(Debug, thiserror::Error)]
#[error("the recording did not start")]
pub(super) struct RecordingNotStarted;

/// How much of a dictation a pending notice says was lost, least first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Gravity {
    /// Shown at once, never kept for the end of a cycle.
    ShownAtOnce,
    PartLost,
    NotPasted,
    Lost,
}

fn gravity(notice: &FailureNotice) -> Gravity {
    match notice {
        FailureNotice::PartOfDictationLost(_) => Gravity::PartLost,
        FailureNotice::NotPasted(_) => Gravity::NotPasted,
        FailureNotice::DictationLost(_) => Gravity::Lost,
        FailureNotice::RecordingNotStarted
        | FailureNotice::CommandFailed(..)
        | FailureNotice::VoiceCommandsUnavailable
        | FailureNotice::PermissionRevoked(_) => Gravity::ShownAtOnce,
    }
}

/// The notice a cycle ends with once `notice` applies too: the graver of the two.
pub(super) fn graver(pending: Option<FailureNotice>, notice: FailureNotice) -> FailureNotice {
    match pending {
        Some(pending) if gravity(&pending) >= gravity(&notice) => pending,
        _ => notice,
    }
}

/// The part a speech recognition `error` failed on: the program on the service's own port
/// when one holds it.
pub(super) fn recognition_part(error: &anyhow::Error) -> FailingPart {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<ServicePortTaken>())
        .map_or(FailingPart::SpeechRecognition, |taken| {
            FailingPart::ServicePortTaken(taken.0.clone())
        })
}

/// The notice of a paste that failed with `error`: the Accessibility permission when it
/// is missing, no part otherwise.
pub(super) fn not_pasted(error: &anyhow::Error) -> FailureNotice {
    FailureNotice::NotPasted(accessibility_if_missing(error))
}

/// The notice of a voice command that failed with `error`: the Accessibility permission
/// when it is missing, no part otherwise.
pub(super) fn command_failed(command: VoiceCommand, error: &anyhow::Error) -> FailureNotice {
    FailureNotice::CommandFailed(command, accessibility_if_missing(error))
}

/// The Accessibility permission when `error` holds its absence.
fn accessibility_if_missing(error: &anyhow::Error) -> Option<FailingPart> {
    error
        .chain()
        .any(<(dyn std::error::Error + 'static)>::is::<AccessibilityMissing>)
        .then_some(FailingPart::Accessibility)
}

impl Pipeline {
    /// Ends `dictation` on the overlay: its pending failure notice, or hidden without one.
    pub(super) fn end_cycle_on_overlay(&self, dictation: &mut Dictation) {
        match dictation.pending_notice.take() {
            Some(notice) => self.visual_indicator.show_failure_notice(&notice),
            None => self.visual_indicator.hide(),
        }
    }

    /// Ends `dictation`, whose text was delivered, on the overlay: its pending failure
    /// notice, the Pasted sign when the text was pasted, or hidden.
    pub(super) fn end_delivery_on_overlay(
        &self,
        dictation: &mut Dictation,
        outcome: DictationOutcome,
    ) {
        if outcome == DictationOutcome::Pasted && dictation.pending_notice.is_none() {
            self.visual_indicator.show_pasted();
        } else {
            self.end_cycle_on_overlay(dictation);
        }
    }

    /// Shows `notice` at once: the failure happened outside a dictation.
    pub(super) fn show_failure_at_once(&self, notice: &FailureNotice) {
        log::warn!("failure notice: {}", notice.message());
        self.visual_indicator.show_failure_notice(notice);
    }

    /// Shows at once that the recorder did not start.
    pub(super) fn recording_not_started(&self, error: anyhow::Error) -> anyhow::Error {
        log::error!("recording not started: {error:#}");
        self.visual_indicator
            .show_failure_notice(&FailureNotice::RecordingNotStarted);
        error.context(RecordingNotStarted)
    }
}

/// Lets a recording that did not start pass: its notice is shown and the app goes on.
pub(super) fn past_recording_not_started(result: anyhow::Result<()>) -> anyhow::Result<()> {
    match result {
        Err(e) if e.is::<RecordingNotStarted>() => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECOGNITION: FailingPart = FailingPart::SpeechRecognition;

    #[test]
    fn should_keep_the_first_notice_of_a_cycle() {
        assert_eq!(
            graver(None, FailureNotice::PartOfDictationLost(RECOGNITION)),
            FailureNotice::PartOfDictationLost(RECOGNITION)
        );
    }

    #[test]
    fn should_rank_a_lost_dictation_over_a_failed_paste_over_a_lost_part() {
        let gravest_first = [
            FailureNotice::DictationLost(RECOGNITION),
            FailureNotice::NotPasted(None),
            FailureNotice::PartOfDictationLost(RECOGNITION),
        ];
        for (at, graver_notice) in gravest_first.iter().enumerate() {
            for lighter in &gravest_first[at + 1..] {
                assert_eq!(
                    graver(Some(graver_notice.clone()), lighter.clone()),
                    *graver_notice
                );
                assert_eq!(
                    graver(Some(lighter.clone()), graver_notice.clone()),
                    *graver_notice
                );
            }
        }
    }

    #[test]
    fn should_name_the_accessibility_permission_for_a_paste_it_refused() {
        let error = anyhow::Error::new(AccessibilityMissing).context("paste failed");
        assert_eq!(
            not_pasted(&error),
            FailureNotice::NotPasted(Some(FailingPart::Accessibility))
        );
    }

    #[test]
    fn should_name_no_part_for_any_other_failed_paste() {
        let error = anyhow::anyhow!("clipboard busy");
        assert_eq!(not_pasted(&error), FailureNotice::NotPasted(None));
    }

    #[test]
    fn should_name_the_accessibility_permission_for_a_key_press_it_refused() {
        let error = anyhow::Error::new(AccessibilityMissing).context("key press failed");
        assert_eq!(
            command_failed(VoiceCommand::PressEnter, &error),
            FailureNotice::CommandFailed(
                VoiceCommand::PressEnter,
                Some(FailingPart::Accessibility)
            )
        );
    }

    #[test]
    fn should_name_no_part_for_any_other_failed_command() {
        let error = anyhow::anyhow!("recorder stuck");
        assert_eq!(
            command_failed(VoiceCommand::CancelDictation, &error),
            FailureNotice::CommandFailed(VoiceCommand::CancelDictation, None)
        );
    }

    #[test]
    fn should_let_a_recording_that_did_not_start_pass() {
        let error = anyhow::anyhow!("microphone busy").context(RecordingNotStarted);
        assert!(past_recording_not_started(Err(error)).is_ok());
    }

    #[test]
    fn should_keep_any_other_failure() {
        let error = anyhow::anyhow!("live chunks failed");
        assert!(past_recording_not_started(Err(error)).is_err());
    }
}
