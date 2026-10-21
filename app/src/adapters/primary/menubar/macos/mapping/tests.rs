use super::*;
use crate::core::domain::Permission;

fn ready(mode: RecordingMode, voice_control: bool) -> Option<RuntimeState> {
    Some(RuntimeState::Ready {
        hotkey: HotKey::RightOption,
        recording_mode: mode,
        voice_control,
    })
}

fn state(dictation: Option<DictationStage>, runtime: Option<RuntimeState>) -> StatusItemState {
    StatusItemState {
        dictation,
        runtime,
        commands_unavailable: false,
    }
}

fn ready_state() -> StatusItemState {
    state(None, ready(RecordingMode::Hold, false))
}

#[test]
fn should_show_waiting_before_the_runtime_reports() {
    assert_eq!(StatusItemState::default().icon(), IconState::Waiting);
}

#[test]
fn should_show_each_runtime_state_while_no_dictation_runs() {
    let cases = [
        (Some(RuntimeState::Starting), IconState::Waiting),
        (Some(RuntimeState::Restarting), IconState::Waiting),
        (Some(RuntimeState::Stopped), IconState::Waiting),
        (Some(RuntimeState::Failed("x".into())), IconState::Failed),
        (ready(RecordingMode::Hold, false), IconState::Ready),
        (ready(RecordingMode::Hold, true), IconState::Listening),
    ];
    for (runtime, expected) in cases {
        assert_eq!(state(None, runtime.clone()).icon(), expected, "{runtime:?}");
    }
}

#[test]
fn should_show_each_dictation_stage() {
    let cases = [
        (DictationStage::Recording, IconState::Recording),
        (DictationStage::RecognizingSpeech, IconState::Processing),
        (DictationStage::RefiningText, IconState::Processing),
        (DictationStage::WaitingForFocus, IconState::WaitingForFocus),
        (DictationStage::FailureNotice, IconState::Warning),
    ];
    for (stage, expected) in cases {
        let shown = state(Some(stage), ready(RecordingMode::Hold, true)).icon();
        assert_eq!(shown, expected, "{stage:?}");
    }
}

#[test]
fn should_show_commands_unavailable_over_listening_in_standby() {
    // Given
    let mut shown = state(None, ready(RecordingMode::Hold, true));
    shown.commands_unavailable = true;
    // When / Then
    assert_eq!(shown.icon(), IconState::CommandsUnavailable);
}

#[test]
fn should_let_a_dictation_outrank_commands_unavailable() {
    // Given
    let mut shown = state(
        Some(DictationStage::Recording),
        ready(RecordingMode::Hold, true),
    );
    shown.commands_unavailable = true;
    // When / Then
    assert_eq!(shown.icon(), IconState::Recording);
}

#[test]
fn should_draw_every_icon_state_with_its_symbol() {
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
        let expected = IconPicture {
            symbol,
            template,
            red_dot,
            animated,
        };
        assert_eq!(icon.picture(), expected, "{icon:?}");
    }
}

#[test]
fn should_name_the_hotkey_and_how_to_dictate_when_ready() {
    assert_eq!(
        ready_state().status_line(),
        "Ready — hold Right Option to dictate"
    );
    let toggle = state(None, ready(RecordingMode::Toggle, false));
    assert_eq!(
        toggle.status_line(),
        "Ready — press Right Option to dictate"
    );
}

#[test]
fn should_name_how_to_stop_while_recording() {
    let hold = state(
        Some(DictationStage::Recording),
        ready(RecordingMode::Hold, false),
    );
    assert_eq!(
        hold.status_line(),
        "Recording — release Right Option to stop"
    );
    let toggle = state(
        Some(DictationStage::Recording),
        ready(RecordingMode::Toggle, false),
    );
    assert_eq!(
        toggle.status_line(),
        "Recording — press Right Option to stop"
    );
}

