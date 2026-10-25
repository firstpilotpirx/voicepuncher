//! The icon of each runtime state, dictation stage and sign.

use super::*;

#[test]
pub(super) fn should_show_waiting_before_the_runtime_reports() {
    assert_eq!(StatusItemState::default().icon(), IconState::Waiting);
}

#[test]
pub(super) fn should_show_each_runtime_state_while_no_dictation_runs() {
    let cases = [
        (Some(RuntimeState::Starting), IconState::Waiting),
        (Some(RuntimeState::Restarting), IconState::Waiting),
        (Some(RuntimeState::Stopped), IconState::Waiting),
        (Some(RuntimeState::Failed("x".into())), IconState::Failed),
        (Some(ready(RecordingMode::Hold, false)), IconState::Ready),
        (Some(ready(RecordingMode::Hold, true)), IconState::Listening),
    ];
    for (runtime, expected) in cases {
        assert_eq!(state(None, runtime.clone()).icon(), expected, "{runtime:?}");
    }
}

#[test]
pub(super) fn should_show_each_dictation_stage() {
    let cases = [
        (DictationStage::Recording, IconState::Recording),
        (DictationStage::RecognizingSpeech, IconState::Processing),
        (DictationStage::WaitingForFocus, IconState::WaitingForFocus),
        (DictationStage::FailureNotice, IconState::Warning),
    ];
    for (stage, expected) in cases {
        let shown = state(Some(stage), Some(ready(RecordingMode::Hold, true))).icon();
        assert_eq!(shown, expected, "{stage:?}");
    }
}

#[test]
pub(super) fn should_show_commands_unavailable_over_listening_in_standby() {
    // Given
    let mut shown = state(None, Some(ready(RecordingMode::Hold, true)));
    shown.commands_unavailable = true;
    // When / Then
    assert_eq!(shown.icon(), IconState::CommandsUnavailable);
}

#[test]
pub(super) fn should_let_a_dictation_outrank_commands_unavailable() {
    // Given
    let mut shown = state(
        Some(DictationStage::Recording),
        Some(ready(RecordingMode::Hold, true)),
    );
    shown.commands_unavailable = true;
    // When / Then
    assert_eq!(shown.icon(), IconState::Recording);
}

#[test]
pub(super) fn should_draw_every_icon_state_with_its_symbol() {
    let cases = [
        (IconState::Ready, "mic", true, false, false),
        (IconState::Recording, "mic.fill", false, true, false),
        (IconState::Processing, "waveform", true, false, true),
        (IconState::WaitingForFocus, "keyboard", true, false, false),
        (IconState::Listening, "ear", true, false, false),
        (
            IconState::CommandsUnavailable,
            "ear.trianglebadge.exclamationmark",
            true,
            false,
            false,
        ),
        (
            IconState::Warning,
            "exclamationmark.triangle",
            true,
            false,
            false,
        ),
        (IconState::Waiting, "hourglass", true, false, false),
        (
            IconState::Failed,
            "exclamationmark.triangle.fill",
            true,
            false,
            false,
        ),
    ];
    for (icon, symbol, template, red_dot, animated) in cases {
        let expected = IconPicture { symbol, template, red_dot, animated };
        assert_eq!(icon.picture(), expected, "{icon:?}");
    }
}
