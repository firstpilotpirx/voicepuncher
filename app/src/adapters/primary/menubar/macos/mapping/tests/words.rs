//! The status line and the menu title in words.

use super::*;

#[test]
pub(super) fn should_name_the_hotkey_and_how_to_dictate_when_ready() {
    assert_eq!(
        ready_state().status_line(),
        "Ready — hold Right Option to dictate"
    );
    let toggle = state(None, Some(ready(RecordingMode::Toggle, false)));
    assert_eq!(
        toggle.status_line(),
        "Ready — press Right Option to dictate"
    );
}

#[test]
pub(super) fn should_name_how_to_stop_while_recording() {
    let hold = state(
        Some(DictationStage::Recording),
        Some(ready(RecordingMode::Hold, false)),
    );
    assert_eq!(
        hold.status_line(),
        "Recording — release Right Option to stop"
    );
    let toggle = state(
        Some(DictationStage::Recording),
        Some(ready(RecordingMode::Toggle, false)),
    );
    assert_eq!(
        toggle.status_line(),
        "Recording — press Right Option to stop"
    );
}

#[test]
pub(super) fn should_name_every_other_dictation_stage_in_words() {
    let holding = |stage| state(Some(stage), Some(ready(RecordingMode::Hold, false)));
    let cases = [
        (
            holding(DictationStage::RecognizingSpeech),
            "Recognizing speech…",
        ),
        (
            holding(DictationStage::WaitingForFocus),
            "Waiting for a text field…",
        ),
        (
            holding(DictationStage::FailureNotice),
            "Ready — hold Right Option to dictate",
        ),
    ];
    for (shown, expected) in cases {
        assert_eq!(shown.status_line(), expected, "{shown:?}");
    }
}

#[test]
pub(super) fn should_name_every_other_state_in_words() {
    let failed = Some(RuntimeState::Failed(
        "speech recognition service is down".into(),
    ));
    let mut unavailable = state(None, Some(ready(RecordingMode::Hold, true)));
    unavailable.commands_unavailable = true;
    let cases = [
        (
            state(None, Some(ready(RecordingMode::Hold, true))),
            "Voice control: listening",
        ),
        (unavailable, "Voice control: commands unavailable"),
        (StatusItemState::default(), "Starting…"),
        (state(None, Some(RuntimeState::Starting)), "Starting…"),
        (state(None, Some(RuntimeState::Restarting)), "Restarting…"),
        (state(None, Some(RuntimeState::Stopped)), "Quitting…"),
        (
            state(None, failed),
            "Failed: speech recognition service is down",
        ),
    ];
    for (shown, expected) in cases {
        assert_eq!(shown.status_line(), expected, "{shown:?}");
    }
}

#[test]
pub(super) fn should_name_every_hotkey_as_a_person_would_say_it() {
    let cases = [
        (HotKey::RightOption, "Right Option"),
        (HotKey::RightControl, "Right Control"),
        (HotKey::RightShift, "Right Shift"),
        (HotKey::RightCommand, "Right Command"),
        (HotKey::LeftOption, "Left Option"),
        (HotKey::LeftControl, "Left Control"),
        (HotKey::Fn, "Fn"),
        (HotKey::CapsLock, "Caps Lock"),
        (HotKey::Space, "Space"),
        (HotKey::CmdE, "Cmd+E"),
        (HotKey::LeftOptionE, "Option+E"),
        (HotKey::F13, "F13"),
        (HotKey::F14, "F14"),
        (HotKey::F15, "F15"),
        (HotKey::F16, "F16"),
        (HotKey::F17, "F17"),
        (HotKey::F18, "F18"),
        (HotKey::F19, "F19"),
    ];
    for (key, name) in cases {
        assert_eq!(hotkey_name(key), name);
    }
}

#[test]
pub(super) fn should_title_the_menu_with_the_name_and_version() {
    assert_eq!(
        title_line(),
        format!("voicepuncher {}", env!("CARGO_PKG_VERSION"))
    );
}
