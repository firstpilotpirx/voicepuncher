use super::permission::Permission;
use super::voice_command::VoiceCommand;

/// The part whose failure an action the user started ran into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailingPart {
    SpeechRecognition,
    CommandSpotting,
    Microphone,
    Accessibility,
}

impl FailingPart {
    pub const ALL: [Self; 4] = [
        Self::SpeechRecognition,
        Self::CommandSpotting,
        Self::Microphone,
        Self::Accessibility,
    ];

    /// The short uppercase name the overlay pill shows.
    pub fn short_name(self) -> &'static str {
        match self {
            Self::SpeechRecognition => "STT",
            Self::CommandSpotting => "VOICE",
            Self::Microphone => "MIC",
            Self::Accessibility => "ACCESS",
        }
    }

    fn failure(self) -> &'static str {
        match self {
            Self::SpeechRecognition => "speech recognition failed",
            Self::CommandSpotting => "command spotting failed",
            Self::Microphone => "the microphone failed",
            Self::Accessibility => "Accessibility permission missing",
        }
    }

    fn remedy(self) -> &'static str {
        match self {
            Self::SpeechRecognition | Self::CommandSpotting => "start the local services",
            Self::Microphone => "check the microphone and its permission",
            Self::Accessibility => "check the Accessibility permission",
        }
    }

    fn cause(self) -> String {
        format!("{} — {}", self.failure(), self.remedy())
    }
}

const PILL_LABEL_WITHOUT_PART: &str = "FAILED";

/// What the overlay tells the user when an action failed: what was lost and which part failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureNotice {
    DictationLost(FailingPart),
    PartOfDictationLost(FailingPart),
    NotPasted(Option<FailingPart>),
    RecordingNotStarted,
    CommandFailed(VoiceCommand, Option<FailingPart>),
    VoiceCommandsUnavailable,
    /// The runtime cancelled the dictation because the user revoked the permission.
    PermissionRevoked(Permission),
}

impl FailureNotice {
    pub fn part(&self) -> Option<FailingPart> {
        match *self {
            Self::DictationLost(part) | Self::PartOfDictationLost(part) => Some(part),
            Self::NotPasted(part) | Self::CommandFailed(_, part) => part,
            Self::RecordingNotStarted => Some(FailingPart::Microphone),
            Self::VoiceCommandsUnavailable => Some(FailingPart::CommandSpotting),
            Self::PermissionRevoked(_) => None,
        }
    }

    /// The short sentence the overlay shows: what was lost, what failed and, when known, the remedy.
    pub fn message(&self) -> String {
        let loss: String = self.loss();
        match (self.part(), self) {
            (Some(part), _) => format!("{loss}: {}", part.cause()),
            (None, Self::NotPasted(_)) => format!("{loss}: pasting failed"),
            (None, Self::PermissionRevoked(permission)) => {
                format!("{loss}: {permission} permission was revoked")
            }
            (None, _) => loss,
        }
    }

    /// The short uppercase label the overlay pill shows next to the warning sign.
    pub fn part_name(&self) -> &'static str {
        self.part()
            .map_or(PILL_LABEL_WITHOUT_PART, FailingPart::short_name)
    }

    fn loss(self) -> String {
        let loss: &str = match self {
            Self::DictationLost(_) => "Dictation lost",
            Self::PartOfDictationLost(_) => "Part of the dictation lost",
            Self::NotPasted(_) => "Text not pasted",
            Self::RecordingNotStarted => "Recording not started",
            Self::CommandFailed(command, _) => return format!("\"{}\" failed", command.phrase()),
            Self::VoiceCommandsUnavailable => "Voice commands unavailable",
            Self::PermissionRevoked(_) => "Dictation cancelled",
        };
        loss.to_owned()
    }
}

/// The process is not trusted for accessibility, so macOS would drop the events it posts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the Accessibility permission is missing")]
pub struct AccessibilityMissing;

#[cfg(test)]
mod tests;
