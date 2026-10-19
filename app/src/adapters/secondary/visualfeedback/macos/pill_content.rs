//! What the overlay pill shows in each state, as plain data: the SF Symbol, the English label,
//! the colours, and the items left to right. AppKit drawing lives elsewhere.

use crate::core::domain::level_history::{CAPACITY_BESIDE_SIGN, FULL_CAPACITY};
use crate::core::domain::VoiceCommand;

use super::waveform::waveform_width;
use super::OverlayState;

/// The colour an icon of the pill is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IconTint {
    /// The glass's content colour: light over dark glass, dark over milky glass.
    Content,
    /// The content colour dimmed, for the commands-unavailable sign.
    Unavailable,
    PastedGreen,
    /// A dark mark in a yellow triangle, or a dark amber triangle over milky glass.
    FailureTriangle,
}

/// The tint of the glass itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GlassTint {
    Clear,
    /// Orange: the text waits for a text field.
    Waiting,
    /// Amber: a failure notice.
    Failure,
}

/// An icon with its English label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SignLook {
    pub(super) symbol: &'static str,
    pub(super) label: &'static str,
    pub(super) tint: IconTint,
}

pub(super) const FAILURE_SYMBOL: &str = "exclamationmark.triangle.fill";

pub(super) const PASTED_SIGN: SignLook = SignLook {
    symbol: "checkmark",
    label: "Pasted",
    tint: IconTint::PastedGreen,
};

pub(super) const COMMANDS_UNAVAILABLE_SIGN: SignLook = SignLook {
    symbol: "mic.slash",
    label: "CMD",
    tint: IconTint::Unavailable,
};

const RECOGNIZING_SIGN: SignLook = SignLook {
    symbol: "ear",
    label: "Recognizing",
    tint: IconTint::Content,
};

const REFINING_SIGN: SignLook = SignLook {
    symbol: "sparkles",
    label: "Refining text",
    tint: IconTint::Content,
};

const WAITING_SIGN: SignLook = SignLook {
    symbol: "keyboard",
    label: "Click a text field",
    tint: IconTint::Content,
};

/// The icon and label of `state`; `None` for the recording, which shows the record sign and the
/// waveform instead, and for the hidden overlay.
pub(super) fn state_sign(state: OverlayState) -> Option<SignLook> {
    match state {
        OverlayState::RecognizingSpeech => Some(RECOGNIZING_SIGN),
        OverlayState::RefiningText => Some(REFINING_SIGN),
        OverlayState::WaitingForFocus => Some(WAITING_SIGN),
        OverlayState::FailureNotice(notice) => Some(SignLook {
            symbol: FAILURE_SYMBOL,
            label: notice.part_name(),
            tint: IconTint::FailureTriangle,
        }),
        OverlayState::Recording | OverlayState::Hidden => None,
    }
}

/// The sign of a voice command with no visible effect of its own; `None` for a command the
/// overlay already shows (start, stop, cancel).
pub(super) fn command_sign(command: VoiceCommand) -> Option<SignLook> {
    let (symbol, label) = match command {
        VoiceCommand::PressEnter => ("return", "Enter"),
        VoiceCommand::ClearAllText => ("delete.left", "Clear"),
        VoiceCommand::PasteText => ("doc.on.clipboard", "Paste"),
        VoiceCommand::RestartDictation => ("arrow.clockwise", "Restart"),
        VoiceCommand::StartDictation
        | VoiceCommand::StopDictation
        | VoiceCommand::CancelDictation => return None,
    };
    Some(SignLook {
        symbol,
        label,
        tint: IconTint::Content,
    })
}

/// The tint of the glass in `state`.
pub(super) fn glass_tint(state: OverlayState) -> GlassTint {
    match state {
        OverlayState::WaitingForFocus => GlassTint::Waiting,
        OverlayState::FailureNotice(_) => GlassTint::Failure,
        OverlayState::Hidden
        | OverlayState::Recording
        | OverlayState::RecognizingSpeech
        | OverlayState::RefiningText => GlassTint::Clear,
    }
}

/// What the overlay shows once a command sign shown in `state` is over: the recording goes
/// on, anything else was already over when the command ran.
pub(super) fn state_after_sign(state: OverlayState) -> OverlayState {
    match state {
        OverlayState::Recording => OverlayState::Recording,
        _ => OverlayState::Hidden,
    }
}

/// One part of the pill's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PillItem {
    RecordSign,
    Waveform,
    Icon,
    Label,
    Spinner,
    Separator,
    /// The muted microphone of the commands-unavailable sign.
    UnavailableIcon,
}

/// The kind of row the pill draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PillKind {
    /// Record sign and waveform.
    Recording,
    /// Icon, label and spinner.
    Processing,
    /// Icon and label: waiting, Pasted, failure.
    Sign,
    /// Icon and label of a voice command, closer together.
    Command,
    /// The commands-unavailable sign with nothing beside it.
    UnavailableAlone,
}

/// The items of a `kind` row, left to right, with the muted microphone after a separator
/// while `commands_unavailable` and the row has room for it.
pub(super) fn pill_items(kind: PillKind, commands_unavailable: bool) -> Vec<PillItem> {
    let mut items = match kind {
        PillKind::Recording => vec![PillItem::RecordSign, PillItem::Waveform],
        PillKind::Processing => vec![PillItem::Icon, PillItem::Label, PillItem::Spinner],
        PillKind::Sign | PillKind::Command => vec![PillItem::Icon, PillItem::Label],
        PillKind::UnavailableAlone => return vec![PillItem::UnavailableIcon, PillItem::Label],
    };
    if commands_unavailable {
        items.extend([PillItem::Separator, PillItem::UnavailableIcon]);
    }
    items
}

