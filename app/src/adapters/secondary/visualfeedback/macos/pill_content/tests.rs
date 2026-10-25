use super::*;
use crate::core::domain::{FailingPart, FailureNotice};

/// Widths of the longest labels, as a label sized to fit them at the pill's font measures
/// on macOS.
const RECOGNIZING_LABEL: f64 = 86.0;
const CLICK_A_TEXT_FIELD_LABEL: f64 = 112.5;
const ACCESS_LABEL: f64 = 59.0;
const RESTART_LABEL: f64 = 53.0;
/// Any row at all: the pill keeps one width whatever it holds.
const ANY_ROW: f64 = 0.0;

fn look(symbol: &'static str, label: &'static str, tint: IconTint) -> SignLook {
    SignLook { symbol, label, tint }
}

#[test]
fn should_show_an_ear_and_recognizing_for_speech_recognition() {
    assert_eq!(
        state_sign(OverlayState::RecognizingSpeech),
        Some(look("ear", "Recognizing", IconTint::Content))
    );
}

#[test]
fn should_show_a_keyboard_and_click_a_text_field_while_waiting_for_focus() {
    assert_eq!(
        state_sign(OverlayState::WaitingForFocus),
        Some(look("keyboard", "Click a text field", IconTint::Content))
    );
}

#[test]
fn should_show_a_green_check_and_pasted_after_a_paste() {
    assert_eq!(
        PASTED_SIGN,
        look("checkmark", "Pasted", IconTint::PastedGreen)
    );
}

#[test]
fn should_show_no_sign_with_a_label_for_the_recording_or_the_hidden_overlay() {
    assert_eq!(state_sign(OverlayState::Recording), None);
    assert_eq!(state_sign(OverlayState::Hidden), None);
}

#[test]
fn should_name_each_command_without_a_visible_effect_in_english() {
    let expected = [
        (VoiceCommand::PressEnter, "return", "Enter"),
        (VoiceCommand::ClearAllText, "delete.left", "Clear"),
        (VoiceCommand::PasteText, "doc.on.clipboard", "Paste"),
        (VoiceCommand::RestartDictation, "arrow.clockwise", "Restart"),
    ];
    for (command, symbol, label) in expected {
        let sign = look(symbol, label, IconTint::Content);
        assert_eq!(command_sign(command), Some(sign), "{command:?}");
    }
}

#[test]
fn should_give_no_sign_to_commands_the_overlay_already_shows() {
    let shown = [
        VoiceCommand::StartDictation,
        VoiceCommand::StopDictation,
        VoiceCommand::CancelDictation,
    ];
    for command in shown {
        assert_eq!(command_sign(command), None, "{command:?}");
    }
}

#[test]
fn should_sign_a_failure_with_the_triangle_and_the_failing_part_short_name() {
    let expected = [
        (FailingPart::SpeechRecognition, "STT"),
        (FailingPart::CommandSpotting, "VOICE"),
        (FailingPart::Microphone, "MIC"),
        (FailingPart::Accessibility, "ACCESS"),
    ];
    for (part, name) in expected {
        let notice = FailureNotice::DictationLost(part);
        assert_eq!(
            state_sign(OverlayState::FailureNotice(notice)),
            Some(look(FAILURE_SYMBOL, name, IconTint::FailureTriangle)),
            "{part:?}"
        );
    }
}

#[test]
fn should_sign_a_failure_without_a_part_as_failed() {
    let notice = FailureNotice::NotPasted(None);
    assert_eq!(
        state_sign(OverlayState::FailureNotice(notice)),
        Some(look(FAILURE_SYMBOL, "FAILED", IconTint::FailureTriangle))
    );
}

#[test]
fn should_show_a_muted_microphone_and_cmd_when_commands_are_unavailable() {
    assert_eq!(
        COMMANDS_UNAVAILABLE_SIGN,
        look("mic.slash", "CMD", IconTint::Unavailable)
    );
}

#[test]
fn should_tint_the_glass_orange_while_waiting_and_amber_for_a_failure() {
    let notice = FailureNotice::NotPasted(None);
    let expected = [
        (OverlayState::Recording, GlassTint::Clear),
        (OverlayState::RecognizingSpeech, GlassTint::Clear),
        (OverlayState::WaitingForFocus, GlassTint::Waiting),
        (OverlayState::FailureNotice(notice), GlassTint::Failure),
        (OverlayState::Hidden, GlassTint::Clear),
    ];
    for (state, tint) in expected {
        assert_eq!(glass_tint(state), tint, "{state:?}");
    }
}

#[test]
fn should_return_to_the_recording_after_a_sign_shown_during_it() {
    assert_eq!(
        state_after_sign(OverlayState::Recording),
        OverlayState::Recording
    );
}

