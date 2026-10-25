//! How the menu follows the overlay signals and the failure notices.

use super::*;

#[test]
pub(super) fn should_follow_each_overlay_signal() {
    let cases = [
        (OverlaySignal::Recording, Some(DictationStage::Recording)),
        (
            OverlaySignal::RecognizingSpeech,
            Some(DictationStage::RecognizingSpeech),
        ),
        (
            OverlaySignal::WaitingForFocus,
            Some(DictationStage::WaitingForFocus),
        ),
        (
            OverlaySignal::FailureNotice,
            Some(DictationStage::FailureNotice),
        ),
        (OverlaySignal::Pasted, None),
        (OverlaySignal::Hidden, None),
    ];
    for (signal, expected) in cases {
        // Given
        let mut shown = state(
            Some(DictationStage::RecognizingSpeech),
            Some(ready(RecordingMode::Hold, false)),
        );
        // When
        shown.apply(signal);
        // Then
        assert_eq!(shown.dictation, expected, "{signal:?}");
    }
}

#[test]
pub(super) fn should_change_nothing_on_a_command_sign() {
    // Given
    let mut shown = state(
        Some(DictationStage::Recording),
        Some(ready(RecordingMode::Hold, true)),
    );
    let before = shown.clone();
    // When
    shown.apply(OverlaySignal::CommandSign);
    // Then
    assert_eq!(shown, before);
}

#[test]
pub(super) fn should_track_whether_commands_are_unavailable() {
    // Given
    let mut shown = state(None, Some(ready(RecordingMode::Hold, true)));
    // When
    shown.apply(OverlaySignal::CommandsUnavailable);
    // Then
    assert!(shown.commands_unavailable);
    shown.apply(OverlaySignal::CommandsAvailable);
    assert!(!shown.commands_unavailable);
}

#[test]
pub(super) fn should_drop_the_warning_when_the_failure_notice_expires() {
    // Given
    let mut shown = state(
        Some(DictationStage::FailureNotice),
        Some(ready(RecordingMode::Hold, false)),
    );
    // When
    shown.apply(OverlaySignal::FailureNoticeExpired);
    // Then
    assert_eq!(shown.icon(), IconState::Ready);
}

#[test]
pub(super) fn should_keep_a_new_dictation_when_an_old_failure_notice_expires() {
    // Given
    let mut shown = state(
        Some(DictationStage::Recording),
        Some(ready(RecordingMode::Hold, false)),
    );
    // When
    shown.apply(OverlaySignal::FailureNoticeExpired);
    // Then
    assert_eq!(shown.icon(), IconState::Recording);
}

#[test]
pub(super) fn should_forget_commands_unavailable_when_the_runtime_leaves_ready() {
    // Given
    let mut shown = state(None, Some(ready(RecordingMode::Hold, true)));
    shown.commands_unavailable = true;
    // When
    shown.runtime_changed(RuntimeState::Restarting);
    // Then
    assert!(!shown.commands_unavailable);
    assert_eq!(shown.runtime, Some(RuntimeState::Restarting));
}

#[test]
pub(super) fn should_keep_commands_unavailable_while_the_runtime_stays_ready() {
    // Given
    let mut shown = state(None, Some(ready(RecordingMode::Hold, true)));
    shown.commands_unavailable = true;
    // When
    shown.runtime_changed(ready(RecordingMode::Hold, true));
    // Then
    assert!(shown.commands_unavailable);
}
