//! What the status item shows, as pure functions of what it was told: the stage of the
//! dictation the overlay shows, the runtime state and the voice control signs.

use crate::core::domain::{HotKey, RecordingMode, ServiceStatus};
use crate::runtime::RuntimeState;

/// The stage of a dictation, as the overlay shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationStage {
    Recording,
    RecognizingSpeech,
    RefiningText,
    WaitingForFocus,
    FailureNotice,
}

/// Everything the status item was told; the icon and the status line follow from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusItemState {
    /// The dictation in progress, or the failure notice it ended with; none in standby.
    pub dictation: Option<DictationStage>,
    /// None until the runtime reports its first state.
    pub runtime: Option<RuntimeState>,
    pub commands_unavailable: bool,
}

/// What the overlay was told to show, as the status item hears it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySignal {
    Recording,
    RecognizingSpeech,
    RefiningText,
    WaitingForFocus,
    Pasted,
    Hidden,
    FailureNotice,
    /// The failure notice's lifetime ended.
    FailureNoticeExpired,
    CommandSign,
    CommandsUnavailable,
    CommandsAvailable,
}

/// The picture the status item shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Ready,
    Recording,
    Processing,
    WaitingForFocus,
    Listening,
    CommandsUnavailable,
    Warning,
    Waiting,
    Failed,
}

/// How an icon state is drawn: an SF Symbol, a template image unless it carries colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconPicture {
    pub symbol: &'static str,
    /// Drawn by the system in the menu bar's ink, light or dark.
    pub template: bool,
    /// A red dot at the lower right of the symbol.
    pub red_dot: bool,
    /// The symbol's variable value steps through the processing frames.
    pub animated: bool,
}

/// The variable values of the waveform the processing icon steps through.
pub const PROCESSING_FRAMES: [f64; 3] = [0.33, 0.66, 1.0];
/// The waveform's variable value while Reduce Motion is on.
pub const STILL_PROCESSING_FRAME: f64 = 0.66;
/// Drawn when the system has no symbol of the wanted name.
pub const FALLBACK_SYMBOL: &str = "mic";

impl StatusItemState {
    /// Follows what the overlay was told; a command sign leaves the icon as it is.
    pub fn apply(&mut self, signal: OverlaySignal) {
        match signal {
            OverlaySignal::Recording => self.dictation = Some(DictationStage::Recording),
            OverlaySignal::RecognizingSpeech => {
                self.dictation = Some(DictationStage::RecognizingSpeech)
            }
            OverlaySignal::RefiningText => self.dictation = Some(DictationStage::RefiningText),
            OverlaySignal::WaitingForFocus => {
                self.dictation = Some(DictationStage::WaitingForFocus)
            }
            OverlaySignal::FailureNotice => self.dictation = Some(DictationStage::FailureNotice),
            OverlaySignal::Pasted | OverlaySignal::Hidden => self.dictation = None,
            OverlaySignal::FailureNoticeExpired => {
                if self.dictation == Some(DictationStage::FailureNotice) {
                    self.dictation = None;
                }
            }
            OverlaySignal::CommandSign => {}
            OverlaySignal::CommandsUnavailable => self.commands_unavailable = true,
            OverlaySignal::CommandsAvailable => self.commands_unavailable = false,
        }
    }

    /// A dictation outranks everything; in standby the runtime state, then voice control.
    pub fn icon(&self) -> IconState {
        if let Some(stage) = self.dictation {
            return stage_icon(stage);
        }
        match &self.runtime {
            Some(RuntimeState::Failed(_)) => IconState::Failed,
            Some(RuntimeState::Ready { voice_control, .. }) => {
                if self.commands_unavailable {
                    IconState::CommandsUnavailable
                } else if *voice_control {
                    IconState::Listening
                } else {
                    IconState::Ready
                }
            }
            _ => IconState::Waiting,
        }
    }

    /// Takes the runtime's new state; a pipeline that ends takes its signs with it.
    pub fn runtime_changed(&mut self, state: RuntimeState) {
        if !matches!(state, RuntimeState::Ready { .. }) {
            self.commands_unavailable = false;
        }
        self.runtime = Some(state);
    }

    /// The state in words, for the menu's second line.
    pub fn status_line(&self) -> String {
        match self.dictation {
            Some(DictationStage::Recording) => self.recording_line(),
            Some(DictationStage::RecognizingSpeech) => "Recognizing speech…".into(),
            Some(DictationStage::RefiningText) => "Refining text…".into(),
            Some(DictationStage::WaitingForFocus) => "Waiting for a text field…".into(),
            Some(DictationStage::FailureNotice) | None => self.standby_line(),
        }
    }