#[test]
fn should_name_every_other_state_in_words() {
    let failed = Some(RuntimeState::Failed(
        "speech recognition service is down".into(),
    ));
    let mut unavailable = state(None, ready(RecordingMode::Hold, true));
    unavailable.commands_unavailable = true;
    let cases = [
        (
            state(
                Some(DictationStage::RecognizingSpeech),
                ready(RecordingMode::Hold, false),
            ),
            "Recognizing speech…",
        ),
        (
            state(
                Some(DictationStage::RefiningText),
                ready(RecordingMode::Hold, false),
            ),
            "Refining text…",
        ),
        (
            state(
                Some(DictationStage::WaitingForFocus),
                ready(RecordingMode::Hold, false),
            ),
            "Waiting for a text field…",
        ),
        (
            state(
                Some(DictationStage::FailureNotice),
                ready(RecordingMode::Hold, false),
            ),
            "Ready — hold Right Option to dictate",
        ),
        (
            state(None, ready(RecordingMode::Hold, true)),
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
fn should_name_every_hotkey_as_a_person_would_say_it() {
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
fn should_title_the_menu_with_the_name_and_version() {
    assert_eq!(
        title_line(),
        format!("voicepuncher {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn should_follow_each_overlay_signal() {
    let cases = [
        (OverlaySignal::Recording, Some(DictationStage::Recording)),
        (
            OverlaySignal::RecognizingSpeech,
            Some(DictationStage::RecognizingSpeech),
        ),
        (
            OverlaySignal::RefiningText,
            Some(DictationStage::RefiningText),
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
            Some(DictationStage::RefiningText),
            ready(RecordingMode::Hold, false),
        );
        // When
        shown.apply(signal);
        // Then
        assert_eq!(shown.dictation, expected, "{signal:?}");
    }
}

#[test]
fn should_change_nothing_on_a_command_sign() {
    // Given
    let mut shown = state(
        Some(DictationStage::Recording),
        ready(RecordingMode::Hold, true),
    );
    let before = shown.clone();
    // When
    shown.apply(OverlaySignal::CommandSign);
    // Then
    assert_eq!(shown, before);
}

#[test]
fn should_track_whether_commands_are_unavailable() {
    // Given
    let mut shown = state(None, ready(RecordingMode::Hold, true));
    // When
    shown.apply(OverlaySignal::CommandsUnavailable);
    // Then
    assert!(shown.commands_unavailable);
    shown.apply(OverlaySignal::CommandsAvailable);
    assert!(!shown.commands_unavailable);
}

#[test]
fn should_drop_the_warning_when_the_failure_notice_expires() {
    // Given
    let mut shown = state(
        Some(DictationStage::FailureNotice),
        ready(RecordingMode::Hold, false),
    );
    // When
    shown.apply(OverlaySignal::FailureNoticeExpired);
    // Then
    assert_eq!(shown.icon(), IconState::Ready);
}

#[test]
fn should_keep_a_new_dictation_when_an_old_failure_notice_expires() {
    // Given
    let mut shown = state(
        Some(DictationStage::Recording),
        ready(RecordingMode::Hold, false),
    );
    // When
    shown.apply(OverlaySignal::FailureNoticeExpired);
    // Then
    assert_eq!(shown.icon(), IconState::Recording);
}

#[test]
fn should_forget_commands_unavailable_when_the_runtime_leaves_ready() {
    // Given
    let mut shown = state(None, ready(RecordingMode::Hold, true));
    shown.commands_unavailable = true;
    // When
    shown.runtime_changed(RuntimeState::Restarting);
    // Then
    assert!(!shown.commands_unavailable);
    assert_eq!(shown.runtime, Some(RuntimeState::Restarting));
}

#[test]
fn should_keep_commands_unavailable_while_the_runtime_stays_ready() {
    // Given
    let mut shown = state(None, ready(RecordingMode::Hold, true));
    shown.commands_unavailable = true;
    // When
    shown.runtime_changed(ready(RecordingMode::Hold, true).unwrap());
    // Then
    assert!(shown.commands_unavailable);
}

fn waiting_for_accessibility() -> Option<RuntimeState> {
    Some(RuntimeState::WaitingForPermissions {
        missing: vec![Permission::Accessibility],
        revoked: vec![],
    })
}

#[test]
fn should_show_the_waiting_icon_while_waiting_for_permissions() {
    assert_eq!(
        state(None, waiting_for_accessibility()).icon(),
        IconState::Waiting
    );
}

#[test]
fn should_point_at_the_permissions_window_while_waiting_for_permissions() {
    assert_eq!(
        state(None, waiting_for_accessibility()).status_line(),
        "Waiting for permissions — open Permissions…"
    );
}

#[test]
fn should_offer_permissions_right_under_the_status_line() {
    let status = MENU
        .iter()
        .position(|entry| *entry == MenuEntry::StatusLine)
        .expect("the menu has a status line");
    assert_eq!(MENU.get(status + 1), Some(&MenuEntry::Permissions));
    assert_eq!(MenuEntry::Permissions.title(), Some("Permissions…"));
}

#[test]
fn should_count_the_services_ready_while_they_start() {
    // Given
    let starting = state(
        None,
        Some(RuntimeState::StartingServices { ready: 2, total: 5 }),
    );
    // When / Then
    assert_eq!(starting.icon(), IconState::Waiting);
    assert_eq!(starting.status_line(), "Starting services… (2 of 5 ready)");
}

#[test]
fn should_show_a_service_that_cannot_start_as_failed() {
    let failed = state(
        None,
        Some(RuntimeState::Failed(
            "text refinement service: model missing".into(),
        )),
    );
    assert_eq!(failed.icon(), IconState::Failed);
    assert_eq!(
        failed.status_line(),
        "Failed: text refinement service: model missing"
    );
}

fn service(label: &str, state: crate::core::domain::ServiceState, owned: bool) -> ServiceStatus {
    ServiceStatus {
        kind: crate::core::domain::ServiceKind::Dictation,
        label: label.into(),
        port: 18082,
        state,
        owned,
        log_file: std::path::PathBuf::from("/logs/whisper-server-18082.log"),
    }
}

#[test]
fn should_list_a_service_with_its_state_in_words_and_restart_only_owned_ones() {
    use crate::core::domain::ServiceState;
    // Given
    let copy = service(
        "speech recognition service for dictation: copy 1 of 3",
        ServiceState::Ready,
        true,
    );
    let adopted = service("text refinement service", ServiceState::Adopted, false);
    // When / Then
    assert_eq!(
        service_line(&copy),
        "Speech recognition service for dictation: copy 1 of 3 — Ready"
    );
    assert_eq!(service_line(&adopted), "Text refinement service — Adopted");
    assert!(offers_restart(&copy));
    assert!(!offers_restart(&adopted));
}
