//! What the overlay pill shows in each state, as plain data: the SF Symbol, the English label,
//! the colours, and the items left to right. `AppKit` drawing lives elsewhere.

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

pub(super) const PASTED_SIGN: SignLook =
    SignLook { symbol: "checkmark", label: "Pasted", tint: IconTint::PastedGreen };

pub(super) const COMMANDS_UNAVAILABLE_SIGN: SignLook =
    SignLook { symbol: "mic.slash", label: "CMD", tint: IconTint::Unavailable };

const RECOGNIZING_SIGN: SignLook =
    SignLook { symbol: "ear", label: "Recognizing", tint: IconTint::Content };

const WAITING_SIGN: SignLook =
    SignLook { symbol: "keyboard", label: "Click a text field", tint: IconTint::Content };

/// The icon and label of `state`; `None` for the recording, which shows the record sign and the
/// waveform instead, and for the hidden overlay.
pub(super) fn state_sign(state: &OverlayState) -> Option<SignLook> {
    match state {
        OverlayState::RecognizingSpeech => Some(RECOGNIZING_SIGN),
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
    Some(SignLook { symbol, label, tint: IconTint::Content })
}

/// The tint of the glass in `state`.
pub(super) fn glass_tint(state: &OverlayState) -> GlassTint {
    match state {
        OverlayState::WaitingForFocus => GlassTint::Waiting,
        OverlayState::FailureNotice(_) => GlassTint::Failure,
        OverlayState::Hidden | OverlayState::Recording | OverlayState::RecognizingSpeech => {
            GlassTint::Clear
        }
    }
}

/// What the overlay shows once a command sign shown in `state` is over: the recording goes
/// on, anything else was already over when the command ran.
pub(super) fn state_after_sign(state: &OverlayState) -> OverlayState {
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
mod tests;