    fn recording_line(&self) -> String {
        match &self.runtime {
            Some(RuntimeState::Ready { hotkey, recording_mode, .. }) => {
                let verb = match recording_mode {
                    RecordingMode::Hold => "release",
                    RecordingMode::Toggle => "press",
                };
                format!("Recording — {verb} {} to stop", hotkey_name(*hotkey))
            }
            _ => "Recording…".into(),
        }
    }

    fn standby_line(&self) -> String {
        match &self.runtime {
            None | Some(RuntimeState::Starting) => "Starting…".into(),
            Some(RuntimeState::WaitingForPermissions { .. }) => {
                "Waiting for permissions — open Permissions…".into()
            }
            Some(RuntimeState::WaitingForModels { .. }) => {
                "Waiting for models — open Download models…".into()
            }
            Some(RuntimeState::Restarting) => "Restarting…".into(),
            Some(RuntimeState::StartingServices { ready, total }) => {
                format!("Starting services… ({ready} of {total} ready)")
            }
            Some(RuntimeState::Stopped) => "Quitting…".into(),
            Some(RuntimeState::Failed(reason)) => format!("Failed: {reason}"),
            Some(RuntimeState::Ready { hotkey, recording_mode, voice_control }) => {
                if self.commands_unavailable {
                    "Voice control: commands unavailable".into()
                } else if *voice_control {
                    "Voice control: listening".into()
                } else {
                    let verb = match recording_mode {
                        RecordingMode::Hold => "hold",
                        RecordingMode::Toggle => "press",
                    };
                    format!("Ready — {verb} {} to dictate", hotkey_name(*hotkey))
                }
            }
        }
    }
}

fn stage_icon(stage: DictationStage) -> IconState {
    match stage {
        DictationStage::Recording => IconState::Recording,
        DictationStage::RecognizingSpeech | DictationStage::RefiningText => IconState::Processing,
        DictationStage::WaitingForFocus => IconState::WaitingForFocus,
        DictationStage::FailureNotice => IconState::Warning,
    }
}

impl IconState {
    pub fn picture(self) -> IconPicture {
        let symbol = match self {
            IconState::Ready => "mic",
            IconState::Recording => "mic.fill",
            IconState::Processing => "waveform",
            IconState::WaitingForFocus => "keyboard",
            IconState::Listening => "ear",
            IconState::CommandsUnavailable => "ear.trianglebadge.exclamationmark",
            IconState::Warning => "exclamationmark.triangle",
            IconState::Waiting => "hourglass",
            IconState::Failed => "exclamationmark.triangle.fill",
        };
        let recording = self == IconState::Recording;
        IconPicture {
            symbol,
            template: !recording,
            red_dot: recording,
            animated: self == IconState::Processing,
        }
    }
}

/// An entry of the menu, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEntry {
    TitleLine,
    StatusLine,
    Permissions,
    /// Opens the Download window; hidden when the app leaves the services to the scripts.
    DownloadModels,
    /// The submenu of the local services; hidden when the app leaves them to the scripts.
    Services,
    Separator,
    Settings,
    Logs,
    Quit,
}

/// The menu, the same in every state.
pub const MENU: [MenuEntry; 10] = [
    MenuEntry::TitleLine,
    MenuEntry::StatusLine,
    MenuEntry::Permissions,
    MenuEntry::DownloadModels,
    MenuEntry::Services,
    MenuEntry::Separator,
    MenuEntry::Settings,
    MenuEntry::Logs,
    MenuEntry::Separator,
    MenuEntry::Quit,
];

impl MenuEntry {
    /// The words of a chosen item; none for the lines and separators.
    pub fn title(self) -> Option<&'static str> {
        match self {
            MenuEntry::Permissions => Some("Permissions…"),
            MenuEntry::DownloadModels => Some("Download models…"),
            MenuEntry::Services => Some("Services"),
            MenuEntry::Settings => Some("Settings…"),
            MenuEntry::Logs => Some("Open Logs Folder"),
            MenuEntry::Quit => Some("Quit voicepuncher"),
            _ => None,
        }
    }
}

/// A line of the Services submenu: the service and its state in words.
pub fn service_line(status: &ServiceStatus) -> String {
    format!("{} — {}", capitalized(&status.label), status.state)
}

/// The items of one service's submenu.
pub const RESTART_SERVICE: &str = "Restart";
pub const OPEN_SERVICE_LOG: &str = "Open Log";

/// The Services submenu offers Restart only for a service the app started.
pub fn offers_restart(status: &ServiceStatus) -> bool {
    status.owned
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The menu's first line: the app's name and version.
pub fn title_line() -> String {
    format!("voicepuncher {}", env!("CARGO_PKG_VERSION"))
}

/// The key as a person would say it.
pub fn hotkey_name(key: HotKey) -> &'static str {
    key.human_name()
}

#[cfg(test)]
mod tests;