#[test]
fn should_hide_after_a_sign_shown_outside_a_recording() {
    let others = [
        OverlayState::Hidden,
        OverlayState::RecognizingSpeech,
        OverlayState::WaitingForFocus,
    ];
    for state in others {
        assert_eq!(state_after_sign(state), OverlayState::Hidden, "{state:?}");
    }
}

#[test]
fn should_put_the_waveform_after_the_record_sign() {
    assert_eq!(
        pill_items(PillKind::Recording, false),
        [PillItem::RecordSign, PillItem::Waveform]
    );
}

#[test]
fn should_put_the_muted_microphone_after_a_separator_while_commands_are_unavailable() {
    assert_eq!(
        pill_items(PillKind::Processing, true),
        [
            PillItem::Icon,
            PillItem::Label,
            PillItem::Spinner,
            PillItem::Separator,
            PillItem::UnavailableIcon
        ]
    );
}

#[test]
fn should_show_the_commands_unavailable_sign_alone_with_its_label() {
    assert_eq!(
        pill_items(PillKind::UnavailableAlone, true),
        [PillItem::UnavailableIcon, PillItem::Label]
    );
}

#[test]
fn should_keep_the_lone_commands_unavailable_pill_compact() {
    assert!(
        (pill_width(PillKind::UnavailableAlone, 60.0) - 92.0).abs() < 1e-6,
        "{} is not {}",
        pill_width(PillKind::UnavailableAlone, 60.0),
        92.0
    );
}

#[test]
fn should_add_a_gap_between_neighbouring_items() {
    assert!(
        (row_width(&[26.0, 176.5], item_gap(PillKind::Recording)) - 216.5).abs() < 1e-6,
        "{} is not {}",
        row_width(&[26.0, 176.5], item_gap(PillKind::Recording)),
        216.5
    );
    assert!(
        (row_width(&[24.0, 40.0], item_gap(PillKind::Command)) - 74.0).abs() < 1e-6,
        "{} is not {}",
        row_width(&[24.0, 40.0], item_gap(PillKind::Command)),
        74.0
    );
}

fn row_of(kind: PillKind, item_widths: &[f64]) -> f64 {
    row_width(item_widths, item_gap(kind))
}

fn row_beside_unavailable_sign(kind: PillKind, item_widths: &[f64]) -> f64 {
    let mut widths = item_widths.to_vec();
    widths.extend([SEPARATOR_WIDTH, ICON_BOX]);
    row_of(kind, &widths)
}

/// The widest row of every state the pill shows, with the commands-unavailable sign beside
/// it when it has room for one.
fn widest_rows() -> Vec<(PillKind, f64)> {
    let full_waveform = [RECORD_RING_SIZE, waveform_width(FULL_CAPACITY)];
    let short_waveform = [RECORD_RING_SIZE, waveform_width(CAPACITY_BESIDE_SIGN)];
    let recognizing = [ICON_BOX, RECOGNIZING_LABEL, SPINNER_SIZE];
    let waiting = [ICON_BOX, CLICK_A_TEXT_FIELD_LABEL];
    let failure = [ICON_BOX, ACCESS_LABEL];
    let command = [ICON_BOX, RESTART_LABEL];
    vec![
        (
            PillKind::Recording,
            row_of(PillKind::Recording, &full_waveform),
        ),
        (
            PillKind::Recording,
            row_beside_unavailable_sign(PillKind::Recording, &short_waveform),
        ),
        (
            PillKind::Processing,
            row_beside_unavailable_sign(PillKind::Processing, &recognizing),
        ),
        (
            PillKind::Sign,
            row_beside_unavailable_sign(PillKind::Sign, &waiting),
        ),
        (
            PillKind::Sign,
            row_beside_unavailable_sign(PillKind::Sign, &failure),
        ),
        (
            PillKind::Command,
            row_beside_unavailable_sign(PillKind::Command, &command),
        ),
    ]
}

#[test]
fn should_give_every_pill_but_the_lone_commands_unavailable_sign_one_width() {
    let one_width = pill_width(PillKind::Recording, ANY_ROW);
    let kinds = [
        PillKind::Recording,
        PillKind::Processing,
        PillKind::Sign,
        PillKind::Command,
    ];
    for kind in kinds {
        for (_, row) in widest_rows() {
            assert!(
                (pill_width(kind, row) - one_width).abs() < 1e-6,
                "{kind:?} around {row}"
            );
        }
    }
}

#[test]
fn should_fit_the_widest_content_of_every_state_in_the_one_pill_width() {
    for (kind, row) in widest_rows() {
        let width = pill_width(kind, ANY_ROW);
        let needed = row + 2.0 * pill_padding(kind);
        assert!(
            needed <= width,
            "{kind:?} needs {needed}, the pill is {width}"
        );
    }
}
