//! What the status item shows, as pure functions of what it was told: the stage of the
//! dictation the overlay shows, the runtime state and the voice control signs.

use crate::adapters::primary::settings::hints::joined_names;
use crate::core::domain::command_words::capitalized;
use crate::core::domain::{HotKey, RecordingMode, ServiceStatus};
use crate::runtime::RuntimeState;

/// The stage of a dictation, as the overlay shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationStage {
    Recording,
    RecognizingSpeech,
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
    /// The runtime waits for a permission the user has to grant.
    MissingPermission,
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
            OverlaySignal::FailureNoticeExpired => {
                if self.dictation == Some(DictationStage::FailureNotice) {
                    self.dictation = None;
                }
            }
            OverlaySignal::CommandSign => {}
            OverlaySignal::CommandsUnavailable => self.commands_unavailable = true,
            OverlaySignal::CommandsAvailable => self.commands_unavailable = false,
            shown => self.dictation = shown_stage(shown),
        }
    }

    /// A dictation outranks everything; in standby the runtime state, then voice control.
    pub fn icon(&self) -> IconState {
        if let Some(stage) = self.dictation {
            return stage_icon(stage);
        }
        match &self.runtime {
            Some(RuntimeState::Failed(_)) => IconState::Failed,
            Some(RuntimeState::WaitingForPermissions { .. }) => IconState::MissingPermission,
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

    /// Whether choosing the status line opens the Permissions window: while the runtime
    /// waits for permissions and no dictation shows.
    pub fn status_opens_permissions(&self) -> bool {
        self.dictation.is_none()
            && matches!(
                self.runtime,
                Some(RuntimeState::WaitingForPermissions { .. })
            )
    }

    /// The state in words, for the menu's second line.
    pub fn status_line(&self) -> String {
        match self.dictation {
            Some(DictationStage::Recording) => self.recording_line(),
            Some(DictationStage::RecognizingSpeech) => "Recognizing speech…".into(),
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
            Some(RuntimeState::WaitingForPermissions { missing, .. }) => {
                format!("Waiting for {}…", joined_names(missing))
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
                self.ready_line(*hotkey, *recording_mode, *voice_control)
            }
        }
    }

    fn ready_line(&self, hotkey: HotKey, mode: RecordingMode, voice_control: bool) -> String {
        if self.commands_unavailable {
            "Voice control: commands unavailable".into()
        } else if voice_control {
            "Voice control: listening".into()
        } else {
            let verb = match mode {
                RecordingMode::Hold => "hold",
                RecordingMode::Toggle => "press",
            };
            format!("Ready — {verb} {} to dictate", hotkey_name(hotkey))
        }
    }
}

/// The dictation stage a signal shows; none for a signal that ends the dictation.
fn shown_stage(signal: OverlaySignal) -> Option<DictationStage> {
    match signal {
        OverlaySignal::Recording => Some(DictationStage::Recording),
        OverlaySignal::RecognizingSpeech => Some(DictationStage::RecognizingSpeech),
        OverlaySignal::WaitingForFocus => Some(DictationStage::WaitingForFocus),
        OverlaySignal::FailureNotice => Some(DictationStage::FailureNotice),
        _ => None,
    }
}

fn stage_icon(stage: DictationStage) -> IconState {
    match stage {
        DictationStage::Recording => IconState::Recording,
        DictationStage::RecognizingSpeech => IconState::Processing,
        DictationStage::WaitingForFocus => IconState::WaitingForFocus,
        DictationStage::FailureNotice => IconState::Warning,
    }
}

impl IconState {
    pub fn picture(self) -> IconPicture {
        let symbol = SYMBOLS[self as usize];
        let recording = self == Self::Recording;
        IconPicture {
            symbol,
            template: !recording,
            red_dot: recording,
            animated: self == Self::Processing,
        }
    }
}

/// The SF Symbol of each icon state, in the order `IconState` declares them.
const SYMBOLS: [&str; 10] = [
    "mic",
    "mic.fill",
    "waveform",
    "keyboard",
    "ear",
    "ear.trianglebadge.exclamationmark",
    "exclamationmark.triangle",
    "hourglass",
    "lock.shield",
    "exclamationmark.triangle.fill",
];

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
    /// The entry's index in [`MENU`], where the status item finds its menu item; a separator
    /// gives its first place.
    pub fn place(self) -> isize {
        MENU.iter()
            .position(|entry| *entry == self)
            .and_then(|index| isize::try_from(index).ok())
            .unwrap_or_default()
    }

    /// The words of a chosen item; none for the lines and separators.
    pub fn title(self) -> Option<&'static str> {
        match self {
            Self::Permissions => Some("Permissions…"),
            Self::DownloadModels => Some("Download models…"),
            Self::Services => Some("Services"),
            Self::Settings => Some("Settings…"),
            Self::Logs => Some("Open Logs Folder"),
            Self::Quit => Some("Quit voicepuncher"),
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

/// The menu's first line: the app's name and version.
pub fn title_line() -> String {
    format!("voicepuncher {}", env!("CARGO_PKG_VERSION"))
}

/// The key as a person would say it.
pub fn hotkey_name(key: HotKey) -> &'static str {
    key.human_name()
}

#[cfg(test)]
mod permission_wait_tests;
#[cfg(test)]
mod place_tests;
#[cfg(test)]
mod tests;