pub(super) const RECORD_RING_SIZE: f64 = 26.0;
pub(super) const ICON_BOX: f64 = 24.0;
pub(super) const SPINNER_SIZE: f64 = 20.0;
pub(super) const SEPARATOR_WIDTH: f64 = 1.0;
const PILL_PADDING: f64 = 22.0;
const COMPACT_PILL_PADDING: f64 = 16.0;
const ITEM_GAP: f64 = 14.0;
const COMMAND_ITEM_GAP: f64 = 10.0;

/// Space between neighbouring items of a `kind` row.
pub(super) fn item_gap(kind: PillKind) -> f64 {
    match kind {
        PillKind::Command => COMMAND_ITEM_GAP,
        _ => ITEM_GAP,
    }
}

/// How far the first item sits from the pill's left end, and the last from its right end.
pub(super) fn pill_padding(kind: PillKind) -> f64 {
    match kind {
        PillKind::UnavailableAlone => COMPACT_PILL_PADDING,
        _ => PILL_PADDING,
    }
}

/// The width of a pill around `row_width` of items: one width for every kind, whatever its
/// row, but the commands-unavailable sign standing alone, which hugs its row.
pub(super) fn pill_width(kind: PillKind, row_width: f64) -> f64 {
    match kind {
        PillKind::UnavailableAlone => row_width + 2.0 * pill_padding(kind),
        _ => one_pill_width(),
    }
}

/// The width of the widest recording, the widest row there is: the record sign and the
/// waveform, full or shortened beside the commands-unavailable sign.
fn one_pill_width() -> f64 {
    let full_waveform = [RECORD_RING_SIZE, waveform_width(FULL_CAPACITY)];
    let beside_sign = [
        RECORD_RING_SIZE,
        waveform_width(CAPACITY_BESIDE_SIGN),
        SEPARATOR_WIDTH,
        ICON_BOX,
    ];
    let widest_row = row_width(&full_waveform, ITEM_GAP).max(row_width(&beside_sign, ITEM_GAP));
    widest_row + 2.0 * PILL_PADDING
}

/// Width of a row of `item_widths` set `gap` apart.
pub(super) fn row_width(item_widths: &[f64], gap: f64) -> f64 {
    let gaps = item_widths.len().saturating_sub(1) as f64 * gap;
    item_widths.iter().sum::<f64>() + gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::{FailingPart, FailureNotice};

    /// Widths of the longest labels, as a label sized to fit them at the pill's font measures
    /// on macOS.
    const RECOGNIZING_LABEL: f64 = 86.0;
    const REFINING_TEXT_LABEL: f64 = 89.0;
    const CLICK_A_TEXT_FIELD_LABEL: f64 = 112.5;
    const ACCESS_LABEL: f64 = 59.0;
    const RESTART_LABEL: f64 = 53.0;
    /// Any row at all: the pill keeps one width whatever it holds.
    const ANY_ROW: f64 = 0.0;

    fn look(symbol: &'static str, label: &'static str, tint: IconTint) -> SignLook {
        SignLook {
            symbol,
            label,
            tint,
        }
    }

    #[test]
    fn should_show_an_ear_and_recognizing_for_speech_recognition() {
        assert_eq!(
            state_sign(OverlayState::RecognizingSpeech),
            Some(look("ear", "Recognizing", IconTint::Content))
        );
    }

    #[test]
    fn should_show_sparkles_and_refining_text_for_text_refinement() {
        assert_eq!(
            state_sign(OverlayState::RefiningText),
            Some(look("sparkles", "Refining text", IconTint::Content))
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
            (
                VoiceCommand::PressEnter,
                look("return", "Enter", IconTint::Content),
            ),
            (
                VoiceCommand::ClearAllText,
                look("delete.left", "Clear", IconTint::Content),
            ),
            (
                VoiceCommand::PasteText,
                look("doc.on.clipboard", "Paste", IconTint::Content),
            ),
            (
                VoiceCommand::RestartDictation,
                look("arrow.clockwise", "Restart", IconTint::Content),
            ),
        ];
        for (command, sign) in expected {
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
            (FailingPart::TextRefinement, "REFINE"),
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
            (OverlayState::RefiningText, GlassTint::Clear),
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
            OverlayState::RefiningText,
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
        assert_eq!(pill_width(PillKind::UnavailableAlone, 60.0), 92.0);
    }

    #[test]
    fn should_add_a_gap_between_neighbouring_items() {
        assert_eq!(
            row_width(&[26.0, 176.5], item_gap(PillKind::Recording)),
            216.5
        );
        assert_eq!(row_width(&[24.0, 40.0], item_gap(PillKind::Command)), 74.0);
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
        let refining = [ICON_BOX, REFINING_TEXT_LABEL, SPINNER_SIZE];
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
                PillKind::Processing,
                row_beside_unavailable_sign(PillKind::Processing, &refining),
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
                assert_eq!(pill_width(kind, row), one_width, "{kind:?} around {row}");
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
}
