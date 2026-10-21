use crate::core::application::DictationOutcome;
use crate::core::domain::{AccessibilityMissing, FailingPart, FailureNotice, VoiceCommand};

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
    Unrefined,
    PartLost,
    NotPasted,
    Lost,
}

fn gravity(notice: FailureNotice) -> Gravity {
    match notice {
        FailureNotice::PastedUnrefined(_) => Gravity::Unrefined,
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
fn graver(pending: Option<FailureNotice>, notice: FailureNotice) -> FailureNotice {
    match pending {
        Some(pending) if gravity(pending) >= gravity(notice) => pending,
        _ => notice,
    }
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
        .any(|cause| cause.is::<AccessibilityMissing>())
        .then_some(FailingPart::Accessibility)
}

impl Pipeline {
    /// Keeps `notice` for the end of the cycle, unless a graver one is already pending.
    pub(super) fn note_failure(&mut self, notice: FailureNotice) {
        log::warn!("failure notice: {}", notice.message());
        self.pending_notice = Some(graver(self.pending_notice, notice));
    }

    /// Ends the cycle on the overlay: the pending failure notice, or hidden without one.
    pub(super) fn end_cycle_on_overlay(&mut self) {
        match self.pending_notice.take() {
            Some(notice) => self.visual_indicator.show_failure_notice(&notice),
            None => self.visual_indicator.hide(),
        }
    }

    /// Ends a dictation whose text was delivered on the overlay: the pending failure notice,
    /// the Pasted sign when the text was pasted, or hidden.
    pub(super) fn end_delivery_on_overlay(&mut self, outcome: DictationOutcome) {
        if outcome == DictationOutcome::Pasted && self.pending_notice.is_none() {
            self.visual_indicator.show_pasted();
        } else {
            self.end_cycle_on_overlay();
        }
    }

    /// Shows `notice` at once: the failure happened outside a dictation.
    pub(super) fn show_failure_at_once(&self, notice: FailureNotice) {
        log::warn!("failure notice: {}", notice.message());
        self.visual_indicator.show_failure_notice(&notice);
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
            graver(
                None,
                FailureNotice::PastedUnrefined(FailingPart::TextRefinement)
            ),
            FailureNotice::PastedUnrefined(FailingPart::TextRefinement)
        );
    }

    #[test]
    fn should_rank_a_lost_dictation_over_a_failed_paste_over_a_lost_part_over_no_refinement() {
        let gravest_first = [
            FailureNotice::DictationLost(RECOGNITION),
            FailureNotice::NotPasted(None),
            FailureNotice::PartOfDictationLost(RECOGNITION),
            FailureNotice::PastedUnrefined(FailingPart::TextRefinement),
        ];
        for (at, graver_notice) in gravest_first.iter().enumerate() {
            for lighter in &gravest_first[at + 1..] {
                assert_eq!(graver(Some(*graver_notice), *lighter), *graver_notice);
                assert_eq!(graver(Some(*lighter), *graver_notice), *graver_notice);
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
