use super::permission::Permission;
use super::voice_command::VoiceCommand;

/// A program that is no speech recognition service but holds the service's own port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignProgram {
    pub port: u16,
    /// The program as the user can find it, e.g. "app (pid 28344)".
    pub program: String,
}

/// No copy of speech recognition answers, and another program holds the service's own port.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("port {} is held by {}, not by speech recognition", .0.port, .0.program)]
pub struct ServicePortTaken(pub ForeignProgram);

/// The part whose failure an action the user started ran into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailingPart {
    SpeechRecognition,
    CommandSpotting,
    Microphone,
    Accessibility,
    /// Speech recognition, whose own port another program holds while no copy answers.
    ServicePortTaken(ForeignProgram),
}

impl FailingPart {
    pub const ALL: [Self; 4] = [
        Self::SpeechRecognition,
        Self::CommandSpotting,
        Self::Microphone,
        Self::Accessibility,
    ];

    /// The short uppercase name the overlay pill shows.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::SpeechRecognition | Self::ServicePortTaken(_) => "STT",
            Self::CommandSpotting => "VOICE",
            Self::Microphone => "MIC",
            Self::Accessibility => "ACCESS",
        }
    }

    fn failure(&self) -> &'static str {
        match self {
            Self::SpeechRecognition => "speech recognition failed",
            Self::ServicePortTaken(_) => "its port is held by another program",
            Self::CommandSpotting => "command spotting failed",
            Self::Microphone => "the microphone failed",
            Self::Accessibility => "Accessibility permission missing",
        }
    }

    fn remedy(&self) -> &'static str {
        match self {
            Self::SpeechRecognition | Self::CommandSpotting => "start the local services",
            Self::ServicePortTaken(_) => "quit it",
            Self::Microphone => "check the microphone and its permission",
            Self::Accessibility => "check the Accessibility permission",
        }
    }

    fn cause(&self) -> String {
        if let Self::ServicePortTaken(foreign) = self {
            return format!(
                "port {} is held by {} — {}",
                foreign.port,
                foreign.program,
                self.remedy()
            );
        }
        format!("{} — {}", self.failure(), self.remedy())
    }
}

const PILL_LABEL_WITHOUT_PART: &str = "FAILED";

/// What the overlay tells the user when an action failed: what was lost and which part failed.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        match self {
            Self::DictationLost(part) | Self::PartOfDictationLost(part) => Some(part.clone()),
            Self::NotPasted(part) | Self::CommandFailed(_, part) => part.clone(),
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
            .as_ref()
            .map_or(PILL_LABEL_WITHOUT_PART, FailingPart::short_name)
    }

    fn loss(&self) -> String {
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
